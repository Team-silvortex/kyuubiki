defmodule KyuubikiWeb.Orchestra.WorkflowProgressFencingTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowNodeProgress, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
  end

  test "healthy throttled node progress checks its claim without writing or refreshing liveness" do
    {fixture, progress} = throttled_progress()
    assert :ok = record(fixture, progress)
    Fixture.unchanged(fixture)
  end

  for status <- [:completed, :failed, :cancelled] do
    test "throttled node progress cannot cross a #{status} job fence" do
      {fixture, progress} = throttled_progress()

      assert {:ok, terminal} =
               Store.apply_progress(%{
                 job_id: fixture.id,
                 stage: unquote(status),
                 progress: 1.0
               })

      assert {:error, {:workflow_job_terminal, unquote(status)}} = record(fixture, progress)
      Fixture.unchanged(%{fixture | job: terminal})
    end
  end

  test "public cancellation fences throttled node progress without changing its recovery record" do
    {fixture, progress} = throttled_progress()
    assert :ok = WorkflowRecoveryCoordinator.cancel(fixture.id)
    {:ok, job} = Store.get(fixture.id)
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime[WorkflowRecoveryEnvelope.internal_key()]["state"] == "cancelled"
    assert {:error, {:workflow_job_terminal, :cancelled}} = record(fixture, progress)
    Fixture.unchanged(%{fixture | job: job, runtime: runtime})
  end

  test "throttled node progress rejects a retired recovery envelope even with an active job" do
    {fixture, progress} = throttled_progress()
    key = WorkflowRecoveryEnvelope.internal_key()
    recovery = WorkflowRecoveryEnvelope.transition(fixture.runtime[key], "cancelled")
    runtime = Map.put(fixture.runtime, key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)
    assert {:error, :stale_workflow_execution_claim} = record(fixture, progress)
    Fixture.unchanged(%{fixture | runtime: runtime})
  end

  test "throttled node progress cannot use a replaced execution claim" do
    {fixture, progress} = throttled_progress()
    key = WorkflowRecoveryEnvelope.internal_key()

    assert {:ok, next, _claim} =
             WorkflowRecoveryEnvelope.claim(fixture.runtime[key], "new-owner", :process_restart)

    runtime = Map.put(fixture.runtime, key, next)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)
    assert {:error, :stale_workflow_execution_claim} = record(fixture, progress)
    Fixture.unchanged(%{fixture | runtime: runtime})
  end

  test "throttled node progress cannot borrow a valid generation with a different attempt" do
    {fixture, progress} = throttled_progress()
    claim = Map.put(fixture.claim, "attempt", fixture.claim["attempt"] + 1)

    assert {:error, :stale_workflow_execution_claim} =
             record(%{fixture | claim: claim}, progress)

    Fixture.unchanged(fixture)
  end

  test "throttled node progress cannot bypass a revoked Orchestra lease" do
    {fixture, progress} = throttled_progress()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = record(fixture, progress)
    Fixture.unchanged(fixture)
  end

  test "a nearby invalid node progress event cannot hide behind the write throttle" do
    {fixture, progress} = throttled_progress()

    assert {:error, :invalid_workflow_progress} =
             record(fixture, Map.put(progress, "node_id", ""))

    Fixture.unchanged(fixture)
  end

  defp record(fixture, progress),
    do: WorkflowRecoveryCoordinator.record_progress(fixture.id, fixture.claim, progress)

  defp throttled_progress do
    fixture = Fixture.claimed_job()
    first = %{"node_id" => "diagnose", "completed_nodes" => 100, "total_nodes" => 1_000}
    assert :ok = record(fixture, first)
    {:ok, job} = Store.get(fixture.id)
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)

    # Pin the throttle window so slow CI cannot accidentally exercise the persisted branch.
    state =
      :sys.replace_state(WorkflowRecoveryCoordinator, fn state ->
        put_in(
          state,
          [:progress, fixture.id, :persisted_at_ms],
          System.monotonic_time(:millisecond) + 60_000
        )
      end)

    progress = Map.merge(first, %{"node_id" => "quality", "completed_nodes" => 101})
    refute WorkflowNodeProgress.persist?(progress, state.progress[fixture.id])
    {%{fixture | job: job, runtime: runtime}, progress}
  end
end
