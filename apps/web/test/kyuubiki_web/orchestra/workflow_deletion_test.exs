defmodule KyuubikiWeb.Orchestra.WorkflowDeletionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.TestSupport.{StorageOutageFixture, StorageOutageWorkflow}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    StorageOutageFixture.setup()
  end

  test "a revoked Orchestra lease cannot delete the job or its recovery record" do
    fixture = Fixture.claimed_job()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)

    assert {:error, :orchestra_lease_lost} = Analysis.delete_job(fixture.id)
    Fixture.unchanged(fixture)
    assert {:error, :orchestra_standby} = Analysis.delete_job(fixture.id)
    Fixture.unchanged(fixture)
  end

  test "deleting a running workflow prevents late progress and completion from restoring it" do
    fixture = Fixture.claimed_job()
    assert {:ok, %{"deleted" => true}} = Analysis.delete_job(fixture.id)
    assert :error = Store.get(fixture.id)
    assert :error = AnalysisResultStore.get(fixture.id)

    assert {:error, _} =
             WorkflowRecoveryCoordinator.record_progress(
               fixture.id,
               fixture.claim,
               Fixture.progress()
             )

    assert {:error, _} =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               Fixture.result()
             )

    assert :error = Store.get(fixture.id)
    assert :error = AnalysisResultStore.get(fixture.id)
  end

  for status <- [:completed, :failed, :cancelled] do
    test "deleting a #{status} job removes its result without changing another job" do
      fixture = Fixture.claimed_job()
      retained = Fixture.claimed_job()

      assert {:ok, _} =
               Store.apply_progress(%{job_id: fixture.id, stage: unquote(status), progress: 1.0})

      assert {:ok, %{"deleted" => true}} = Analysis.delete_job(fixture.id)
      assert :error = Store.get(fixture.id)
      assert :error = AnalysisResultStore.get(fixture.id)
      Fixture.unchanged(retained)
    end
  end

  test "a job without a result can be deleted and a repeat request is not found" do
    id = "delete-plain-job"
    assert {:ok, _} = Store.create(%{job_id: id, project_id: "plain", simulation_case_id: "case"})
    assert {:ok, %{"deleted" => true}} = Analysis.delete_job(id)
    assert {:error, {:job_not_found, ^id}} = Analysis.delete_job(id)
    assert :error = Store.get(id)
    assert :error = AnalysisResultStore.get(id)
  end

  test "successful deletion stops a tracked local runner and does not recover it" do
    StorageOutageWorkflow.install(self())
    assert {:ok, submitted} = StorageOutageWorkflow.submit("idempotent")
    id = submitted["job"]["job_id"]
    assert_receive {:outage_solver_request, ^id, :hold}, 2_000
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    on_exit(fn -> if Process.alive?(runner), do: Process.exit(runner, :shutdown) end)

    assert {:ok, retained} = StorageOutageWorkflow.submit("idempotent")
    retained_id = retained["job"]["job_id"]
    assert_receive {:outage_solver_request, ^retained_id, :hold}, 2_000
    assert {:ok, other_runner} = WorkflowJobRunner.running(retained_id)
    on_exit(fn -> if Process.alive?(other_runner), do: Process.exit(other_runner, :shutdown) end)

    assert {:ok, %{"deleted" => true}} = Analysis.delete_job(id)
    assert_receive {:DOWN, ^ref, :process, ^runner, _reason}, 1_000

    StorageOutageFixture.eventually(fn ->
      id not in WorkflowRecoveryCoordinator.snapshot()["tracked_jobs"]
    end)

    assert %{"recovered" => 0} = WorkflowRecoveryCoordinator.recover_now()
    assert :error = Store.get(id)
    assert :error = AnalysisResultStore.get(id)
    assert :error = WorkflowJobRunner.running(id)
    assert {:ok, ^other_runner} = WorkflowJobRunner.running(retained_id)
    assert Process.alive?(other_runner)
    assert {:ok, %{status: :solving}} = Store.get(retained_id)
    assert {:ok, _runtime} = AnalysisResultStore.get(retained_id)
  end
end
