defmodule KyuubikiWeb.Orchestra.WorkflowCompletionSupervisorRecoveryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault

  setup do
    Outage.setup()
  end

  test "a staged result publishes automatically when the runner supervisor returns" do
    f = staged_completion()
    coordinator = Process.whereis(Coordinator)

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      assert %{"recovered" => 0, "blocked" => 0, "skipped" => 1} = Coordinator.recover_now()
      Fixture.unchanged(f)
      assert Process.whereis(Coordinator) == coordinator
    end)

    Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["artifacts"] == Fixture.result()["artifacts"]
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
    assert final[Envelope.internal_key()]["attempt"] == f.claim["attempt"]
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
    assert Process.whereis(Coordinator) == coordinator
  end

  test "coordinator restart rebuilds the pending retry from durable evidence" do
    f = staged_completion()

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      Fixture.restart_coordinator()
      assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]
      Fixture.unchanged(f)
    end)

    assert_completed(f)
  end

  test "repeated scans retain one timer and stale timer messages cannot consume its replacement" do
    f = staged_completion()
    use_manual_retry_clock()

    retry =
      Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
        first = schedule(f)
        for _ <- 1..5, do: assert(schedule(f) == first)
        fire(f, first)
        replacement = pending_retry(f)
        assert replacement.token != first.token
        assert Process.read_timer(first.timer) == false
        fire(f, first)
        assert pending_retry(f) == replacement
        Fixture.unchanged(f)
        replacement
      end)

    fire(f, retry)
    assert_completed(f)
    assert Process.read_timer(retry.timer) == false
  end

  for action <- [:cancel, :delete] do
    test "#{action} revokes a pending finalizer and ignores its late timer" do
      f = staged_completion()
      use_manual_retry_clock()

      Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
        retry = schedule(f)

        case unquote(action) do
          :cancel -> assert :ok = Coordinator.cancel(f.id)
          :delete -> assert {:ok, _} = Coordinator.delete(f.id)
        end

        expected = {Store.get(f.id), AnalysisResultStore.get(f.id)}
        assert Coordinator.snapshot()["pending_completion_retries"] == []
        assert Process.read_timer(retry.timer) == false
        fire(f, retry)
        assert {Store.get(f.id), AnalysisResultStore.get(f.id)} == expected
      end)

      if unquote(action) == :cancel,
        do: assert(match?({:ok, %{status: :cancelled}}, Store.get(f.id))),
        else: assert(Store.get(f.id) == :error)
    end
  end

  for damage <- [:digest, :claim] do
    test "retry revalidates a changed #{damage} instead of publishing cached output" do
      f = staged_completion()
      use_manual_retry_clock()

      retry =
        Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
          retry = schedule(f)

          path =
            if unquote(damage) == :digest,
              do: [Envelope.internal_key(), Checkpoint.key(), "sha256"],
              else: [Envelope.internal_key(), "owner_session_id"]

          damaged = put_in(f.runtime, path, "changed-after-scan")
          assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, damaged)
          retry
        end)

      fire(f, retry)
      assert {:ok, %{status: :failed, message: message}} = Store.get(f.id)
      assert message =~ "workflow_completion_checkpoint_invalid"
      assert {:ok, final} = AnalysisResultStore.get(f.id)
      assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
      assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
      assert Coordinator.snapshot()["pending_completion_retries"] == []
    end
  end

  test "a lease takeover clears retries without publishing or modifying the old result" do
    f = staged_completion()
    use_manual_retry_clock()

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retry = schedule(f)
      name = Coordinator.snapshot()["lease"]["lease_name"]
      assert {:ok, lease} = LeaseStore.current(name)
      assert :ok = LeaseStore.release(lease)
      assert {:ok, takeover} = LeaseStore.acquire(name, "replacement-finalizer", 60_000)
      on_exit(fn -> LeaseStore.release(takeover) end)
      send(Coordinator, {:renew_lease, lease.fencing_token, lease.expires_at_ms})
      assert Coordinator.snapshot()["lease"]["status"] == "standby"
      assert Coordinator.snapshot()["pending_completion_retries"] == []
      assert Process.read_timer(retry.timer) == false
      fire(f, retry)
      Fixture.unchanged(f)
    end)

    Fixture.unchanged(f)
  end

  for safety <- ["idempotent", "checkpoint_required"], trigger <- [:timer, :scan] do
    @tag :publication_only
    test "#{trigger} cannot replay #{safety} work after the pending completion disappears" do
      f = staged_completion(unquote(safety))
      use_manual_retry_clock()
      coordinator = Process.whereis(Coordinator)

      retry =
        Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
          retry = schedule(f)

          changed =
            Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))

          assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, changed)
          retry
        end)

      case unquote(trigger) do
        :timer -> fire(f, retry)
        :scan -> assert %{"recovered" => 0, "blocked" => 1} = Coordinator.recover_now()
      end

      assert Process.whereis(Coordinator) == coordinator
      assert {:ok, %{status: :failed, message: message}} = Store.get(f.id)
      assert message =~ "workflow_completion_checkpoint_invalid"
      assert {:ok, final} = AnalysisResultStore.get(f.id)
      assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
      assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
      assert final[Envelope.internal_key()]["attempt"] == f.claim["attempt"]
      assert final["artifacts"] == %{}
      assert Coordinator.snapshot()["pending_completion_retries"] == []
      assert Process.read_timer(retry.timer) == false
      fire(f, retry)
      assert {:ok, ^final} = AnalysisResultStore.get(f.id)
    end
  end

  test "an already committed result cannot be republished by a pending timer" do
    f = staged_completion()
    use_manual_retry_clock()

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retry = schedule(f)
      assert :ok = Coordinator.commit_result(f.id, f.claim, Fixture.result())
      expected = {Store.get(f.id), AnalysisResultStore.get(f.id)}
      assert Coordinator.snapshot()["pending_completion_retries"] == []
      assert Process.read_timer(retry.timer) == false
      fire(f, retry)
      assert {Store.get(f.id), AnalysisResultStore.get(f.id)} == expected
    end)
  end

  for damage <- [:missing, :digest], trigger <- [:timer, :scan] do
    @tag :publication_only
    @tag skip: Storage.postgres?()
    test "#{trigger} retains #{damage} checkpoint rejection when its terminal write fails" do
      f = staged_completion("idempotent")
      use_manual_retry_clock()
      coordinator = Process.whereis(Coordinator)

      {retry, changed} =
        Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
          retry = schedule(f)

          changed =
            if unquote(damage) == :missing,
              do:
                Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key())),
              else:
                put_in(f.runtime, [Envelope.internal_key(), Checkpoint.key(), "sha256"], "bad")

          assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, changed)
          {retry, changed}
        end)

      drop = reject_result_update(f.id)

      case unquote(trigger) do
        :timer ->
          fire(f, retry)

        :scan ->
          assert %{"recovered" => 0, "blocked" => 0, "skipped" => 1} = Coordinator.recover_now()
      end

      Fixture.unchanged(%{f | runtime: changed})
      replacement = pending_retry(f)
      assert is_integer(Process.read_timer(replacement.timer))
      assert Process.whereis(Coordinator) == coordinator
      assert Coordinator.snapshot()["lease"]["status"] == "owner"
      drop.()

      fire(f, replacement)
      assert {:ok, %{status: :failed, message: message}} = Store.get(f.id)
      assert message =~ "workflow_completion_checkpoint_invalid"
      assert {:ok, final} = AnalysisResultStore.get(f.id)
      assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
      assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
      assert final[Envelope.internal_key()]["attempt"] == f.claim["attempt"]
      assert final["artifacts"] == %{}
      assert Coordinator.snapshot()["pending_completion_retries"] == []
      assert Process.whereis(Coordinator) == coordinator
    end
  end

  for rejection <- [:conflicting_result, :stale_failure] do
    test "a rejected #{rejection} cannot revoke the valid pending recovery" do
      f = staged_completion()
      use_manual_retry_clock()

      retry =
        Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
          retry = schedule(f)

          case unquote(rejection) do
            :conflicting_result ->
              assert {:error, :workflow_completion_checkpoint_conflict} =
                       Coordinator.commit_result(f.id, f.claim, %{"artifacts" => %{}})

            :stale_failure ->
              stale = Map.put(f.claim, "generation", f.claim["generation"] + 1)

              assert {:error, :stale_workflow_execution_claim} =
                       Coordinator.fail(f.id, stale, "late failure")
          end

          assert pending_retry(f) == retry
          Fixture.unchanged(f)
          retry
        end)

      fire(f, retry)
      assert_completed(f)
    end
  end

  @tag :publication_only
  test "a missing completion does not prevent another staged job from publishing" do
    broken = staged_completion("idempotent")
    use_manual_retry_clock()

    healthy =
      Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
        schedule(broken)

        changed =
          Map.update!(broken.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))

        assert :ok = AnalysisResultStore.compare_and_swap(broken.id, broken.runtime, changed)
        staged_completion()
      end)

    assert %{"recovered" => 1, "blocked" => 1, "skipped" => 0} = Coordinator.recover_now()
    assert_completed(healthy)
    assert {:ok, %{status: :failed}} = Store.get(broken.id)
    assert {:ok, final} = AnalysisResultStore.get(broken.id)
    assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
    assert final[Envelope.internal_key()]["generation"] == broken.claim["generation"]
  end

  @tag :publication_only
  @tag skip: Storage.postgres?()
  test "rejection automatically completes after persistence becomes writable again" do
    f = staged_completion("idempotent")
    coordinator = Process.whereis(Coordinator)

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retry = schedule(f)
      changed = Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))
      assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, changed)
      drop = reject_result_update(f.id)
      fire(f, retry)
      Fixture.unchanged(%{f | runtime: changed})
      assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]
      drop.()
    end)

    Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
    assert final["artifacts"] == %{}
    assert Coordinator.snapshot()["pending_completion_retries"] == []
    assert Process.whereis(Coordinator) == coordinator
  end

  test "storage outage during retry releases ownership and recovers after both services return" do
    f = staged_completion()
    coordinator = Process.whereis(Coordinator)

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retry = schedule(f)

      Outage.without(Outage.store_child(), fn ->
        fire(f, retry)
        assert Coordinator.snapshot()["lease"]["status"] == "standby"
        assert Coordinator.snapshot()["pending_completion_retries"] == []
        assert Process.whereis(Coordinator) == coordinator
      end)
    end)

    assert_completed(f)
  end

  defp schedule(f) do
    assert %{"recovered" => 0, "blocked" => 0, "skipped" => 1} = Coordinator.recover_now()
    assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]
    pending_retry(f)
  end

  defp pending_retry(f), do: Map.fetch!(:sys.get_state(Coordinator).completion_retries, f.id)

  defp fire(f, retry) do
    send(Coordinator, {:retry_completion, f.id, retry.token})
    Coordinator.snapshot()
  end

  defp use_manual_retry_clock do
    :sys.replace_state(Coordinator, &%{&1 | lease_retry_ms: 10_000})
  end

  defp assert_completed(f) do
    Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["artifacts"] == Fixture.result()["artifacts"]
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
    assert final[Envelope.internal_key()]["attempt"] == f.claim["attempt"]
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
    assert Coordinator.snapshot()["pending_completion_retries"] == []
  end

  defp staged_completion(safety \\ "checkpoint_required") do
    f =
      Fixture.claimed_job(
        graph: %{
          "id" => "publication-only",
          "nodes" => [],
          "recovery_policy" => %{"retry_safety" => safety}
        }
      )

    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert {:ok, job} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    %{f | job: job, runtime: runtime}
  end

  defp reject_result_update(id) do
    if Storage.sqlite?() do
      AnalysisCommitFault.reject_result_update(id)
    else
      path = Persistence.analysis_state_path() <> ".next"
      File.mkdir_p!(path)
      File.write!(Path.join(path, "blocker"), "injected finalizer rejection write failure")
      cleanup = fn -> File.rm_rf!(path) end
      on_exit(cleanup)
      cleanup
    end
  end
end
