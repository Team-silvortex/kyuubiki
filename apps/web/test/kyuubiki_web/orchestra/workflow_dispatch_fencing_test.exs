defmodule KyuubikiWeb.Orchestra.WorkflowDispatchFencingTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
  end

  test "dispatch admission checks ownership without recording progress or refreshing liveness" do
    fixture = Fixture.claimed_job()
    before = :sys.get_state(WorkflowRecoveryCoordinator)
    assert :ok = authorize(fixture)
    Fixture.unchanged(fixture)
    after_check = :sys.get_state(WorkflowRecoveryCoordinator)
    assert after_check.activity == before.activity
    assert after_check.progress == before.progress
  end

  for status <- [:completed, :failed, :cancelled] do
    test "dispatch admission rejects a #{status} job without changing retained data" do
      fixture = Fixture.terminal_job(unquote(status))
      assert {:error, {:workflow_job_terminal, unquote(status)}} = authorize(fixture)
      Fixture.unchanged(fixture)
    end
  end

  test "dispatch admission rejects a retired envelope and stale attempt" do
    fixture = Fixture.claimed_job()
    stale = Map.update!(fixture.claim, "attempt", &(&1 + 1))
    assert {:error, :stale_workflow_execution_claim} = authorize(%{fixture | claim: stale})
    Fixture.unchanged(fixture)

    key = WorkflowRecoveryEnvelope.internal_key()

    runtime =
      Map.update!(fixture.runtime, key, &WorkflowRecoveryEnvelope.transition(&1, "cancelled"))

    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)
    assert {:error, :stale_workflow_execution_claim} = authorize(fixture)
    Fixture.unchanged(%{fixture | runtime: runtime})
  end

  test "dispatch admission rejects a replaced execution generation" do
    fixture = Fixture.claimed_job()
    key = WorkflowRecoveryEnvelope.internal_key()

    assert {:ok, next, _} =
             WorkflowRecoveryEnvelope.claim(fixture.runtime[key], "other-owner", :process_restart)

    runtime = Map.put(fixture.runtime, key, next)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)
    assert {:error, :stale_workflow_execution_claim} = authorize(fixture)
    Fixture.unchanged(%{fixture | runtime: runtime})
  end

  test "dispatch admission cannot bypass a revoked Orchestra lease" do
    fixture = Fixture.claimed_job()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = authorize(fixture)
    Fixture.unchanged(fixture)
  end

  defp authorize(fixture),
    do:
      WorkflowRecoveryCoordinator.record_progress(fixture.id, fixture.claim, %{
        "event" => "operator_dispatch",
        "node_id" => "solve"
      })
end
