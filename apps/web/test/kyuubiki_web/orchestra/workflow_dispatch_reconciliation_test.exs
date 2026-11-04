defmodule KyuubikiWeb.Orchestra.WorkflowDispatchReconciliationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, ResultReadProbe}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @reason "workflow_recovery_dispatch_incomplete"

  setup do
    Outage.setup()
  end

  for defect <- [:attempts, :digest, :replay] do
    test "unchanged #{defect} refusal commits one blocked job and leaves later scans empty" do
      healthy = terminal_job()
      fixture = rejected_job(unquote(defect))

      assert %{"status" => "completed", "blocked" => 1, "active_jobs" => 2} =
               Coordinator.recover_now()

      assert_blocked(fixture)
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
      assert :error = WorkflowJobRunner.running(fixture.id)
      assert %{"active_jobs" => 0, "blocked" => 0} = Coordinator.recover_now()
    end

    for replacement <- [:active, :terminal, :annotate, :plain, :delete_result] do
      @tag skip: not Storage.sqlite?()
      test "#{defect} refusal cannot be rebound to #{replacement} after the claim read" do
        healthy = terminal_job()
        fixture = rejected_job(unquote(defect))
        before = Coordinator.snapshot()
        {task, coordinator, probe} = pause_scan(fixture.id)
        replace(fixture, unquote(replacement))
        retained = snapshot(fixture.id)
        resume(coordinator, probe)
        summary = Task.await(task, 2_000)

        assert snapshot(fixture.id) == retained
        assert %{"status" => "interrupted", "reason" => @reason, "blocked" => 0} = summary
        assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
        assert Process.whereis(Coordinator) == coordinator
        assert {:ok, %{status: :completed}} = Store.get(healthy.id)
        assert :error = WorkflowJobRunner.running(fixture.id)

        if unquote(replacement) in [:terminal, :annotate] do
          assert %{"status" => "completed", "active_jobs" => 1} = Coordinator.recover_now()
          expected = if unquote(replacement) == :terminal, do: :completed, else: :failed
          assert {:ok, %{status: ^expected}} = Store.get(fixture.id)
        end
      end
    end
  end

  for replacement <- [:active, :terminal] do
    @tag skip: not Storage.sqlite?()
    test "initial refusal cannot consume a newer #{replacement} execution snapshot" do
      fixture = rejected_job(:digest)
      key = Envelope.internal_key()

      runtime =
        update_in(fixture.runtime, [key], fn recovery ->
          Map.merge(recovery, %{"state" => "pending", "attempt" => 0, "generation" => 0})
        end)

      assert :ok = AnalysisResultStore.put(fixture.id, runtime)
      fixture = %{fixture | runtime: runtime}
      {task, coordinator, probe} = pause_scan(fixture.id, :initial)
      replace(fixture, unquote(replacement))
      retained = snapshot(fixture.id)
      resume(coordinator, probe)
      assert {:error, _} = Task.await(task, 2_000)
      assert snapshot(fixture.id) == retained
      assert :error = WorkflowJobRunner.running(fixture.id)
    end
  end

  for winner <- [:completed, :failed, :cancelled, :deleted] do
    @tag skip: not Storage.sqlite?()
    test "dispatch refusal preserves an independent #{winner} job without counting it blocked" do
      healthy = terminal_job()
      fixture = rejected_job(:attempts)
      {task, coordinator, probe} = pause_scan(fixture.id)

      if unquote(winner) == :deleted do
        assert {:ok, _} = Store.delete(fixture.id)
      else
        assert {:ok, _} =
                 Store.apply_progress(%{
                   job_id: fixture.id,
                   stage: unquote(winner),
                   progress: 1.0,
                   message: "independent terminal decision"
                 })
      end

      retained = snapshot(fixture.id)
      resume(coordinator, probe)
      summary = Task.await(task, 2_000)
      assert snapshot(fixture.id) == retained
      assert summary["blocked"] == 0
      expected = if unquote(winner) == :deleted, do: "interrupted", else: "completed"
      assert summary["status"] == expected
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
    end
  end

  for target <- [:job, :result] do
    @tag skip: not Storage.sqlite?()
    test "rejected #{target} blocking write interrupts the scan and leaves a fresh retry usable" do
      healthy = terminal_job()
      fixture = rejected_job(:attempts)
      before = Coordinator.snapshot()

      drop =
        if unquote(target) == :job,
          do: AnalysisCommitFault.reject_job_update(fixture.id),
          else: AnalysisCommitFault.reject_result_update(fixture.id)

      assert %{"status" => "interrupted", "reason" => @reason, "blocked" => 0} =
               Coordinator.recover_now()

      Fixture.unchanged(fixture)
      assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
      drop.()
      assert %{"status" => "completed", "blocked" => 1} = Coordinator.recover_now()
      assert_blocked(fixture)
    end
  end

  @tag skip: not Storage.memory?()
  test "a rejected JSON refusal write retains bytes and cannot complete a recovery scan" do
    fixture = rejected_job(:digest)
    before = Coordinator.snapshot()
    path = Persistence.analysis_state_path()
    bytes = File.read!(path)
    pending = path <> ".next"
    assert :ok = File.mkdir(pending)
    File.write!(Path.join(pending, "blocker"), "injected fault")
    on_exit(fn -> File.rm_rf!(pending) end)

    assert %{"status" => "interrupted", "reason" => @reason, "blocked" => 0} =
             Coordinator.recover_now()

    Fixture.unchanged(fixture)
    assert File.read!(path) == bytes
    assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
    File.rm_rf!(pending)
    assert %{"status" => "completed", "blocked" => 1} = Coordinator.recover_now()
    assert_blocked(fixture)
  end

  @tag skip: not Storage.sqlite?()
  test "lease replacement after the claim read prevents refusal publication" do
    healthy = terminal_job()
    fixture = rejected_job(:attempts)
    before = Coordinator.snapshot()
    name = before["lease"]["lease_name"]
    assert {:ok, lease} = LeaseStore.current(name)
    {task, coordinator, probe} = pause_scan(fixture.id)
    assert :ok = LeaseStore.release(lease)
    assert {:ok, other} = LeaseStore.acquire(name, "dispatch-replacement-owner", 60_000)
    on_exit(fn -> LeaseStore.release(other) end)
    resume(coordinator, probe)

    assert %{"status" => "interrupted", "reason" => "orchestra_lease_lost", "blocked" => 0} =
             Task.await(task, 2_000)

    Fixture.unchanged(fixture)
    Fixture.unchanged(healthy)
    assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
  end

  defp rejected_job(defect) do
    fixture = Fixture.claimed_job()
    key = Envelope.internal_key()

    runtime =
      case defect do
        :attempts -> put_in(fixture.runtime, [key, "attempt"], 3)
        :digest -> put_in(fixture.runtime, [key, "envelope_sha256"], String.duplicate("0", 64))
        :replay -> replay_required(fixture)
      end

    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    Map.put(%{fixture | runtime: runtime}, :valid_runtime, fixture.runtime)
  end

  defp replay_required(fixture) do
    key = Envelope.internal_key()
    graph = fixture.runtime[key]["envelope"]["graph"]
    graph = put_in(graph, ["recovery_policy", "retry_safety"], "checkpoint_required")
    assert {:ok, pending} = Envelope.new(graph, %{}, %{"job_id" => fixture.id}, %{})

    assert {:ok, recovery, _} =
             Envelope.claim(pending, fixture.claim["owner_session_id"], :initial)

    Map.put(fixture.runtime, key, recovery)
  end

  defp terminal_job do
    fixture = Fixture.claimed_job()
    runtime = completed(fixture.runtime)
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    %{fixture | runtime: runtime}
  end

  defp completed(runtime) do
    key = Envelope.internal_key()
    Map.put(runtime, key, Envelope.transition(runtime[key], "completed"))
  end

  defp replace(fixture, :delete_result),
    do: assert({:ok, _} = AnalysisResultStore.delete(fixture.id))

  defp replace(fixture, replacement) do
    runtime =
      case replacement do
        :active -> fixture.valid_runtime
        :terminal -> completed(fixture.valid_runtime)
        :annotate -> Map.put(fixture.runtime, "annotation", "newer evidence")
        :plain -> %{"output" => 42}
      end

    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)
  end

  defp pause_scan(id, mode \\ :recovery) do
    coordinator = Process.whereis(Coordinator)
    # The first read routes recovery; the second checks pending completion under lease.
    probe = ResultReadProbe.pause(id, read: if(mode == :initial, do: 1, else: 3))

    task =
      Task.async(fn ->
        if mode == :initial, do: Coordinator.dispatch(id), else: Coordinator.recover_now()
      end)

    assert_receive {:result_read_paused, ^probe, ^coordinator, false}, 2_000
    on_exit(probe, fn -> send(coordinator, :resume_result_read) end)
    {task, coordinator, probe}
  end

  defp resume(coordinator, probe) do
    send(coordinator, :resume_result_read)
    on_exit(probe, fn -> :ok end)
  end

  defp assert_blocked(fixture) do
    assert {:ok, %{status: :failed, progress: 1.0}} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    key = Envelope.internal_key()
    assert runtime[key]["state"] == "recovery_blocked"
    assert runtime[key]["envelope"] == fixture.runtime[key]["envelope"]
    assert runtime[key]["envelope_sha256"] == fixture.runtime[key]["envelope_sha256"]
  end

  defp snapshot(id), do: {Store.get(id), AnalysisResultStore.get(id)}
end
