defmodule KyuubikiWeb.Results.AdministrationMemoryTest do
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

  for action <- [:replace, :delete] do
    test "failed #{action} preserves durable bytes and processes, then retries" do
      fixture = Fixture.terminal_job()
      path = Persistence.analysis_state_path()
      next = path <> ".next"
      before = File.read!(path)
      on_exit(fn -> File.rm_rf!(next) end)
      File.mkdir_p!(next)
      File.write!(Path.join(next, "blocker"), "injected fault")
      coordinator = Process.whereis(WorkflowRecoveryCoordinator)
      store = Process.whereis(AnalysisMemoryState)

      assert {:error, {:completion_persistence_failed, _}} = edit(fixture.id, unquote(action))
      Fixture.unchanged(fixture)
      assert File.read!(path) == before
      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
      assert Process.whereis(AnalysisMemoryState) == store

      File.rm_rf!(next)
      assert {:ok, _} = edit(fixture.id, unquote(action))
      assert File.read!(path <> ".previous") == before
      assert {:ok, job} = Store.get(fixture.id)
      assert job == fixture.job
    end

    test "successful #{action} survives a fresh store process without modifying the job" do
      fixture = Fixture.terminal_job()
      assert {:ok, _} = edit(fixture.id, unquote(action))
      expected = AnalysisResultStore.get(fixture.id)
      assert :ok = StorageOutageFixture.stop(WorkflowRecoveryCoordinator)
      assert :ok = StorageOutageFixture.stop(AnalysisMemoryState)
      assert :ok = StorageOutageFixture.start(AnalysisMemoryState)
      assert AnalysisResultStore.get(fixture.id) == expected
      assert {:ok, job} = Store.get(fixture.id)
      assert job == fixture.job
    end
  end

  defp edit(id, :replace), do: Analysis.update_result(id, %{"reviewed" => true})
  defp edit(id, :delete), do: Analysis.delete_result(id)
end
