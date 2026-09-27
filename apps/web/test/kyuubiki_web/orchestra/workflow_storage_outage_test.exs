defmodule KyuubikiWeb.Orchestra.WorkflowStorageOutageTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseMemoryBackend, LeaseStore, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.StorageOutageWorkflow, as: Workflow
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Outage.setup()
  end

  test "a failed recovery read preserves the coordinator and reports unknown work" do
    pid = Process.whereis(WorkflowRecoveryCoordinator)
    fixture = Fixture.claimed_job()
    before = WorkflowRecoveryCoordinator.snapshot()

    Outage.without(Outage.store_child(), fn ->
      assert %{
               "status" => "unavailable",
               "reason" => "analysis_store_unavailable",
               "active_jobs" => nil,
               "recovered" => 0,
               "blocked" => 0
             } = WorkflowRecoveryCoordinator.recover_now()

      assert Process.whereis(WorkflowRecoveryCoordinator) == pid
      snapshot = WorkflowRecoveryCoordinator.snapshot()
      assert snapshot["recovery_runs"] == before["recovery_runs"]
      assert snapshot["lease"]["status"] == "standby"
      assert snapshot["lease"]["last_error"] == "analysis_store_unavailable"
    end)

    Fixture.unchanged(fixture)
  end

  for message <- [:recover, :recover_job] do
    test "#{message} handles unavailable storage and retries after restoration" do
      fixture = exhausted_job()
      pid = Process.whereis(WorkflowRecoveryCoordinator)

      Outage.without(Outage.store_child(), fn ->
        message =
          if unquote(message) == :recover,
            do: :recover,
            else: {:recover_job, fixture.id, :runner_loss}

        send(pid, message)
        assert %{"lease" => %{"status" => "standby"}} = WorkflowRecoveryCoordinator.snapshot()
        Process.sleep(250)
        assert Process.whereis(WorkflowRecoveryCoordinator) == pid
      end)

      Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(fixture.id)) end)
      assert Process.whereis(WorkflowRecoveryCoordinator) == pid
      assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
      assert runtime[WorkflowRecoveryEnvelope.internal_key()]["state"] == "recovery_blocked"
      assert WorkflowRecoveryCoordinator.snapshot()["blocked_jobs"] == 1
    end
  end

  test "dispatch storage errors do not terminate the coordinator" do
    fixture = Fixture.claimed_job()
    pid = Process.whereis(WorkflowRecoveryCoordinator)

    Outage.without(Outage.store_child(), fn ->
      assert {:error, :analysis_store_unavailable} =
               WorkflowRecoveryCoordinator.dispatch(fixture.id)

      assert Process.whereis(WorkflowRecoveryCoordinator) == pid
    end)

    Fixture.unchanged(fixture)
  end

  test "stale lease messages cannot discard the current heartbeat or retry timer" do
    pid = Process.whereis(WorkflowRecoveryCoordinator)
    before = :sys.get_state(pid)
    send(pid, :acquire_lease)
    send(pid, {:renew_lease, -1, -1})
    assert :sys.get_state(pid).lease_timer_ref == before.lease_timer_ref

    Outage.without(Outage.store_child(), fn ->
      assert %{"status" => "unavailable"} = WorkflowRecoveryCoordinator.recover_now()
      standby = :sys.get_state(pid)
      send(pid, {:renew_lease, before.lease.fencing_token, before.lease.expires_at_ms})
      assert :sys.get_state(pid).lease_timer_ref == standby.lease_timer_ref
    end)
  end

  test "lease-store outage rejects ownership writes and leaves the coordinator responsive" do
    pid = Process.whereis(WorkflowRecoveryCoordinator)
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, lease} = LeaseStore.current(name)
    child = if Storage.memory?(), do: LeaseMemoryBackend, else: Outage.store_child()

    Outage.without(child, fn ->
      assert {:error, :orchestra_lease_store_unavailable} = LeaseStore.current(name)

      assert {:error, :orchestra_lease_store_unavailable} =
               LeaseStore.acquire(name, "other", 1000)

      assert {:error, :orchestra_lease_store_unavailable} = LeaseStore.renew(lease, 1000)
      assert {:error, :orchestra_lease_store_unavailable} = LeaseStore.release(lease)

      assert {:error, :orchestra_lease_store_unavailable} =
               LeaseStore.with_lease(lease, fn -> flunk("unowned callback ran") end)

      send(pid, {:renew_lease, lease.fencing_token, lease.expires_at_ms})
      assert %{"lease" => %{"status" => "standby"}} = WorkflowRecoveryCoordinator.snapshot()
      assert Process.whereis(WorkflowRecoveryCoordinator) == pid
    end)

    Outage.eventually(fn ->
      WorkflowRecoveryCoordinator.snapshot()["lease"]["status"] == "owner"
    end)

    assert Process.whereis(WorkflowRecoveryCoordinator) == pid
  end

  for operation <- [:progress, :complete, :fail, :cancel, :initialize] do
    test "#{operation} during an outage leaves a retryable record and a live coordinator" do
      fixture = Fixture.claimed_job()
      pid = Process.whereis(WorkflowRecoveryCoordinator)

      Outage.without(Outage.store_child(), fn ->
        reply =
          case unquote(operation) do
            :progress ->
              WorkflowRecoveryCoordinator.record_progress(
                fixture.id,
                fixture.claim,
                Fixture.progress()
              )

            :complete ->
              WorkflowRecoveryCoordinator.commit_result(
                fixture.id,
                fixture.claim,
                Fixture.result()
              )

            :fail ->
              WorkflowRecoveryCoordinator.fail(fixture.id, fixture.claim, "failed")

            :cancel ->
              WorkflowRecoveryCoordinator.cancel(fixture.id)

            :initialize ->
              WorkflowRecoveryCoordinator.initialize(fixture.id, %{}, %{}, %{}, %{})
          end

        assert {:error, :orchestra_lease_store_unavailable} = reply
        assert Process.whereis(WorkflowRecoveryCoordinator) == pid
      end)

      Fixture.unchanged(fixture)
    end
  end

  for policy <- ["idempotent", "checkpoint_required"] do
    test "running #{policy} workflows stop safely and obey replay policy after storage returns" do
      Workflow.install(self())
      assert {:ok, submitted} = Workflow.submit(unquote(policy))
      id = submitted["job"]["job_id"]
      assert_receive {:outage_solver_request, ^id, :hold}, 2000
      assert {:ok, runner} = WorkflowJobRunner.running(id)
      ref = Process.monitor(runner)
      coordinator = Process.whereis(WorkflowRecoveryCoordinator)

      Outage.without(Outage.store_child(), fn ->
        assert %{"status" => "unavailable"} = WorkflowRecoveryCoordinator.recover_now()
        assert_receive {:DOWN, ^ref, :process, ^runner, :shutdown}, 2000
        assert WorkflowRecoveryCoordinator.snapshot()["tracked_jobs"] == []
        Workflow.configure(self(), :succeed)
      end)

      status = if unquote(policy) == "idempotent", do: :completed, else: :failed
      Outage.eventually(fn -> match?({:ok, %{status: ^status}}, Store.get(id)) end)
      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
      assert {:ok, runtime} = AnalysisResultStore.get(id)
      recovery = runtime[WorkflowRecoveryEnvelope.internal_key()]

      if unquote(policy) == "idempotent" do
        assert_receive {:outage_solver_request, ^id, :succeed}, 2000
        assert recovery["generation"] == 2
        assert recovery["attempt"] == 2
        assert runtime["artifacts"]["output.result"]["recovered"]
      else
        refute_receive {:outage_solver_request, ^id, :succeed}, 100
        assert recovery["state"] == "recovery_blocked"
        assert recovery["generation"] == 1
        assert recovery["attempt"] == 1
        assert is_map(recovery["envelope"])
      end
    end
  end

  defp exhausted_job do
    fixture = Fixture.claimed_job()
    key = WorkflowRecoveryEnvelope.internal_key()
    runtime = put_in(fixture.runtime, [key, "attempt"], 3)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)
    %{fixture | runtime: runtime}
  end
end
