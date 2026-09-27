defmodule KyuubikiWeb.Orchestra.WorkflowAdmissionMemoryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Storage.AnalysisMemoryState
  alias KyuubikiWeb.TestSupport.{StorageOutageFixture, WorkflowPreflightContract}

  @moduletag skip: not Storage.memory?()

  setup do
    StorageOutageFixture.setup()
  end

  test "failed admission leaves the shared snapshot unchanged and can be retried" do
    attrs = attrs()
    path = Persistence.analysis_state_path()
    next = path <> ".next"
    before = File.read!(path)
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)
    store = Process.whereis(AnalysisMemoryState)
    File.mkdir_p!(next)
    File.write!(Path.join(next, "blocker"), "injected fault")
    on_exit(fn -> File.rm_rf!(next) end)

    assert {:error, {:completion_persistence_failed, _}} = admit(attrs)
    assert :error = Store.get(attrs.job_id)
    assert :error = AnalysisResultStore.get(attrs.job_id)
    assert File.read!(path) == before
    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
    assert Process.whereis(AnalysisMemoryState) == store

    File.rm_rf!(next)
    assert {:ok, _job} = admit(attrs)
    assert {:ok, _runtime} = AnalysisResultStore.get(attrs.job_id)
  end

  test "a restarted memory store restores both halves of an admitted workflow" do
    attrs = attrs()
    assert {:ok, job} = admit(attrs)
    assert {:ok, runtime} = AnalysisResultStore.get(attrs.job_id)
    assert :ok = StorageOutageFixture.stop(WorkflowRecoveryCoordinator)
    assert :ok = StorageOutageFixture.stop(AnalysisMemoryState)
    assert :ok = StorageOutageFixture.start(AnalysisMemoryState)
    assert {:ok, ^job} = Store.get(attrs.job_id)
    assert {:ok, ^runtime} = AnalysisResultStore.get(attrs.job_id)
  end

  defp attrs do
    %{
      job_id: "memory-admission-#{System.unique_integer([:positive])}",
      project_id: "admission",
      simulation_case_id: "case"
    }
  end

  defp admit(attrs) do
    request = WorkflowPreflightContract.request()

    WorkflowRecoveryCoordinator.admit(
      attrs,
      request["graph"],
      request["input_artifacts"],
      %{},
      %{}
    )
  end
end
