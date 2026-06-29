defmodule KyuubikiWeb.Jobs.WatchdogTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Playground.AgentRegistry

  setup do
    Store.reset()
    Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)

    original_config = Application.get_env(:kyuubiki_web, Watchdog, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, Watchdog, original_config)
      Store.reset()
      Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)
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

  test "summarizes orchestra agent and operator load posture" do
    now = DateTime.utc_now()

    for index <- 1..3 do
      {:ok, _job} =
        Store.create(%{
          job_id: "thermal-hot-#{index}",
          project_id: "project-load",
          simulation_case_id: "case-load-#{index}",
          worker_id: "solve.heat_plane_quad_2d",
          status: :solving,
          progress: 0.35,
          created_at: DateTime.add(now, -1, :second),
          updated_at: DateTime.add(now, -1, :second)
        })
    end

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "watchdog-agent-a",
               "host" => "127.0.0.1",
               "port" => 6901,
               "capacity" => 2,
               "orch_id" => "orchestra/default",
               "methods" => ["solve.heat_plane_quad_2d"]
             })

    assert {:ok, _lease} =
             AgentRegistry.claim_execution("watchdog-agent-a", %{
               "lease_id" => "lease-thermal-a",
               "job_id" => "thermal-hot-1",
               "method" => "solve.heat_plane_quad_2d"
             })

    Application.put_env(:kyuubiki_web, Watchdog,
      scan_interval_ms: 60_000,
      stale_job_ms: 60_000,
      job_timeout_ms: 120_000,
      orchestra_warn_active_jobs: 2,
      orchestra_critical_active_jobs: 4,
      operator_warn_active_jobs: 2,
      operator_critical_active_jobs: 3,
      agent_warn_utilization: 0.5,
      agent_critical_utilization: 0.95
    )

    snapshot = Watchdog.status_snapshot()

    assert snapshot.watchdog_state == "critical"
    assert snapshot.orchestra_load.state == "watch"
    assert snapshot.orchestra_load.active_jobs == 3
    assert snapshot.agent_load.available == true
    assert snapshot.agent_load.capacity_slots == 2
    assert snapshot.agent_load.leased_slots == 1
    assert snapshot.agent_load.state == "watch"
    assert snapshot.operator_load.state == "critical"

    assert [
             %{
               operator: "worker:solve.heat_plane_quad_2d",
               active_jobs: 3,
               state: "critical"
             }
             | _
           ] = snapshot.operator_load.hotspots
  end
end
