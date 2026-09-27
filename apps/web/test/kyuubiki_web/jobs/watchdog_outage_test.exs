defmodule KyuubikiWeb.Jobs.WatchdogOutageTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.{Persistence, Storage}
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage

  setup do
    Outage.setup()
  end

  @tag pre_fix: true
  @tag capture_log: true
  test "an unavailable job store does not terminate the watchdog or report a successful scan" do
    pid = Process.whereis(Watchdog)

    Outage.without(Outage.store_child(), fn ->
      assert %{available: false, reason: "analysis_store_unavailable", stalled: 0, timed_out: 0} =
               Watchdog.scan_now()

      assert Process.whereis(Watchdog) == pid
    end)

    assert %{available: true, stalled: 0, timed_out: 0} = Watchdog.scan_now()
    assert Process.whereis(Watchdog) == pid
  end

  @tag pre_fix: true
  test "unreadable job counts remain unknown in the health snapshot" do
    Outage.without(Outage.store_child(), fn ->
      snapshot = Watchdog.status_snapshot()
      assert snapshot.available == false
      assert snapshot.reason == "analysis_store_unavailable"
      assert snapshot.watchdog_state == "unknown"
      assert snapshot.active_jobs == nil
      assert snapshot.stalled_jobs == nil
      assert snapshot.timed_out_jobs == nil
      refute Map.has_key?(snapshot, :orchestra_load)
      refute Map.has_key?(snapshot, :operator_load)
    end)

    assert %{available: true, active_jobs: 0} = Watchdog.status_snapshot()
  end

  test "periodic scans survive an outage and resume without restarting the watchdog" do
    now = DateTime.utc_now()

    assert {:ok, _job} =
             Store.create(%{
               job_id: "outage-stale-job",
               project_id: "outage",
               simulation_case_id: "case",
               status: :solving,
               created_at: DateTime.add(now, -20, :second),
               updated_at: DateTime.add(now, -10, :second)
             })

    config = Application.get_env(:kyuubiki_web, Watchdog)
    Application.put_env(:kyuubiki_web, Watchdog, Keyword.put(config, :scan_interval_ms, 20))
    :ok = Outage.stop(Watchdog)

    Outage.without(Outage.store_child(), fn ->
      Outage.start(Watchdog)
      pid = Process.whereis(Watchdog)
      Process.sleep(100)
      assert Process.whereis(Watchdog) == pid
      assert %{available: false} = Watchdog.scan_now()
    end)

    pid = Process.whereis(Watchdog)

    Outage.eventually(fn ->
      match?({:ok, %{status: :failed}}, Store.get("outage-stale-job"))
    end)

    assert Process.whereis(Watchdog) == pid
    assert %{available: true, stalled_jobs: 1} = Watchdog.status_snapshot()
  end

  @tag skip: Storage.postgres?()
  test "a failed watchdog write stops the scan without failing unrelated jobs" do
    now = DateTime.utc_now()

    jobs =
      for {id, age} <- [{"outage-write-first", 10}, {"outage-write-next", 20}] do
        assert {:ok, job} =
                 Store.create(%{
                   job_id: id,
                   project_id: "outage",
                   simulation_case_id: "case",
                   status: :solving,
                   created_at: DateTime.add(now, -30, :second),
                   updated_at: DateTime.add(now, -age, :second)
                 })

        job
      end

    drop =
      if Storage.sqlite?() do
        AnalysisCommitFault.reject_job_update(hd(jobs).job_id)
      else
        path = Persistence.analysis_state_path() <> ".next"
        File.mkdir_p!(path)
        File.write!(Path.join(path, "blocker"), "injected fault")
        cleanup = fn -> File.rm_rf!(path) end
        on_exit(cleanup)
        cleanup
      end

    pid = Process.whereis(Watchdog)
    assert %{available: false, stalled: 0, timed_out: 0} = Watchdog.scan_now()
    assert Process.whereis(Watchdog) == pid
    for job <- jobs, do: assert({:ok, ^job} = Store.get(job.job_id))

    assert %{available: true, watchdog_state: "unknown", last_scan: %{available: false}} =
             Watchdog.status_snapshot()

    drop.()
    assert %{available: true, stalled: 2, timed_out: 0} = Watchdog.scan_now()
    assert %{last_scan: %{available: true}} = Watchdog.status_snapshot()
  end
end
