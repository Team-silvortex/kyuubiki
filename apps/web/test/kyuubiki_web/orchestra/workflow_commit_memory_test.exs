defmodule KyuubikiWeb.Orchestra.WorkflowCommitMemoryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Persistence
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.AnalysisMemoryState
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.memory?()

  setup do
    Fixture.setup_coordinator()
  end

  for operation <- [:progress, :complete, :fail, :cancel] do
    test "#{operation} survives a failed shared-snapshot write without partial receipts" do
      fixture = Fixture.claimed_job()
      coordinator = Process.whereis(WorkflowRecoveryCoordinator)
      store = Process.whereis(AnalysisMemoryState)
      next = Persistence.analysis_state_path() <> ".next"
      before = File.read!(Persistence.analysis_state_path())
      File.mkdir_p!(next)
      File.write!(Path.join(next, "blocker"), "injected fault")
      on_exit(fn -> File.rm_rf!(next) end)

      assert {:error, {:completion_persistence_failed, _reason}} =
               write(unquote(operation), fixture)

      assert File.read!(Persistence.analysis_state_path()) == before
      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
      assert Process.whereis(AnalysisMemoryState) == store
      Fixture.unchanged(fixture)

      File.rm_rf!(next)
      assert :ok = write(unquote(operation), fixture)
    end
  end

  defp write(:progress, fixture),
    do: WorkflowRecoveryCoordinator.record_progress(fixture.id, fixture.claim, Fixture.progress())

  defp write(:complete, fixture),
    do: WorkflowRecoveryCoordinator.commit_result(fixture.id, fixture.claim, Fixture.result())

  defp write(:fail, fixture),
    do: WorkflowRecoveryCoordinator.fail(fixture.id, fixture.claim, "solver failure")

  defp write(:cancel, fixture), do: WorkflowRecoveryCoordinator.cancel(fixture.id)
end
