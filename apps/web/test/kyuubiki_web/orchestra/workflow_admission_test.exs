defmodule KyuubikiWeb.Orchestra.WorkflowAdmissionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture
  alias KyuubikiWeb.TestSupport.WorkflowPreflightContract

  setup do
    StorageOutageFixture.setup()
  end

  test "invalid recovery input cannot create a job or interrupt the coordinator" do
    attrs = attrs()
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)

    assert {:error, :invalid_workflow_recovery_envelope} =
             admit(attrs, %{"invalid" => fn -> :not_json end})

    assert :error = Store.get(attrs.job_id)
    assert :error = AnalysisResultStore.get(attrs.job_id)
    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
    assert {:ok, _job} = admit(attrs)
  end

  test "a revoked Orchestra lease cannot admit even half a workflow" do
    attrs = attrs()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)

    assert {:error, :orchestra_lease_lost} = admit(attrs)
    assert :error = Store.get(attrs.job_id)
    assert :error = AnalysisResultStore.get(attrs.job_id)
    assert %{"lease" => %{"status" => "standby"}} = WorkflowRecoveryCoordinator.snapshot()
  end

  test "an admitted but undispatched workflow resumes after coordinator restart" do
    attrs = attrs()
    assert {:ok, %{status: :queued}} = admit(attrs)
    assert {:ok, runtime} = AnalysisResultStore.get(attrs.job_id)
    assert runtime["_workflow_recovery"]["state"] == "pending"

    WorkflowCommitFixture.restart_coordinator()

    StorageOutageFixture.eventually(fn ->
      match?({:ok, %{status: :completed}}, Store.get(attrs.job_id))
    end)

    assert {:ok, result} = AnalysisResultStore.get(attrs.job_id)
    assert result["artifacts"]["output.payload"] == %{"value" => 42}
    assert result["_workflow_recovery"]["state"] == "completed"
    assert result["_workflow_recovery"]["attempt"] == 1
    assert length(Store.list()) == 1
    assert length(AnalysisResultStore.list()) == 1
  end

  test "initialization cannot create a runtime for a missing job" do
    assert {:error, {:job_not_found, "missing-admission"}} = initialize("missing-admission")
    assert :error = AnalysisResultStore.get("missing-admission")
    assert Store.list() == []
  end

  test "a queued job can be initialized once but not overwritten" do
    job = queued_job()
    assert :ok = initialize(job.job_id)
    assert {:ok, original} = AnalysisResultStore.get(job.job_id)
    assert :ok = WorkflowRecoveryEnvelope.verify(original["_workflow_recovery"])

    assert {:error, {:result_already_exists, _}} = initialize(job.job_id, %{"changed" => true})
    assert {:ok, ^original} = AnalysisResultStore.get(job.job_id)
    assert {:ok, ^job} = Store.get(job.job_id)
  end

  test "initialization cannot replace a running execution generation" do
    fixture = WorkflowCommitFixture.claimed_job()
    assert {:error, _} = initialize(fixture.id)
    WorkflowCommitFixture.unchanged(fixture)
  end

  test "initialization cannot erase a completed workflow receipt" do
    fixture = WorkflowCommitFixture.claimed_job()

    assert :ok =
             WorkflowRecoveryCoordinator.commit_result(
               fixture.id,
               fixture.claim,
               WorkflowCommitFixture.result()
             )

    assert {:ok, job} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert {:error, _} = initialize(fixture.id)
    assert {:ok, ^job} = Store.get(fixture.id)
    assert {:ok, ^runtime} = AnalysisResultStore.get(fixture.id)
  end

  for status <- [:preprocessing, :solving, :completed, :failed, :cancelled] do
    test "initialization rejects an existing #{status} job without a result" do
      job = queued_job()
      stage = unquote(status)

      assert {:ok, current} =
               Store.apply_progress(%{job_id: job.job_id, stage: stage, progress: 1.0})

      assert {:error, {:job_not_queued, _}} = initialize(job.job_id)
      assert {:ok, ^current} = Store.get(job.job_id)
      assert :error = AnalysisResultStore.get(job.job_id)
    end
  end

  defp queued_job do
    {:ok, job} = Store.create(attrs())
    job
  end

  defp attrs do
    %{
      job_id: "admission-#{System.unique_integer([:positive])}",
      project_id: "admission",
      simulation_case_id: "case"
    }
  end

  defp admit(attrs, context \\ %{}) do
    request = WorkflowPreflightContract.request()

    WorkflowRecoveryCoordinator.admit(
      attrs,
      request["graph"],
      request["input_artifacts"],
      context,
      KyuubikiWeb.WorkflowGraphResponse.resolve_options(request["graph"], nil)
    )
  end

  defp initialize(id, context \\ %{}) do
    request = WorkflowPreflightContract.request()

    WorkflowRecoveryCoordinator.initialize(
      id,
      request["graph"],
      request["input_artifacts"],
      context,
      %{}
    )
  end
end
