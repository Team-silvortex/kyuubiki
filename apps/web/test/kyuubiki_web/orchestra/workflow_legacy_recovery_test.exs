defmodule KyuubikiWeb.Orchestra.WorkflowLegacyRecoveryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryScan, as: Scan
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, ResultReadProbe}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @reason "workflow_legacy_reconciliation_incomplete"
  @message "workflow recovery blocked: legacy active job has no durable execution envelope"

  setup do
    Outage.setup()
  end

  for trigger <- [:manual, :restart], shape <- [:absent, nil, [], "invalid"] do
    test "#{trigger} blocks legacy #{inspect(shape)} recovery without fabricating an envelope" do
      fixture = legacy_job(unquote(Macro.escape(shape)))

      if unquote(trigger) == :restart do
        Fixture.restart_coordinator()
        assert Coordinator.snapshot()["blocked_jobs"] == 1
      else
        assert %{"status" => "completed", "blocked" => 1} = Coordinator.recover_now()
      end

      assert_blocked(fixture)
      assert :error = WorkflowJobRunner.running(fixture.id)
      assert Coordinator.snapshot()["tracked_jobs"] == []
      assert %{"active_jobs" => 0, "blocked" => 0} = Coordinator.recover_now()
    end
  end

  for result <- [:missing, %{}, %{"workflow_id" => nil}, %{"workflow_id" => 42}] do
    test "a plain job with #{inspect(result)} runtime is not inferred to be a failed workflow" do
      id = "plain-recovery-#{System.unique_integer([:positive])}"

      assert {:ok, _} =
               Store.create(%{
                 job_id: id,
                 project_id: "plain",
                 simulation_case_id: "case",
                 status: :solving,
                 progress: 0.2
               })

      put_result(id, unquote(Macro.escape(result)))
      retained = snapshot(id)

      assert %{"status" => "completed", "skipped" => 1, "blocked" => 0} =
               Coordinator.recover_now()

      assert snapshot(id) == retained
      assert :error = WorkflowJobRunner.running(id)
    end
  end

  for mutation <- [:terminal, :active, :annotate, :plain, :delete_result] do
    test "legacy #{mutation} replacement rejects a stale paired commit on either backend" do
      fixture = legacy_job()
      mutate_runtime(fixture, unquote(mutation))
      retained = snapshot(fixture.id)

      assert {:error, _} = Scan.block_legacy(fixture.id, fixture.runtime)
      assert snapshot(fixture.id) == retained
    end

    @tag skip: not Storage.sqlite?()
    test "legacy #{mutation} after the result read rejects the old failure decision" do
      healthy = terminal_job()
      fixture = legacy_job()
      assert [first, second] = Store.list()
      assert {first.job_id, second.job_id} == {fixture.id, healthy.id}
      before = Coordinator.snapshot()
      {task, coordinator, probe} = pause_scan(fixture.id)
      mutate_runtime(fixture, unquote(mutation))
      retained = snapshot(fixture.id)
      resume(coordinator, probe)

      assert %{"status" => "interrupted", "reason" => @reason, "blocked" => 0, "skipped" => 2} =
               Task.await(task, 2_000)

      assert snapshot(fixture.id) == retained
      assert Process.whereis(Coordinator) == coordinator
      assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
      assert :error = WorkflowJobRunner.running(fixture.id)

      if unquote(mutation) in [:terminal, :annotate, :delete_result] do
        if unquote(mutation) == :delete_result,
          do: assert(:ok = AnalysisResultStore.put(fixture.id, fixture.runtime))

        assert %{"status" => "completed", "active_jobs" => 1} = Coordinator.recover_now()
        assert {:ok, job} = Store.get(fixture.id)
        expected = if unquote(mutation) == :terminal, do: :completed, else: :failed
        assert job.status == expected
      end
    end
  end

  for winner <- [:completed, :failed, :cancelled, :deleted] do
    @tag skip: not Storage.sqlite?()
    test "legacy recovery preserves a concurrent #{winner} job and does not count it blocked" do
      healthy = terminal_job()
      fixture = legacy_job()
      {task, coordinator, probe} = pause_scan(fixture.id)

      if unquote(winner) == :deleted do
        assert {:ok, _} = Store.delete(fixture.id)
      else
        assert {:ok, _} =
                 Store.apply_progress(%{
                   job_id: fixture.id,
                   stage: unquote(winner),
                   progress: 1.0,
                   message: "independent decision"
                 })
      end

      retained = snapshot(fixture.id)
      resume(coordinator, probe)
      summary = Task.await(task, 2_000)
      assert summary["blocked"] == 0
      expected = if unquote(winner) == :deleted, do: "interrupted", else: "completed"
      assert summary["status"] == expected
      assert snapshot(fixture.id) == retained
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
    end
  end

  for target <- [:job, :result] do
    @tag skip: not Storage.sqlite?()
    test "legacy #{target} write rejection rolls back and does not complete the scan" do
      healthy = terminal_job()
      fixture = legacy_job()
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

  for first_kind <- [:legacy, :terminal] do
    @tag skip: not Storage.memory?()
    test "legacy shared-snapshot failure retains the first #{first_kind} reason and persisted bytes" do
      {fixture, healthy, first_id, reason} = ordered_pair(unquote(first_kind))
      assert [first, _second] = Store.list()
      assert first.job_id == first_id
      before = Coordinator.snapshot()
      path = Persistence.analysis_state_path()
      bytes = File.read!(path)
      pending = path <> ".next"
      assert :ok = File.mkdir(pending)
      File.write!(Path.join(pending, "blocker"), "injected fault")
      on_exit(fn -> File.rm_rf!(pending) end)

      summary = Coordinator.recover_now()
      assert summary["status"] == "interrupted"
      assert summary["reason"] == reason
      assert summary["blocked"] == 0
      assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
      assert File.read!(path) == bytes
      Fixture.unchanged(fixture)
      Fixture.unchanged(healthy)
      File.rm_rf!(pending)
      assert %{"status" => "completed", "blocked" => 1} = Coordinator.recover_now()
      assert_blocked(fixture)
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
    end
  end

  @tag skip: not Storage.sqlite?()
  test "lease loss after a legacy read cannot fail either active job" do
    healthy = terminal_job()
    fixture = legacy_job()
    before = Coordinator.snapshot()
    name = before["lease"]["lease_name"]
    assert {:ok, lease} = LeaseStore.current(name)
    {task, coordinator, probe} = pause_scan(fixture.id)
    assert :ok = LeaseStore.release(lease)
    assert {:ok, other} = LeaseStore.acquire(name, "legacy-replacement-owner", 60_000)
    on_exit(fn -> LeaseStore.release(other) end)
    resume(coordinator, probe)

    assert %{"status" => "interrupted", "reason" => "orchestra_lease_lost", "blocked" => 0} =
             Task.await(task, 2_000)

    Fixture.unchanged(fixture)
    Fixture.unchanged(healthy)
    assert Process.whereis(Coordinator) == coordinator
    assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
  end

  defp legacy_job(shape \\ :absent) do
    fixture = Fixture.claimed_job()
    key = Envelope.internal_key()

    runtime =
      if shape == :absent,
        do: Map.delete(fixture.runtime, key),
        else: Map.put(fixture.runtime, key, shape)

    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    Map.put(%{fixture | runtime: runtime}, :active_runtime, fixture.runtime)
  end

  defp ordered_pair(:legacy) do
    healthy = terminal_job()
    fixture = legacy_job()
    {fixture, healthy, fixture.id, @reason}
  end

  defp ordered_pair(:terminal) do
    fixture = legacy_job()
    healthy = terminal_job()
    {fixture, healthy, healthy.id, "workflow_terminal_reconciliation_incomplete"}
  end

  defp terminal_job do
    fixture = Fixture.claimed_job()
    key = Envelope.internal_key()

    runtime =
      Map.put(fixture.runtime, key, Envelope.transition(fixture.runtime[key], "completed"))

    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    %{fixture | runtime: runtime}
  end

  defp mutate_runtime(fixture, :delete_result) do
    assert {:ok, _} = AnalysisResultStore.delete(fixture.id)
  end

  defp mutate_runtime(fixture, mutation) do
    replacement =
      case mutation do
        :terminal ->
          key = Envelope.internal_key()

          Map.put(
            fixture.active_runtime,
            key,
            Envelope.transition(fixture.active_runtime[key], "completed")
          )

        :active ->
          fixture.active_runtime

        :annotate ->
          Map.put(fixture.runtime, "annotation", "newer record")

        :plain ->
          %{"output" => 42}
      end

    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, replacement)
  end

  defp pause_scan(id) do
    coordinator = Process.whereis(Coordinator)
    probe = ResultReadProbe.pause(id)
    task = Task.async(fn -> Coordinator.recover_now() end)
    assert_receive {:result_read_paused, ^probe, ^coordinator, false}, 2_000
    on_exit(probe, fn -> send(coordinator, :resume_result_read) end)
    {task, coordinator, probe}
  end

  defp resume(coordinator, probe) do
    send(coordinator, :resume_result_read)
    on_exit(probe, fn -> :ok end)
  end

  defp assert_blocked(fixture) do
    assert {:ok, %{status: :failed, progress: 1.0, message: @message}} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime == fixture.runtime
  end

  defp snapshot(id), do: {Store.get(id), AnalysisResultStore.get(id)}

  defp put_result(_id, :missing), do: :ok
  defp put_result(id, result), do: assert(:ok = AnalysisResultStore.put(id, result))
end
