defmodule KyuubikiWeb.Orchestra.WorkflowRegistryOutageTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner, as: Runner
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryRunners, as: Runners
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.StorageOutageWorkflow, as: Workflow
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry

  setup do
    Outage.setup()
  end

  test "registry absence is unavailable, not an absent runner or an exception" do
    coordinator = Process.whereis(Coordinator)

    Outage.without(@registry, fn ->
      assert {:error, :workflow_runner_registry_unavailable} = Runner.running("not-dispatched")
      assert Process.whereis(Coordinator) == coordinator
    end)

    assert :error = Runner.running("not-dispatched")
  end

  test "dispatch and cancellation delivery report unavailability without consuming a claim" do
    f = Fixture.claimed_job()
    coordinator = Process.whereis(Coordinator)

    Outage.without(@registry, fn ->
      assert {:error, :workflow_runner_registry_unavailable} = Coordinator.dispatch(f.id)
      assert {:error, :workflow_runner_registry_unavailable} = Delivery.kick([f.id])
      Fixture.unchanged(f)
      assert Process.whereis(Coordinator) == coordinator
    end)
  end

  for policy <- ["idempotent", "checkpoint_required"] do
    test "running #{policy} work obeys replay policy after registry loss without coordinator restart" do
      Workflow.install(self())
      Workflow.configure(self(), :controlled)
      assert {:ok, submitted} = Workflow.submit(unquote(policy))
      id = submitted["job"]["job_id"]
      assert_receive {:outage_solver_request, ^id, :controlled}, 2_000
      assert_receive {:outage_solver_waiting, ^id, solver}, 2_000
      assert {:ok, runner} = Runner.running(id)
      runner_ref = Process.monitor(runner)
      solver_ref = Process.monitor(solver)
      coordinator = Process.whereis(Coordinator)
      supervisor = Process.whereis(KyuubikiWeb.Supervisor)
      tasks = Process.whereis(KyuubikiWeb.TaskSupervisor)

      Outage.without(@registry, fn ->
        assert_receive {:DOWN, ^runner_ref, :process, ^runner, _}, 2_000
        assert_receive {:DOWN, ^solver_ref, :process, ^solver, _}, 2_000
        assert_unavailable()
        {:ok, job} = Store.get(id)
        {:ok, runtime} = AnalysisResultStore.get(id)
        assert runtime[Envelope.internal_key()]["generation"] == 1
        Process.sleep(250)
        assert Process.whereis(Coordinator) == coordinator
        assert Process.whereis(KyuubikiWeb.Supervisor) == supervisor
        assert Process.whereis(KyuubikiWeb.TaskSupervisor) == tasks
        assert {:ok, ^job} = Store.get(id)
        assert {:ok, ^runtime} = AnalysisResultStore.get(id)
        refute_receive {:outage_solver_request, ^id, _}, 20
        Workflow.configure(self(), :succeed)
      end)

      status = if unquote(policy) == "idempotent", do: :completed, else: :failed
      Outage.eventually(fn -> match?({:ok, %{status: ^status}}, Store.get(id)) end)
      assert {:ok, final} = AnalysisResultStore.get(id)

      if unquote(policy) == "idempotent" do
        assert_receive {:outage_solver_request, ^id, :succeed}, 2_000
        assert final[Envelope.internal_key()]["generation"] == 2
      else
        assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
        assert final[Envelope.internal_key()]["generation"] == 1
      end

      refute_receive {:outage_solver_request, ^id, _}, 50
      assert Process.whereis(Coordinator) == coordinator
      assert Coordinator.snapshot()["runner_registry_retry_pending"] == false
    end
  end

  test "staged completion publishes automatically when registry returns" do
    f = staged()
    coordinator = Process.whereis(Coordinator)

    Outage.without(@registry, fn ->
      assert_unavailable()
      Fixture.unchanged(f)
    end)

    assert_completed(f)
    assert Process.whereis(Coordinator) == coordinator
  end

  test "coordinator can restart while registry remains unavailable" do
    f = staged()

    coordinator =
      Outage.without(@registry, fn ->
        Fixture.restart_coordinator()
        coordinator = Process.whereis(Coordinator)
        assert Coordinator.snapshot()["runner_registry_retry_pending"]
        Process.sleep(250)
        assert Process.whereis(Coordinator) == coordinator
        Fixture.unchanged(f)
        coordinator
      end)

    assert_completed(f)
    assert Process.whereis(Coordinator) == coordinator
  end

  test "repeated scans deduplicate registry retries and stale messages cannot consume a replacement" do
    f = staged()
    manual_clock()

    retry =
      Outage.without(@registry, fn ->
        assert_unavailable()
        first = registry_retry()
        for _ <- 1..5, do: assert_unavailable()
        assert registry_retry() == first
        fire_registry(first)
        replacement = registry_retry()
        assert replacement.token != first.token
        assert Process.read_timer(first.timer) == false
        fire_registry(first)
        assert registry_retry() == replacement
        Fixture.unchanged(f)
        replacement
      end)

    fire_registry(retry)
    assert_completed(f)
    assert Process.read_timer(retry.timer) == false
  end

  test "publication-only retry intent survives a registry outage" do
    f = staged()
    manual_clock()

    retry =
      Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
        assert %{"skipped" => 1} = Coordinator.recover_now()
        completion = :sys.get_state(Coordinator).completion_retries[f.id]

        Outage.without(@registry, fn ->
          send(Coordinator, {:retry_completion, f.id, completion.token})
          assert Coordinator.snapshot()["runner_registry_retry_pending"]
          assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]

          changed =
            Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))

          assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, changed)
          registry_retry()
        end)
      end)

    fire_registry(retry)
    assert {:ok, %{status: :failed, message: message}} = Store.get(f.id)
    assert message =~ "workflow_completion_checkpoint_invalid"
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
    assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
    assert Coordinator.snapshot()["pending_completion_retries"] == []
  end

  for policy <- ["idempotent", "checkpoint_required"], evidence <- [:valid, :missing] do
    @tag capture_log: true
    test "finalizer startup race preserves #{policy} publication with #{evidence} evidence" do
      graph = %{
        "id" => "registry-startup",
        "nodes" => [],
        "recovery_policy" => %{"retry_safety" => unquote(policy)}
      }

      f = staged(graph: graph)
      coordinator = Process.whereis(Coordinator)
      supervisor = Process.whereis(KyuubikiWeb.TaskSupervisor)
      :ok = :sys.suspend(supervisor)

      try do
        scan = Task.async(fn -> Coordinator.recover_now() end)
        await_start_request(supervisor, coordinator)

        Outage.without(@registry, fn ->
          if unquote(evidence) == :missing do
            changed =
              Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))

            assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, changed)
          end

          :ok = :sys.resume(supervisor)
          assert %{"recovered" => 1} = Task.await(scan)
          Outage.eventually(fn -> Coordinator.snapshot()["runner_registry_retry_pending"] end)
          assert Process.whereis(Coordinator) == coordinator
        end)
      after
        :sys.resume(supervisor)
      end

      Outage.eventually(fn ->
        match?({:ok, %{status: status}} when status in [:completed, :failed], Store.get(f.id))
      end)

      if unquote(evidence) == :valid do
        assert_completed(f)
      else
        assert {:ok, %{status: :failed, message: message}} = Store.get(f.id)
        assert message =~ "workflow_completion_checkpoint_invalid"
        assert {:ok, final} = AnalysisResultStore.get(f.id)
        assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
        assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
        assert final["artifacts"] == %{}
      end
    end
  end

  @tag capture_log: true
  test "repeated scans retain a tracked live finalizer before registry registration" do
    f = staged()
    pid = idle_runner()

    :sys.replace_state(Coordinator, &Runners.track(f.id, pid, &1, :finalizing))
    runner = :sys.get_state(Coordinator).jobs[f.id]
    assert :error = Runner.running(f.id)

    for _ <- 1..3 do
      assert %{"recovered" => 0, "skipped" => 1} = Coordinator.recover_now()
      assert :sys.get_state(Coordinator).jobs[f.id] == runner
    end

    Fixture.unchanged(f)
    send(pid, :finish)
    assert_completed(f)
  end

  test "replacing a runner monitor keeps publication intent and ignores the old DOWN" do
    f = staged()
    old = idle_runner()
    replacement = idle_runner()
    :sys.replace_state(Coordinator, &Runners.track(f.id, old, &1, :finalizing))
    previous = :sys.get_state(Coordinator).jobs[f.id]
    :sys.replace_state(Coordinator, &Runners.track(f.id, replacement, &1))
    current = :sys.get_state(Coordinator).jobs[f.id]
    assert current.pid == replacement
    assert current.ref != previous.ref
    assert current.purpose == :finalizing
    refute Map.has_key?(:sys.get_state(Coordinator).refs, previous.ref)
    send(Coordinator, {:DOWN, previous.ref, :process, old, :killed})
    assert f.id in Coordinator.snapshot()["tracked_jobs"]
    assert :sys.get_state(Coordinator).jobs[f.id] == current

    changed = Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, changed)
    send(replacement, :finish)
    Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(f.id)) end)
    assert {:ok, %{message: message}} = Store.get(f.id)
    assert message =~ "workflow_completion_checkpoint_invalid"
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
  end

  test "accepted staging changes the executing runner to publication-only recovery" do
    f = Fixture.claimed_job()
    pid = idle_runner()
    :sys.replace_state(Coordinator, &Runners.track(f.id, pid, &1, :executing))
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert :sys.get_state(Coordinator).jobs[f.id].purpose == :finalizing
    assert {:ok, staged} = AnalysisResultStore.get(f.id)
    changed = Map.update!(staged, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, staged, changed)
    send(pid, :finish)
    Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(f.id)) end)
    assert {:ok, %{message: message}} = Store.get(f.id)
    assert message =~ "workflow_completion_checkpoint_invalid"
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
  end

  test "rejected staging does not change an executing runner's recovery purpose" do
    f = Fixture.claimed_job()
    pid = idle_runner()
    :sys.replace_state(Coordinator, &Runners.track(f.id, pid, &1, :executing))
    stale = Map.update!(f.claim, "generation", &(&1 + 1))

    assert {:error, :stale_workflow_execution_claim} =
             Coordinator.stage_result(f.id, stale, Fixture.result())

    assert :sys.get_state(Coordinator).jobs[f.id].purpose == :executing
    Fixture.unchanged(f)
    assert :ok = Coordinator.cancel(f.id)
    send(pid, :finish)
    Outage.eventually(fn -> Coordinator.snapshot()["tracked_jobs"] == [] end)
    assert Coordinator.snapshot()["pending_completion_retries"] == []
  end

  for action <- [:cancel, :delete, :watchdog] do
    test "accepted #{action} retires a tracked finalizer without rescheduling publication" do
      f = staged()
      pid = idle_runner()
      :sys.replace_state(Coordinator, &Runners.track(f.id, pid, &1, :finalizing))

      case unquote(action) do
        :cancel -> assert :ok = Coordinator.cancel(f.id)
        :delete -> assert {:ok, _} = Coordinator.delete(f.id)
        :watchdog -> assert {:ok, _} = Coordinator.fail_observed(f.job, "watchdog timeout")
      end

      expected = {Store.get(f.id), AnalysisResultStore.get(f.id)}
      send(pid, :finish)
      Outage.eventually(fn -> Coordinator.snapshot()["tracked_jobs"] == [] end)
      assert Coordinator.snapshot()["pending_completion_retries"] == []
      assert {Store.get(f.id), AnalysisResultStore.get(f.id)} == expected
    end
  end

  for action <- [:cancel, :delete] do
    test "#{action} remains authoritative through registry restoration" do
      f = staged()
      manual_clock()

      {retry, expected} =
        Outage.without(@registry, fn ->
          assert_unavailable()

          case unquote(action) do
            :cancel -> assert :ok = Coordinator.cancel(f.id)
            :delete -> assert {:ok, _} = Coordinator.delete(f.id)
          end

          {registry_retry(), {Store.get(f.id), AnalysisResultStore.get(f.id)}}
        end)

      fire_registry(retry)
      assert {Store.get(f.id), AnalysisResultStore.get(f.id)} == expected
      assert Coordinator.snapshot()["runner_registry_retry_pending"] == false
    end
  end

  test "lease takeover cancels the deferred registry scan and ignores its late timer" do
    f = staged()
    manual_clock()

    Outage.without(@registry, fn ->
      assert_unavailable()
      retry = registry_retry()
      name = Coordinator.snapshot()["lease"]["lease_name"]
      assert {:ok, lease} = LeaseStore.current(name)
      assert :ok = LeaseStore.release(lease)
      assert {:ok, takeover} = LeaseStore.acquire(name, "registry-replacement", 60_000)
      on_exit(fn -> LeaseStore.release(takeover) end)
      send(Coordinator, {:renew_lease, lease.fencing_token, lease.expires_at_ms})
      assert Coordinator.snapshot()["lease"]["status"] == "standby"
      assert Coordinator.snapshot()["runner_registry_retry_pending"] == false
      assert Process.read_timer(retry.timer) == false
      fire_registry(retry)
      Fixture.unchanged(f)
    end)

    Fixture.unchanged(f)
  end

  test "watchdog retirement survives an unavailable registry without crashing the coordinator" do
    f = Fixture.claimed_job()
    coordinator = Process.whereis(Coordinator)

    Outage.without(@registry, fn ->
      assert {:ok, %{status: :failed}} = Coordinator.fail_observed(f.job, "watchdog timeout")
      assert Process.whereis(Coordinator) == coordinator
      assert {:ok, final} = AnalysisResultStore.get(f.id)
      assert final[Envelope.internal_key()]["state"] == "failed"
    end)
  end

  defp assert_unavailable do
    assert %{
             "status" => "unavailable",
             "reason" => "workflow_runner_registry_unavailable",
             "active_jobs" => nil,
             "recovered" => 0,
             "blocked" => 0
           } = Coordinator.recover_now()
  end

  defp manual_clock, do: :sys.replace_state(Coordinator, &%{&1 | lease_retry_ms: 10_000})
  defp registry_retry, do: :sys.get_state(Coordinator).registry_retry

  defp await_start_request(supervisor, coordinator) do
    Outage.eventually(fn ->
      {:messages, messages} = Process.info(supervisor, :messages)

      Enum.any?(messages, fn
        {:"$gen_call", {^coordinator, _}, _request} -> true
        _ -> false
      end)
    end)
  end

  defp fire_registry(retry) do
    send(Coordinator, {:retry_runner_registry, retry.token})
    Coordinator.snapshot()
  end

  defp idle_runner do
    pid = spawn(fn -> receive do: (:finish -> :ok) end)
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    pid
  end

  defp staged(opts \\ []) do
    f = Fixture.claimed_job(opts)
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert {:ok, job} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    %{f | job: job, runtime: runtime}
  end

  defp assert_completed(f) do
    Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["artifacts"] == Fixture.result()["artifacts"]
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
    assert final[Envelope.internal_key()]["attempt"] == f.claim["attempt"]
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
    assert Coordinator.snapshot()["runner_registry_retry_pending"] == false
  end
end
