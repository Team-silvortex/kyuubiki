defmodule KyuubikiWeb.Orchestra.WorkflowCommitTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
  end

  test "rejected progress cannot publish a node receipt ahead of the job" do
    fixture = Fixture.claimed_job(progress: 0.8)

    assert {:error, _reason} =
             WorkflowRecoveryCoordinator.record_progress(
               fixture.id,
               fixture.claim,
               Fixture.progress()
             )

    Fixture.unchanged(fixture)

    assert :ok =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )

    assert {:ok, %{status: :completed}} = Store.get(fixture.id)
  end

  test "rejected final progress cannot discard the recovery envelope or publish a completed runtime" do
    fixture = Fixture.claimed_job()

    assert {:ok, future_job} =
             Store.apply_progress(%{
               job_id: fixture.id,
               stage: :solving,
               progress: 0.2,
               emitted_at: DateTime.add(DateTime.utc_now(), 60, :second)
             })

    assert {:error, _} =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )

    Fixture.unchanged(%{fixture | job: future_job})
  end

  test "an invalid failure message cannot partially retire a workflow" do
    fixture = Fixture.claimed_job()

    assert {:error, _} =
             WorkflowRecoveryCoordinator.fail(
               fixture.id,
               fixture.claim,
               String.duplicate("x", 4097)
             )

    Fixture.unchanged(fixture)
    assert :ok = WorkflowRecoveryCoordinator.fail(fixture.id, fixture.claim, "solver failed")
    assert {:ok, %{status: :failed}} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime[WorkflowRecoveryEnvelope.internal_key()]["state"] == "failed"
  end

  for status <- [:completed, :failed, :cancelled] do
    test "late failure cannot rewrite recovery state after a #{status} job" do
      fixture = Fixture.claimed_job()

      assert {:ok, terminal} =
               Store.apply_progress(%{job_id: fixture.id, stage: unquote(status), progress: 1.0})

      assert {:error, {:workflow_job_terminal, unquote(status)}} =
               WorkflowRecoveryCoordinator.fail(fixture.id, fixture.claim, "late failure")

      Fixture.unchanged(%{fixture | job: terminal})
    end
  end

  test "cancellation commits the job and recovery state together" do
    fixture = Fixture.claimed_job()
    assert :ok = WorkflowRecoveryCoordinator.cancel(fixture.id)
    assert {:ok, cancelled} = Store.get(fixture.id)
    assert cancelled.status == :cancelled
    assert cancelled.progress == fixture.job.progress
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime[WorkflowRecoveryEnvelope.internal_key()]["state"] == "cancelled"

    assert {:error, _} =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )

    Fixture.unchanged(%{fixture | job: cancelled, runtime: runtime})
  end

  test "cancelling a completed workflow cannot rewrite its final receipt" do
    fixture = Fixture.claimed_job()

    assert :ok =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )

    assert {:ok, completed} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert :ok = WorkflowRecoveryCoordinator.cancel(fixture.id)
    Fixture.unchanged(%{fixture | job: completed, runtime: runtime})
  end

  test "old execution claims cannot publish progress completion or failure" do
    fixture = Fixture.claimed_job()
    key = WorkflowRecoveryEnvelope.internal_key()

    assert {:ok, next, _claim} =
             WorkflowRecoveryEnvelope.claim(fixture.runtime[key], "new-owner", :process_restart)

    runtime = Map.put(fixture.runtime, key, next)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)

    for operation <- [:progress, :complete, :fail] do
      reply =
        case operation do
          :progress ->
            WorkflowRecoveryCoordinator.record_progress(
              fixture.id,
              fixture.claim,
              Fixture.progress()
            )

          :complete ->
            WorkflowRecoveryCoordinator.commit_result(fixture.id, fixture.claim, Fixture.result())

          :fail ->
            WorkflowRecoveryCoordinator.fail(fixture.id, fixture.claim, "stale")
        end

      assert {:error, :stale_workflow_execution_claim} = reply
      Fixture.unchanged(%{fixture | runtime: runtime})
    end
  end

  test "a changed attempt cannot borrow a valid generation and session" do
    fixture = Fixture.claimed_job()
    claim = Map.put(fixture.claim, "attempt", fixture.claim["attempt"] + 1)

    assert {:error, :stale_workflow_execution_claim} =
             WorkflowRecoveryCoordinator.commit_result(fixture.id, claim, Fixture.result())

    Fixture.unchanged(fixture)
  end

  test "an invalid final result keeps the coordinator alive and the envelope retryable" do
    fixture = Fixture.claimed_job()
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)

    for invalid <- [nil, %{}, "invalid"] do
      result = Map.put(Fixture.result(), "failed_nodes", invalid)

      assert {:error, {:invalid_workflow_result, :failed_nodes}} =
               WorkflowRecoveryCoordinator.commit_result(fixture.id, fixture.claim, result)

      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
      Fixture.unchanged(fixture)
    end

    assert :ok =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )
  end

  test "an unencodable result rolls back without losing the lease or recovery payload" do
    fixture = Fixture.claimed_job()
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)
    result = Map.put(Fixture.result(), "bad", fn -> :not_json end)

    assert {:error, {:completion_persistence_failed, _}} =
             WorkflowRecoveryCoordinator.commit_result(fixture.id, fixture.claim, result)

    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
    assert %{"lease" => %{"status" => "owner"}} = WorkflowRecoveryCoordinator.snapshot()
    Fixture.unchanged(fixture)

    assert :ok =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )
  end

  test "a revoked Orchestra lease cannot publish a final result" do
    fixture = Fixture.claimed_job()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)

    assert {:error, :orchestra_lease_lost} =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )

    Fixture.unchanged(fixture)
    assert %{"lease" => %{"status" => "standby"}} = WorkflowRecoveryCoordinator.snapshot()
  end

  test "plain solver cancellation does not invent a workflow result" do
    id = "plain-solver-cancel"
    assert {:ok, _} = Store.create(%{job_id: id, project_id: "plain", simulation_case_id: "case"})
    assert :ok = WorkflowRecoveryCoordinator.cancel(id)
    assert {:ok, %{status: :cancelled, message: "job cancelled by operator"}} = Store.get(id)
    assert :error = AnalysisResultStore.get(id)
  end
end
