defmodule KyuubikiWeb.Orchestra.WorkflowWatchdogFailureTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.{Persistence, Storage}
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.StorageOutageWorkflow, as: Workflow
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"

  setup do
    Outage.setup()
    config = Application.get_env(:kyuubiki_web, Watchdog)
    Application.put_env(:kyuubiki_web, Watchdog, Keyword.put(config, :stale_job_ms, 1))
    :ok
  end

  test "periodic watchdog commits local workflow failure with its recovery record" do
    f = Fixture.claimed_job()
    Process.sleep(5)
    assert %{available: true, stalled: 1, timed_out: 0} = Watchdog.scan_now()
    assert {:ok, job} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    assert job.status == :failed
    assert job.progress == f.job.progress
    assert runtime[@key]["state"] == "failed"
    refute Map.has_key?(runtime[@key], "envelope")
    assert List.last(runtime[@key]["history"])["message"] == job.message
  end

  test "periodic watchdog stops a stalled runner only after durable failure" do
    Workflow.install(self())
    assert {:ok, submitted} = Workflow.submit("idempotent")
    id = submitted["job"]["job_id"]
    assert_receive {:outage_solver_request, ^id, :hold}, 2_000
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    on_exit(fn -> if Process.alive?(runner), do: Process.exit(runner, :shutdown) end)
    Process.sleep(5)
    assert %{available: true, stalled: 1} = Watchdog.scan_now()
    assert {:ok, runtime} = AnalysisResultStore.get(id)
    assert runtime[@key]["state"] == "failed"
    assert_receive {:DOWN, ^ref, :process, ^runner, :shutdown}, 1_000
    refute_receive {:outage_solver_request, ^id, _}, 30
  end

  test "a queued pending workflow is retired without inventing an execution claim" do
    attrs = %{job_id: "watchdog-pending", project_id: "watchdog", simulation_case_id: "case"}
    graph = %{"id" => "pending", "nodes" => []}
    assert {:ok, job} = Coordinator.admit(attrs, graph, %{}, %{}, %{})
    config = Application.get_env(:kyuubiki_web, Watchdog)
    Application.put_env(:kyuubiki_web, Watchdog, Keyword.put(config, :queue_timeout_ms, 1))
    Process.sleep(5)
    assert %{available: true, timed_out: 1} = Watchdog.scan_now()
    assert {:ok, runtime} = AnalysisResultStore.get(job.job_id)
    assert runtime[@key]["state"] == "failed"
    assert runtime[@key]["generation"] == 0
    assert runtime[@key]["attempt"] == 0
  end

  test "a stale scan cannot overwrite a fresh heartbeat or its recovery state" do
    f = Fixture.claimed_job()

    assert {:ok, updated} =
             Store.apply_progress(%{
               job_id: f.id,
               stage: :solving,
               progress: f.job.progress,
               message: "fresh heartbeat"
             })

    assert {:error, {:stale_job_snapshot, _}} = Coordinator.fail_observed(f.job, "expired scan")
    Fixture.unchanged(%{f | job: updated})
  end

  for status <- [:completed, :failed, :cancelled] do
    test "a scan cannot rewrite a concurrent #{status} completion" do
      f = Fixture.claimed_job()

      assert {:ok, final} =
               Store.apply_progress(%{job_id: f.id, stage: unquote(status), progress: 1.0})

      assert {:error, {:stale_job_snapshot, _}} = Coordinator.fail_observed(f.job, "expired scan")

      assert {:error, {:workflow_job_terminal, unquote(status)}} =
               Coordinator.fail_observed(final, "repeated scan")

      Fixture.unchanged(%{f | job: final})
    end
  end

  for state <- ~w(completed failed cancelled recovery_blocked) do
    test "a scan does not rewrite already #{state} recovery evidence" do
      f = Fixture.claimed_job()
      recovery = WorkflowRecoveryEnvelope.transition(f.runtime[@key], unquote(state))
      runtime = Map.put(f.runtime, @key, recovery)
      assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)

      assert {:error, :workflow_recovery_not_active} =
               Coordinator.fail_observed(f.job, "expired scan")

      Fixture.unchanged(%{f | runtime: runtime})
    end
  end

  test "an invalid private recovery value is not treated as a plain job" do
    f = Fixture.claimed_job()
    runtime = Map.put(f.runtime, @key, nil)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)

    assert {:error, :invalid_workflow_recovery_record} =
             Coordinator.fail_observed(f.job, "expired scan")

    Fixture.unchanged(%{f | runtime: runtime})
  end

  test "loss of coordinator availability leaves the watchdog and both snapshots intact" do
    f = Fixture.claimed_job()
    watchdog = Process.whereis(Watchdog)
    Process.sleep(5)

    Outage.without(Coordinator, fn ->
      assert %{
               available: false,
               reason: "workflow_coordinator_unavailable",
               stalled: 0,
               timed_out: 0
             } = Watchdog.scan_now()

      assert Process.whereis(Watchdog) == watchdog
      Fixture.unchanged(f)
      Store.reset()
      AnalysisResultStore.reset()
    end)
  end

  test "revoked Orchestra authority cannot retire a workflow from an old scan" do
    f = Fixture.claimed_job()
    name = Coordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, original} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(original)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = Coordinator.fail_observed(f.job, "expired scan")
    Fixture.unchanged(f)
  end

  @tag skip: Storage.postgres?()
  test "failed durable finalization does not kill the runner and a repaired retry succeeds" do
    Workflow.install(self())
    assert {:ok, submitted} = Workflow.submit("idempotent")
    id = submitted["job"]["job_id"]
    assert_receive {:outage_solver_request, ^id, :hold}, 2_000
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    on_exit(fn -> if Process.alive?(runner), do: Process.exit(runner, :shutdown) end)
    assert {:ok, job} = Store.get(id)
    assert {:ok, runtime} = AnalysisResultStore.get(id)
    drop = reject_result_update(id)
    Process.sleep(5)
    assert %{available: false, stalled: 0} = Watchdog.scan_now()
    assert Process.alive?(runner)
    assert {:ok, ^job} = Store.get(id)
    assert {:ok, ^runtime} = AnalysisResultStore.get(id)
    assert Coordinator.snapshot()["lease"]["status"] == "owner"
    drop.()
    assert %{available: true, stalled: 1} = Watchdog.scan_now()
    assert_receive {:DOWN, ^ref, :process, ^runner, :shutdown}, 1_000
    assert {:ok, final} = AnalysisResultStore.get(id)
    assert final[@key]["state"] == "failed"
  end

  defp reject_result_update(id) do
    if Storage.sqlite?() do
      AnalysisCommitFault.reject_result_update(id)
    else
      path = Persistence.analysis_state_path() <> ".next"
      File.mkdir_p!(path)
      File.write!(Path.join(path, "blocker"), "injected fault")
      cleanup = fn -> File.rm_rf!(path) end
      on_exit(cleanup)
      cleanup
    end
  end
end
