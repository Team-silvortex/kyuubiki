defmodule KyuubikiWeb.Orchestra.WorkflowCommitSqlTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.sqlite?()

  setup do
    Fixture.setup_coordinator()
  end

  for operation <- [:progress, :complete, :fail, :cancel], target <- [:job, :result] do
    test "#{operation} rolls back when the #{target} write fails and permits a retry" do
      fixture = Fixture.claimed_job()

      drop =
        case unquote(target) do
          :job -> AnalysisCommitFault.reject_job_update(fixture.id)
          :result -> AnalysisCommitFault.reject_result_update(fixture.id)
        end

      coordinator = Process.whereis(WorkflowRecoveryCoordinator)
      assert {:error, _reason} = write(unquote(operation), fixture)
      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
      Fixture.unchanged(fixture)

      drop.()
      assert :ok = write(unquote(operation), fixture)
    end
  end

  test "the cancellation API cannot ignore a rejected workflow commit" do
    fixture = Fixture.claimed_job()
    drop = AnalysisCommitFault.reject_result_update(fixture.id)
    assert {:error, _reason} = Analysis.cancel_job(fixture.id)
    Fixture.unchanged(fixture)
    drop.()
    assert :ok = WorkflowRecoveryCoordinator.cancel(fixture.id)
  end

  test "a failed recovery-block commit is not counted as a durable blocked job" do
    fixture = Fixture.claimed_job()
    key = WorkflowRecoveryEnvelope.internal_key()
    exhausted = put_in(fixture.runtime, [key, "attempt"], 3)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, exhausted)
    drop = AnalysisCommitFault.reject_result_update(fixture.id)

    assert %{"blocked" => 0, "skipped" => 1} = WorkflowRecoveryCoordinator.recover_now()
    Fixture.unchanged(%{fixture | runtime: exhausted})
    drop.()

    assert %{"blocked" => 1, "skipped" => 0} = WorkflowRecoveryCoordinator.recover_now()
    assert {:ok, %{status: :failed}} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime[key]["state"] == "recovery_blocked"
  end

  @tag capture_log: true
  test "process loss rolls back the joint write and releases the SQL lease transaction" do
    fixture = Fixture.claimed_job()
    assert {:ok, lease} = LeaseStore.acquire("joint-write-#{fixture.id}", "writer", 60_000)
    on_exit(fn -> LeaseStore.release(lease) end)
    probe = AnalysisCommitFault.pause_after_job_write(fixture.id)

    update = fn ->
      Store.apply_progress_with_result(
        %{job_id: fixture.id, stage: :completed, progress: 1.0},
        fixture.job,
        fixture.runtime,
        %{"final" => true}
      )
    end

    {pid, ref} = spawn_monitor(fn -> LeaseStore.with_lease(lease, update) end)
    assert_receive {:completion_paused, ^probe, ^pid}, 2_000
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^ref, :process, ^pid, :killed}, 2_000
    Fixture.unchanged(fixture)

    assert {:ok, %{status: :completed}} = LeaseStore.with_lease(lease, update)
    assert {:ok, %{"final" => true}} = AnalysisResultStore.get(fixture.id)
  end

  defp write(:progress, fixture),
    do: WorkflowRecoveryCoordinator.record_progress(fixture.id, fixture.claim, Fixture.progress())

  defp write(:complete, fixture),
    do: WorkflowRecoveryCoordinator.commit_result(fixture.id, fixture.claim, Fixture.result())

  defp write(:fail, fixture),
    do: WorkflowRecoveryCoordinator.fail(fixture.id, fixture.claim, "solver failure")

  defp write(:cancel, fixture), do: WorkflowRecoveryCoordinator.cancel(fixture.id)
end
