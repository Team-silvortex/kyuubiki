defmodule KyuubikiWeb.Orchestra.WorkflowCommitMemoryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Persistence
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.AnalysisMemoryState
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture
  alias KyuubikiWeb.TestSupport.WorkflowSolveCheckpointFixture, as: SolveFixture
  alias KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture, as: BranchFixture

  @moduletag skip: not Storage.memory?()

  setup do
    Fixture.setup_coordinator()
  end

  for operation <- [
        :progress,
        :complete,
        :fail,
        :cancel,
        :native_block,
        :solve_checkpoint,
        :checkpoint_block,
        :branch_checkpoint,
        :branch_checkpoint_block
      ] do
    test "#{operation} survives a failed shared-snapshot write without partial receipts" do
      fixture =
        case unquote(operation) do
          :solve_checkpoint -> SolveFixture.claimed()
          :branch_checkpoint -> BranchFixture.claimed()
          _ -> Fixture.claimed_job()
        end

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

  defp write(:solve_checkpoint, fixture),
    do:
      WorkflowRecoveryCoordinator.solve_checkpoint(
        fixture.id,
        fixture.claim,
        {:commit, fixture.request, 42}
      )

  defp write(:checkpoint_block, fixture),
    do:
      WorkflowRecoveryCoordinator.block_solve_checkpoint(
        fixture.id,
        fixture.claim,
        :commit_failed
      )

  defp write(:branch_checkpoint, fixture),
    do:
      WorkflowRecoveryCoordinator.branch_checkpoint(
        fixture.id,
        fixture.claim,
        {:commit, fixture.request, BranchFixture.decision()}
      )

  defp write(:branch_checkpoint_block, fixture),
    do:
      WorkflowRecoveryCoordinator.block_branch_checkpoint(
        fixture.id,
        fixture.claim,
        :commit_failed
      )

  defp write(:native_block, fixture),
    do:
      WorkflowRecoveryCoordinator.block_native_recovery(
        fixture.id,
        fixture.claim,
        :receipt_unavailable
      )
end
