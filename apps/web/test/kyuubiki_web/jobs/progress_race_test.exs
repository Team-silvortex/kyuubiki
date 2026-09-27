defmodule KyuubikiWeb.Jobs.ProgressRaceTest do
  use ExUnit.Case, async: false
  import Ecto.Query
  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.JobRecord
  alias KyuubikiWeb.TestSupport.JobQueryRace

  @moduletag skip: not Storage.sql?()

  setup do
    Store.reset()
    original = Application.get_env(:kyuubiki_web, Watchdog, [])

    Application.put_env(:kyuubiki_web, Watchdog,
      scan_interval_ms: 60_000,
      stale_job_ms: 5_000,
      job_timeout_ms: 60_000
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, Watchdog, original)
      Store.reset()
    end)

    :ok
  end

  test "progress update reports a missing job when it disappears after its read" do
    create_job("progress-race", DateTime.utc_now())
    race = delete_after_read("progress-race", {:job, "progress-race"})

    assert {:error, {:job_not_found, "progress-race"}} =
             Store.apply_progress(%{
               job_id: "progress-race",
               stage: "solving",
               progress: 0.5
             })

    assert :error = Store.get("progress-race")
    assert_receive {:interleaved, ^race, {1, _}}

    create_job("healthy-progress", DateTime.utc_now())

    assert {:ok, %{status: :completed}} =
             Store.apply_progress(%{
               job_id: "healthy-progress",
               stage: "completed",
               progress: 1.0
             })
  end

  test "watchdog survives a deleted job without counting it as a successful failure write" do
    now = DateTime.utc_now()
    create_job("watchdog-race", DateTime.add(now, -10, :second))
    create_job("watchdog-survivor", DateTime.add(now, -10, :second))
    race = delete_after_read("watchdog-race", :list)
    pid = Process.whereis(Watchdog)

    assert %{stalled: 1, timed_out: 0} = Watchdog.scan_now()
    assert_receive {:interleaved, ^race, {1, _}}
    assert Process.whereis(Watchdog) == pid
    assert Process.alive?(pid)
    assert :error = Store.get("watchdog-race")
    assert {:ok, %{status: :failed}} = Store.get("watchdog-survivor")
    assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
  end

  defp create_job(id, timestamp) do
    assert {:ok, _} =
             Store.create(%{
               job_id: id,
               project_id: "race-test",
               simulation_case_id: "case",
               status: :solving,
               progress: 0.1,
               created_at: timestamp,
               execution_started_at: timestamp,
               updated_at: timestamp
             })
  end

  defp delete_after_read(job_id, selector) do
    repo = Storage.repo_module!()

    JobQueryRace.after_read(selector, fn ->
      apply(repo, :delete_all, [from(job in JobRecord, where: job.job_id == ^job_id)])
    end)
  end
end
