defmodule KyuubikiWeb.Orchestra.WorkflowFailureRetirementTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.StorageOutageWorkflow, as: Workflow
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"

  setup do
    Outage.setup()
    Workflow.install(self())
    Workflow.configure(self(), :controlled)
    :ok
  end

  for {reason, state, marker} <- [
        {:solver_did_not_converge, "failed", "solver_did_not_converge"},
        {{:native_workflow_recovery_blocked, :receipt_invalid}, "recovery_blocked",
         "native_workflow_recovery_receipt_invalid"},
        {{:workflow_solve_checkpoint_blocked, :commit_failed}, "recovery_blocked",
         "workflow_solve_checkpoint_commit_failed"},
        {{:workflow_branch_checkpoint_blocked, :input_changed}, "recovery_blocked",
         "workflow_branch_checkpoint_input_changed"}
      ] do
    @tag skip: Storage.postgres?()
    test "rejected #{marker} finalization waits without replay and preserves the original reason" do
      {id, solver, runner, ref} = submit()
      {:ok, before} = AnalysisResultStore.get(id)
      drop = reject_result_update(id)
      send(solver, {:reply, {:error, unquote(Macro.escape(reason))}})

      refute_receive {:DOWN, ^ref, :process, ^runner, _}, 600
      assert Process.alive?(runner)
      assert {:ok, ^before} = AnalysisResultStore.get(id)
      assert Coordinator.recover_now()["recovered"] == 0
      refute_receive {:outage_solver_request, ^id, _}, 20

      drop.()
      assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
      assert {:ok, %{status: :failed, message: message}} = Store.get(id)
      assert message =~ unquote(marker)
      assert {:ok, after_failure} = AnalysisResultStore.get(id)
      assert after_failure[@key]["state"] == unquote(state)
      assert after_failure[@key]["generation"] == before[@key]["generation"]
      assert after_failure[@key]["attempt"] == before[@key]["attempt"]
      refute get_in(after_failure, ["artifacts", "output.result"])

      if unquote(state) == "recovery_blocked" do
        assert List.last(after_failure[@key]["history"])["reason"] == unquote(marker)
      end

      Fixture.restart_coordinator()
      assert Coordinator.recover_now()["recovered"] == 0
      refute_receive {:outage_solver_request, ^id, _}, 50
      assert {:ok, ^after_failure} = AnalysisResultStore.get(id)
    end
  end

  @tag skip: Storage.postgres?()
  test "a real rejected solve checkpoint commit cannot publish success or repeat the solver" do
    {id, solver, runner, ref} = submit()
    drop = reject_result_update(id)
    send(solver, {:reply, {:ok, %{"computed" => true}}})
    refute_receive {:DOWN, ^ref, :process, ^runner, _}, 600
    assert {:ok, pending} = AnalysisResultStore.get(id)
    refute Map.has_key?(pending[@key], "solve_checkpoints")
    assert pending[@key]["state"] == "running"
    refute get_in(pending, ["artifacts", "output.result"])

    drop.()
    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert {:ok, %{status: :failed, message: message}} = Store.get(id)
    assert message =~ "workflow_solve_checkpoint_commit_failed"
    assert {:ok, ended} = AnalysisResultStore.get(id)
    assert ended[@key]["state"] == "recovery_blocked"
    assert ended[@key]["generation"] == 1
    refute get_in(ended, ["artifacts", "output.result"])
    Fixture.restart_coordinator()
    assert Coordinator.recover_now()["recovered"] == 0
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  for field <- ~w(generation attempt owner_session_id) do
    @tag skip: Storage.postgres?()
    test "a waiting failure cannot overwrite a newer #{field} claim" do
      {id, runner, ref, drop} = waiting_failure()

      changed =
        while_coordinator_suspended(fn ->
          drop.()
          {:ok, runtime} = AnalysisResultStore.get(id)
          value = if unquote(field) == "owner_session_id", do: "replacement-owner", else: 2
          next = put_in(runtime, [@key, unquote(field)], value)
          assert :ok = AnalysisResultStore.compare_and_swap(id, runtime, next)
          next
        end)

      assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
      # Normal runner-loss policy may start newer work, but the old failure cannot end it.
      assert_receive {:outage_solver_request, ^id, :controlled}, 2_000
      assert_receive {:outage_solver_waiting, ^id, fresh_solver}, 2_000
      assert {:ok, current} = AnalysisResultStore.get(id)
      assert current[@key]["generation"] == changed[@key]["generation"] + 1
      assert current[@key]["attempt"] == changed[@key]["attempt"] + 1
      assert {:ok, %{status: :solving, message: message}} = Store.get(id)
      refute (message || "") =~ "solver_did_not_converge"
      send(fresh_solver, {:reply, {:ok, %{"fresh" => true}}})
      Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(id)) end)
      assert {:ok, finished} = AnalysisResultStore.get(id)
      assert finished["artifacts"]["output.result"]["fresh"]
      refute_receive {:outage_solver_request, ^id, _}, 50
    end
  end

  for status <- [:cancelled, :completed, :failed] do
    @tag skip: Storage.postgres?()
    test "a waiting failure cannot overwrite a competing #{status} terminal commit" do
      {id, runner, ref, drop} = waiting_failure()

      {job, runtime} =
        while_coordinator_suspended(fn ->
          drop.()
          {:ok, previous} = AnalysisResultStore.get(id)
          {:ok, expected} = Store.get(id)

          runtime =
            Map.update!(previous, @key, fn recovery ->
              WorkflowRecoveryEnvelope.transition(recovery, Atom.to_string(unquote(status)))
            end)

          assert {:ok, job} =
                   Store.apply_progress_with_result(
                     %{
                       job_id: id,
                       stage: unquote(status),
                       progress: 1.0,
                       message: "competing terminal decision"
                     },
                     expected,
                     previous,
                     runtime
                   )

          {job, runtime}
        end)

      assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
      assert {:ok, ^job} = Store.get(id)
      assert {:ok, ^runtime} = AnalysisResultStore.get(id)
      Fixture.restart_coordinator()
      assert Coordinator.recover_now()["recovered"] == 0
      assert {:ok, ^job} = Store.get(id)
      assert {:ok, ^runtime} = AnalysisResultStore.get(id)
      refute_receive {:outage_solver_request, ^id, _}, 50
    end
  end

  @tag skip: Storage.postgres?()
  test "a coordinator restart during failure persistence keeps the original waiter and reason" do
    {id, runner, ref, drop} = waiting_failure()

    Outage.without(Coordinator, fn ->
      refute_receive {:DOWN, ^ref, :process, ^runner, _}, 600
      assert Process.alive?(runner)
      drop.()
    end)

    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert {:ok, %{status: :failed, message: message}} = Store.get(id)
    assert message =~ "solver_did_not_converge"
    assert {:ok, runtime} = AnalysisResultStore.get(id)
    assert runtime[@key]["generation"] == 1
    assert runtime[@key]["attempt"] == 1
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  @tag skip: Storage.postgres?()
  test "revoked authority stops the failure waiter without a stale terminal write" do
    {id, runner, ref, drop} = waiting_failure()
    {:ok, before} = AnalysisResultStore.get(id)
    {:ok, job} = Store.get(id)

    while_coordinator_suspended(fn ->
      # Lease state is separate from the rejected job/result write.
      name = :sys.get_state(Coordinator).lease_name
      assert {:ok, lease} = LeaseStore.current(name)
      assert :ok = LeaseStore.release(lease)
      assert {:ok, takeover} = LeaseStore.acquire(name, "replacement-orchestra", 60_000)
      on_exit(fn -> LeaseStore.release(takeover) end)
    end)

    assert_receive {:DOWN, ^ref, :process, ^runner, :shutdown}, 2_000
    assert {:ok, ^before} = AnalysisResultStore.get(id)
    assert {:ok, ^job} = Store.get(id)
    drop.()
    assert Coordinator.snapshot()["lease"]["status"] == "standby"
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  defp waiting_failure do
    {id, solver, runner, ref} = submit()
    drop = reject_result_update(id)
    send(solver, {:reply, {:error, :solver_did_not_converge}})
    refute_receive {:DOWN, ^ref, :process, ^runner, _}, 300
    {id, runner, ref, drop}
  end

  defp while_coordinator_suspended(callback) do
    :ok = :sys.suspend(Coordinator)

    try do
      callback.()
    after
      :sys.resume(Coordinator)
    end
  end

  defp submit do
    assert {:ok, submitted} = Workflow.submit("idempotent")
    id = submitted["job"]["job_id"]
    assert_receive {:outage_solver_request, ^id, :controlled}, 2_000
    assert_receive {:outage_solver_waiting, ^id, solver}, 2_000
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    {id, solver, runner, Process.monitor(runner)}
  end

  defp reject_result_update(id) do
    if Storage.sqlite?() do
      AnalysisCommitFault.reject_result_update(id)
    else
      path = Persistence.analysis_state_path() <> ".next"
      File.mkdir_p!(path)
      File.write!(Path.join(path, "blocker"), "injected fault")
      cleanup = fn -> File.rm_rf!(path) end
      on_exit(cleanup)
      cleanup
    end
  end
end
