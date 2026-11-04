defmodule KyuubikiWeb.Orchestra.WorkflowCompletionReconciliationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner}
  alias KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowCompletionStore, as: CompletionStore
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.TestSupport.ResultReadProbe
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Outage.setup()
    :sys.replace_state(Coordinator, &%{&1 | lease_retry_ms: 60_000})
    :ok
  end

  for damage <- [:digest, :missing], trigger <- [:scan, :timer] do
    for replacement <- [
          :repaired,
          :terminal,
          :annotate,
          :plain,
          :delete_result,
          :progress,
          :recreated_job
        ] do
      @tag skip: not Storage.sqlite?()
      test "#{trigger} #{damage} refusal cannot consume a newer #{replacement} snapshot" do
        f = staged_completion()
        retry = schedule(f)
        damaged = damage(f, unquote(damage))
        before = Coordinator.snapshot()
        {task, coordinator, probe} = pause_recovery(f.id, unquote(trigger), retry)
        replace(f, damaged, unquote(replacement))
        retained = snapshot(f.id)
        resume(coordinator, probe)
        result = Task.await(task, 2_000)

        assert snapshot(f.id) == retained
        assert :error = WorkflowJobRunner.running(f.id)
        assert Process.whereis(Coordinator) == coordinator
        after_recovery = Coordinator.snapshot()
        assert after_recovery["pending_completion_retries"] == [f.id]
        assert after_recovery["recovery_runs"] == before["recovery_runs"]
        assert after_recovery["blocked_jobs"] == before["blocked_jobs"]

        if unquote(trigger) == :scan,
          do: assert(%{"status" => "interrupted", "blocked" => 0, "recovered" => 0} = result)

        case unquote(replacement) do
          :repaired ->
            assert %{"recovered" => 1} = Coordinator.recover_now()
            assert_completed(f)

          :terminal ->
            assert %{"recovered" => 0, "blocked" => 0} = Coordinator.recover_now()
            assert {:ok, %{status: :completed}} = Store.get(f.id)
            assert {:ok, runtime} = elem(retained, 1)
            assert {:ok, ^runtime} = AnalysisResultStore.get(f.id)

          kind when kind in [:annotate, :progress] ->
            assert %{"blocked" => 1} = Coordinator.recover_now()
            assert_blocked(f)

          :delete_result ->
            retry = Map.fetch!(:sys.get_state(Coordinator).completion_retries, f.id)
            recover(f.id, :timer, retry)
            assert snapshot(f.id) == retained
            assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]
            # Restoring execution metadata alone must not turn publication into graph replay.
            without_completion =
              Map.update!(f.runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))

            assert :ok = AnalysisResultStore.put(f.id, without_completion)
            assert %{"blocked" => 1, "recovered" => 0} = Coordinator.recover_now()
            assert_blocked(f)

          _ ->
            :ok
        end
      end
    end

    test "unchanged #{damage} is blocked by #{trigger} without graph replay" do
      f = staged_completion()
      retry = schedule(f)
      damage(f, unquote(damage))
      recover(f.id, unquote(trigger), retry)
      assert_blocked(f)
      assert Coordinator.snapshot()["pending_completion_retries"] == []
    end
  end

  for terminal <- [:completed, :failed, :cancelled, :deleted] do
    @tag skip: not Storage.sqlite?()
    test "a checkpoint refusal cannot overwrite an independent #{terminal} job" do
      f = staged_completion()
      retry = schedule(f)
      damage(f, :digest)
      {task, coordinator, probe} = pause_recovery(f.id, :scan, retry)

      if unquote(terminal) == :deleted do
        assert {:ok, _} = Store.delete(f.id)
      else
        assert {:ok, _} =
                 Store.apply_progress(%{job_id: f.id, stage: unquote(terminal), progress: 1.0})
      end

      retained = snapshot(f.id)
      resume(coordinator, probe)
      assert %{"blocked" => 0, "recovered" => 0} = Task.await(task, 2_000)
      assert snapshot(f.id) == retained
      assert :error = WorkflowJobRunner.running(f.id)
    end
  end

  @tag skip: not Storage.sqlite?()
  test "a first scan retains publication-only retry after its invalid decision becomes stale" do
    f = staged_completion()
    damaged = damage(f, :digest)
    {task, coordinator, probe} = pause_recovery(f.id, :scan, nil)
    replace(f, damaged, :repaired)
    retained = snapshot(f.id)
    resume(coordinator, probe)
    assert %{"status" => "interrupted", "blocked" => 0} = Task.await(task, 2_000)
    assert snapshot(f.id) == retained
    assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]
    assert %{"recovered" => 1} = Coordinator.recover_now()
    assert_completed(f)
  end

  @tag skip: not Storage.sqlite?()
  test "lease replacement after pending validation prevents blocking the original pair" do
    f = staged_completion()
    retry = schedule(f)
    damage(f, :digest)
    name = Coordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, lease} = LeaseStore.current(name)
    {task, coordinator, probe} = pause_recovery(f.id, :scan, retry)
    assert :ok = LeaseStore.release(lease)
    assert {:ok, replacement} = LeaseStore.acquire(name, "pending-replacement", 60_000)
    on_exit(fn -> LeaseStore.release(replacement) end)
    retained = snapshot(f.id)
    resume(coordinator, probe)

    assert %{"status" => "interrupted", "reason" => "orchestra_lease_lost", "blocked" => 0} =
             Task.await(task, 2_000)

    assert snapshot(f.id) == retained
    assert Coordinator.snapshot()["pending_completion_retries"] == []
  end

  test "manual dispatch of a known finalizer publishes its checkpoint without replay" do
    f = staged_completion()
    schedule(f)
    assert {:ok, _pid} = Coordinator.dispatch(f.id)
    assert_completed(f)
  end

  test "missing runtime retries stop when the active job has been deleted" do
    f = staged_completion()
    retry = schedule(f)
    assert {:ok, _} = AnalysisResultStore.delete(f.id)
    assert {:ok, _} = Store.delete(f.id)
    recover(f.id, :timer, retry)
    assert Coordinator.snapshot()["pending_completion_retries"] == []
    assert Store.get(f.id) == :error
    assert AnalysisResultStore.get(f.id) == :error
  end

  for damage <- [:digest, :missing],
      replacement <- [:repaired, :progress, :recreated_job, :delete_result] do
    test "#{damage} observed refusal preserves #{replacement} on either storage backend" do
      f = staged_completion()
      damaged = damage(f, unquote(damage))
      name = Coordinator.snapshot()["lease"]["lease_name"]
      assert {:ok, lease} = LeaseStore.current(name)
      assert {:block, job, runtime} = CompletionStore.pending(f.id, lease, true)
      replace(f, damaged, unquote(replacement))
      retained = snapshot(f.id)
      assert {:error, _} = CompletionStore.block_pending(job, runtime, lease)
      assert snapshot(f.id) == retained
    end
  end

  for trigger <- [:timer, :initial], replacement <- [:missing, :plain] do
    test "#{trigger} preserves publication-only intent across #{replacement} runtime" do
      f = staged_completion()
      retry = schedule(f)

      if unquote(replacement) == :missing,
        do: assert({:ok, _} = AnalysisResultStore.delete(f.id)),
        else: assert(:ok = AnalysisResultStore.put(f.id, %{"output" => 42}))

      retained = snapshot(f.id)

      if unquote(trigger) == :timer,
        do: recover(f.id, :timer, retry),
        else: assert({:error, _} = Coordinator.dispatch(f.id))

      assert snapshot(f.id) == retained
      assert Coordinator.snapshot()["pending_completion_retries"] == [f.id]
      assert :ok = AnalysisResultStore.put(f.id, f.runtime)
      assert %{"recovered" => 1} = Coordinator.recover_now()
      assert_completed(f)
    end
  end

  defp damage(f, kind) do
    key = Envelope.internal_key()

    changed =
      if kind == :digest,
        do: put_in(f.runtime, [key, Checkpoint.key(), "sha256"], "bad"),
        else: Map.update!(f.runtime, key, &Map.delete(&1, Checkpoint.key()))

    assert :ok = AnalysisResultStore.put(f.id, changed)
    changed
  end

  defp replace(f, _damaged, :recreated_job) do
    assert {:ok, _} = Store.delete(f.id)

    assert {:ok, _} =
             Store.create(%{
               job_id: f.id,
               project_id: "replacement",
               simulation_case_id: "replacement-case",
               status: :solving,
               created_at: DateTime.add(f.job.created_at, -1, :second)
             })
  end

  defp replace(f, _damaged, :progress) do
    assert {:ok, _} =
             Store.apply_progress(%{
               job_id: f.id,
               stage: :postprocessing,
               progress: 0.9,
               message: "newer publication progress"
             })
  end

  defp replace(f, _damaged, :delete_result),
    do: assert({:ok, _} = AnalysisResultStore.delete(f.id))

  defp replace(f, damaged, kind) do
    key = Envelope.internal_key()

    replacement =
      case kind do
        :repaired -> f.runtime
        :terminal -> Map.put(f.runtime, key, Envelope.transition(f.runtime[key], "completed"))
        :annotate -> Map.put(damaged, "annotation", "new evidence")
        :plain -> %{"artifacts" => %{"output" => 99}}
      end

    assert :ok = AnalysisResultStore.compare_and_swap(f.id, damaged, replacement)
  end

  defp schedule(f) do
    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      assert %{"recovered" => 0, "blocked" => 0} = Coordinator.recover_now()
      Map.fetch!(:sys.get_state(Coordinator).completion_retries, f.id)
    end)
  end

  defp staged_completion do
    f = Fixture.claimed_job()
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert {:ok, job} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    %{f | job: job, runtime: runtime}
  end

  defp pause_recovery(id, trigger, retry) do
    coordinator = Process.whereis(Coordinator)
    probe = ResultReadProbe.pause(id, read: 2, after_commit: true)
    task = Task.async(fn -> recover(id, trigger, retry) end)
    assert_receive {:result_read_committed, ^probe, ^coordinator, _}, 2_000
    on_exit(probe, fn -> send(coordinator, :resume_result_read) end)
    {task, coordinator, probe}
  end

  defp recover(_id, :scan, _retry), do: Coordinator.recover_now()

  defp recover(id, :timer, retry) do
    send(Coordinator, {:retry_completion, id, retry.token})
    Coordinator.snapshot()
  end

  defp resume(coordinator, probe) do
    send(coordinator, :resume_result_read)
    on_exit(probe, fn -> :ok end)
  end

  defp snapshot(id), do: {Store.get(id), AnalysisResultStore.get(id)}

  defp assert_blocked(f) do
    assert {:ok, %{status: :failed}} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    key = Envelope.internal_key()
    assert runtime[key]["state"] == "recovery_blocked"
    assert runtime[key]["generation"] == f.claim["generation"]
    assert runtime[key]["attempt"] == f.claim["attempt"]
    assert runtime["artifacts"] == %{}
    assert :error = WorkflowJobRunner.running(f.id)
  end

  defp assert_completed(f) do
    Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["artifacts"] == Fixture.result()["artifacts"]
    assert final[Envelope.internal_key()]["generation"] == f.claim["generation"]
    assert final[Envelope.internal_key()]["attempt"] == f.claim["attempt"]
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
  end
end
