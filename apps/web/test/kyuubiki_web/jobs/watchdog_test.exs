defmodule KyuubikiWeb.Jobs.WatchdogTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.{Store, Watchdog}

  setup do
    Store.reset()

    original_config = Application.get_env(:kyuubiki_web, Watchdog, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, Watchdog, original_config)
    end)

    :ok
  end

  test "marks stale active jobs as failed" do
    now = DateTime.utc_now()

    {:ok, job} =
      Store.create(%{
        job_id: "stale-job",
        project_id: "project-a",
        simulation_case_id: "case-a",
        status: :solving,
        progress: 0.4,
        created_at: DateTime.add(now, -20, :second),
        updated_at: DateTime.add(now, -10, :second)
      })

    assert job.status == :solving

    Application.put_env(:kyuubiki_web, Watchdog,
      scan_interval_ms: 60_000,
      stale_job_ms: 5_000,
      job_timeout_ms: 60_000
    )

    assert %{stalled: 1, timed_out: 0} = Watchdog.scan_now()
    assert {:ok, updated_job} = Store.get("stale-job")
    assert updated_job.status == :failed
    assert updated_job.message =~ "stalled"
  end

  test "marks overlong jobs as failed before stale scan" do
    now = DateTime.utc_now()

    {:ok, _job} =
      Store.create(%{
        job_id: "timed-job",
        project_id: "project-b",
        simulation_case_id: "case-b",
        status: :solving,
        progress: 0.7,
        created_at: DateTime.add(now, -180, :second),
        updated_at: DateTime.add(now, -1, :second)
      })

    Application.put_env(:kyuubiki_web, Watchdog,
      scan_interval_ms: 60_000,
      stale_job_ms: 60_000,
      job_timeout_ms: 5_000
    )

    assert %{stalled: 0, timed_out: 1} = Watchdog.scan_now()
    assert {:ok, updated_job} = Store.get("timed-job")
    assert updated_job.status == :failed
    assert updated_job.message =~ "timed out"
  end
end
