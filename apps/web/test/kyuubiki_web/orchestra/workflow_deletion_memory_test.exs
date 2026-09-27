defmodule KyuubikiWeb.Orchestra.WorkflowDeletionMemoryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Storage.AnalysisMemoryState
  alias KyuubikiWeb.TestSupport.StorageOutageFixture
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.memory?()

  setup do
    StorageOutageFixture.setup()
  end

  test "deletion persists one whole generation rather than a resultless intermediate job" do
    fixture = Fixture.claimed_job()
    path = Persistence.analysis_state_path()
    before = File.read!(path)
    assert {:ok, %{"deleted" => true}} = Analysis.delete_job(fixture.id)
    assert File.read!(path <> ".previous") == before

    assert :ok = StorageOutageFixture.stop(WorkflowRecoveryCoordinator)
    assert :ok = StorageOutageFixture.stop(AnalysisMemoryState)
    assert :ok = StorageOutageFixture.start(AnalysisMemoryState)
    assert :error = Store.get(fixture.id)
    assert :error = AnalysisResultStore.get(fixture.id)
  end

  test "failed deletion preserves live state and durable bytes and permits a clean retry" do
    fixture = Fixture.claimed_job()
    path = Persistence.analysis_state_path()
    next = path <> ".next"
    before = File.read!(path)
    on_exit(fn -> File.rm_rf!(next) end)
    File.mkdir_p!(next)
    File.write!(Path.join(next, "blocker"), "injected fault")
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)
    store = Process.whereis(AnalysisMemoryState)

    assert {:error, _} = Analysis.delete_job(fixture.id)
    Fixture.unchanged(fixture)
    assert File.read!(path) == before
    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
    assert Process.whereis(AnalysisMemoryState) == store

    File.rm_rf!(next)
    assert {:ok, %{"deleted" => true}} = Analysis.delete_job(fixture.id)
    assert :error = Store.get(fixture.id)
    assert :error = AnalysisResultStore.get(fixture.id)
  end
end
