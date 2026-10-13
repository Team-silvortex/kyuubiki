defmodule KyuubikiWeb.Orchestra.WorkflowCompletionRetryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, WorkflowCompletionProbe}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.StorageOutageWorkflow, as: Workflow
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"
  @moduletag skip: Storage.postgres?()

  setup do
    Outage.setup()
    Workflow.install(self())
    Workflow.configure(self(), :controlled)
    :ok
  end

  test "coordinator restart retains the successful output without another solver dispatch" do
    {id, runner, ref, _probe, claim, result, drop} = waiting_completion()

    Outage.without(Coordinator, fn ->
      refute_receive {:DOWN, ^ref, :process, ^runner, _}, 600
      assert Process.alive?(runner)
      drop.()
    end)

    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert {:ok, %{status: :completed}} = Store.get(id)
    assert {:ok, final} = AnalysisResultStore.get(id)
    assert Map.take(final, Map.keys(result)) == result
    assert final[@key]["generation"] == claim["generation"]
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  for field <- ~w(generation attempt owner_session_id) do
    test "pending completion cannot publish through a changed #{field} claim" do
      {id, runner, ref, _probe, _claim, _result, drop} = waiting_completion()

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
      Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(id)) end)
      assert {:ok, final} = AnalysisResultStore.get(id)
      assert final[@key]["generation"] == changed[@key]["generation"]
      assert final[@key]["attempt"] == changed[@key]["attempt"]
      assert final[@key]["state"] == "recovery_blocked"

      assert List.last(final[@key]["history"])["reason"] ==
               "workflow_completion_checkpoint_invalid"

      # A mismatched durable finalizer must never fall back to graph execution.
      refute_receive {:outage_solver_request, ^id, _}, 50
    end
  end

  for status <- [:cancelled, :completed, :failed] do
    test "pending completion cannot replace a competing #{status} decision" do
      {id, runner, ref, _probe, _claim, _result, drop} = waiting_completion()

      {job, runtime} =
        while_coordinator_suspended(fn ->
          drop.()
          {:ok, previous} = AnalysisResultStore.get(id)
          {:ok, expected} = Store.get(id)

          runtime =
            previous
            |> Map.update!(@key, fn recovery ->
              WorkflowRecoveryEnvelope.transition(recovery, Atom.to_string(unquote(status)))
            end)
            |> Map.put("artifacts", %{"competing" => true})

          assert {:ok, job} =
                   Store.apply_progress_with_result(
                     %{
                       job_id: id,
                       stage: unquote(status),
                       progress: 1.0,
                       message: "competing decision"
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
      assert {:ok, ^runtime} = AnalysisResultStore.get(id)
      refute_receive {:outage_solver_request, ^id, _}, 50
    end
  end

  test "lease takeover stops pending completion without publishing the old result" do
    {id, runner, ref, _probe, _claim, _result, drop} = waiting_completion()
    {:ok, before} = AnalysisResultStore.get(id)
    {:ok, job} = Store.get(id)

    while_coordinator_suspended(fn ->
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

  test "deletion during completion retry does not recreate the job or result" do
    {id, runner, ref, _probe, _claim, _result, drop} = waiting_completion()

    while_coordinator_suspended(fn ->
      drop.()
      assert {:ok, _} = Store.delete_with_result(id)
    end)

    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert :error = Store.get(id)
    assert :error = AnalysisResultStore.get(id)
    assert Coordinator.recover_now()["recovered"] == 0
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  defp waiting_completion do
    {id, runner, ref, probe, claim, result} = completed_computation()
    drop = reject_result_update(id)
    WorkflowCompletionProbe.resume(probe)
    {_pid, token} = probe
    assert_receive {:workflow_completion, ^token, _, ^claim, ^result}, 2_000
    {id, runner, ref, probe, claim, result, drop}
  end

  defp while_coordinator_suspended(callback) do
    :ok = :sys.suspend(Coordinator)

    try do
      callback.()
    after
      :sys.resume(Coordinator)
    end
  end

  test "rejected final result commit retries the exact successful output instead of failing the job" do
    {id, runner, ref, probe, claim, result} = completed_computation()
    {:ok, before} = AnalysisResultStore.get(id)
    {:ok, job} = Store.get(id)
    drop = reject_result_update(id)
    WorkflowCompletionProbe.resume(probe)
    {_pid, token} = probe

    assert_receive {:workflow_completion, ^token, _, ^claim, ^result}, 2_000
    refute_receive {:DOWN, ^ref, :process, ^runner, _}, 300
    assert {:ok, ^before} = AnalysisResultStore.get(id)
    assert {:ok, ^job} = Store.get(id)
    assert Coordinator.recover_now()["recovered"] == 0
    refute_receive {:outage_solver_request, ^id, _}, 20
    drop.()

    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert {:ok, %{status: :completed, message: "workflow completed"}} = Store.get(id)
    assert {:ok, final} = AnalysisResultStore.get(id)
    assert Map.take(final, Map.keys(result)) == result
    assert final[@key]["state"] == "completed"
    assert final[@key]["generation"] == claim["generation"]
    assert final[@key]["attempt"] == claim["attempt"]
    Fixture.restart_coordinator()
    assert Coordinator.recover_now()["recovered"] == 0
    assert {:ok, ^final} = AnalysisResultStore.get(id)
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  test "a rejected staging write retries without claiming a durable handoff or rerunning solve" do
    {id, runner, ref, probe, claim, result} = completed_computation(:stage_result)
    {:ok, runtime} = AnalysisResultStore.get(id)
    {:ok, job} = Store.get(id)
    refute Map.has_key?(runtime[@key], "pending_completion")
    drop = reject_result_update(id)
    WorkflowCompletionProbe.resume(probe)
    {_pid, token} = probe
    assert_receive {:workflow_completion, ^token, _, ^claim, ^result}, 2_000
    assert {:ok, ^runtime} = AnalysisResultStore.get(id)
    assert {:ok, ^job} = Store.get(id)
    assert Process.alive?(runner)
    drop.()
    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert {:ok, %{status: :completed}} = Store.get(id)
    refute_receive {:outage_solver_request, ^id, _}, 50
  end

  defp completed_computation(probe_phase \\ :commit_result) do
    assert {:ok, submitted} = Workflow.submit("idempotent")
    id = submitted["job"]["job_id"]
    assert_receive {:outage_solver_request, ^id, :controlled}, 2_000
    assert_receive {:outage_solver_waiting, ^id, solver}, 2_000
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    probe = {_pid, token} = WorkflowCompletionProbe.install(id, probe_phase)
    send(solver, {:reply, {:ok, %{"solved" => true, "value" => 1.234567890123456e-12}}})
    assert_receive {:workflow_completion, ^token, _, claim, result}, 2_000
    assert result["artifacts"]["output.result"]["solved"]
    {id, runner, ref, probe, claim, result}
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
