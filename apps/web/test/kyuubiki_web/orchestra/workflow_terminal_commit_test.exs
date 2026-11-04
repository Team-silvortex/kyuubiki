defmodule KyuubikiWeb.Orchestra.WorkflowTerminalCommitTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryScan, as: Scan
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, ResultReadProbe}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Outage.setup()
  end

  for terminal <- ["completed", "failed", "cancelled", "recovery_blocked"],
      mutation <- [:replace, :reactivate, :annotate, :delete_result] do
    test "#{terminal} cannot consume a runtime snapshot invalidated by #{mutation}" do
      fixture = terminal_runtime(unquote(terminal))
      changed = mutate(fixture, unquote(mutation))

      assert {:error, _} =
               Scan.reconcile_terminal(fixture.id, fixture.runtime)

      assert snapshot(fixture) == changed
    end
  end

  for mutation <- [:cancel_job, :delete_job] do
    @tag skip: not Storage.sqlite?()
    test "a concurrent #{mutation} after the runtime read wins without resurrection" do
      healthy = terminal_runtime("completed")
      fixture = terminal_runtime("completed")
      coordinator = Process.whereis(Coordinator)
      probe = ResultReadProbe.pause(fixture.id)
      task = Task.async(fn -> Coordinator.recover_now() end)
      assert_receive {:result_read_paused, ^probe, ^coordinator, false}, 2_000
      on_exit(probe, fn -> send(coordinator, :resume_result_read) end)

      case unquote(mutation) do
        :cancel_job ->
          assert {:ok, _} =
                   Store.apply_progress(%{job_id: fixture.id, stage: :cancelled, progress: 1.0})

        :delete_job ->
          assert {:ok, _} = Store.delete(fixture.id)
      end

      retained = snapshot(fixture)
      send(coordinator, :resume_result_read)
      on_exit(probe, fn -> :ok end)
      summary = Task.await(task, 2_000)
      expected = if unquote(mutation) == :cancel_job, do: "completed", else: "interrupted"
      assert summary["status"] == expected
      assert snapshot(fixture) == retained
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
      assert Process.whereis(Coordinator) == coordinator
    end
  end

  @tag skip: not Storage.sqlite?()
  @tag capture_log: true
  test "process loss after the job write rolls back terminal reconciliation before lease commit" do
    fixture = terminal_runtime("completed")
    assert {:ok, lease} = LeaseStore.acquire("terminal-commit-#{fixture.id}", "writer", 60_000)
    on_exit(fn -> LeaseStore.release(lease) end)
    probe = AnalysisCommitFault.pause_after_job_write(fixture.id)
    reconcile = fn -> Scan.reconcile_terminal(fixture.id, fixture.runtime) end
    {pid, ref} = spawn_monitor(fn -> LeaseStore.with_lease(lease, reconcile) end)
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    assert_receive {:completion_paused, ^probe, ^pid}, 2_000
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^ref, :process, ^pid, :killed}, 2_000
    Fixture.unchanged(fixture)

    assert :ok = LeaseStore.with_lease(lease, reconcile)
    assert {:ok, %{status: :completed}} = Store.get(fixture.id)
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime == fixture.runtime
  end

  for winner <- [:completed, :failed, :cancelled] do
    test "an already #{winner} job wins without rewriting either record" do
      fixture = terminal_runtime("completed")

      assert {:ok, _} =
               Store.apply_progress(%{
                 job_id: fixture.id,
                 stage: unquote(winner),
                 progress: 1.0,
                 message: "independent terminal decision"
               })

      retained = snapshot(fixture)
      assert :ok = Scan.reconcile_terminal(fixture.id, fixture.runtime)
      assert snapshot(fixture) == retained
    end
  end

  test "a removed job is not recreated by terminal reconciliation" do
    fixture = terminal_runtime("completed")
    assert {:ok, _} = Store.delete(fixture.id)
    retained = snapshot(fixture)

    assert {:error, {:job_not_found, _}} =
             Scan.reconcile_terminal(fixture.id, fixture.runtime)

    assert snapshot(fixture) == retained
  end

  for mutation <- [:replace, :annotate, :delete_result] do
    @tag skip: not Storage.sqlite?()
    test "a live #{mutation} after the runtime read cannot finalize a stale terminal decision" do
      healthy = terminal_runtime("completed")
      fixture = terminal_runtime("completed")
      assert [first, second] = Store.list()
      assert {first.job_id, second.job_id} == {fixture.id, healthy.id}
      before = Coordinator.snapshot()
      coordinator = Process.whereis(Coordinator)
      probe = ResultReadProbe.pause(fixture.id)
      task = Task.async(fn -> Coordinator.recover_now() end)
      assert_receive {:result_read_paused, ^probe, ^coordinator, false}, 2_000
      on_exit(probe, fn -> send(coordinator, :resume_result_read) end)
      changed = mutate(fixture, unquote(mutation))
      send(coordinator, :resume_result_read)
      on_exit(probe, fn -> :ok end)

      assert %{
               "status" => "interrupted",
               "reason" => "workflow_terminal_reconciliation_incomplete",
               "active_jobs" => 2,
               "skipped" => 2
             } = Task.await(task, 2_000)

      assert Process.whereis(Coordinator) == coordinator
      assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
      assert snapshot(fixture) == changed
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
      assert {:ok, runtime} = AnalysisResultStore.get(healthy.id)
      assert runtime == healthy.runtime

      # Recovery must re-read the current record; it cannot reuse the rejected snapshot.
      if unquote(mutation) == :delete_result do
        assert :ok = AnalysisResultStore.put(fixture.id, fixture.runtime)
      end

      assert %{"status" => "completed", "active_jobs" => 1} = Coordinator.recover_now()
      expected = if unquote(mutation) == :replace, do: :cancelled, else: :completed
      assert {:ok, job} = Store.get(fixture.id)
      assert job.status == expected
      assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"] + 1
    end
  end

  for target <- [:job, :result] do
    @tag skip: not Storage.sqlite?()
    test "a rejected #{target} write is not a completed recovery scan and permits a fresh retry" do
      healthy = terminal_runtime("completed")
      fixture = terminal_runtime("completed")
      before = Coordinator.snapshot()
      coordinator = Process.whereis(Coordinator)

      drop =
        case unquote(target) do
          :job -> AnalysisCommitFault.reject_job_update(fixture.id)
          :result -> AnalysisCommitFault.reject_result_update(fixture.id)
        end

      assert %{
               "status" => "interrupted",
               "reason" => "workflow_terminal_reconciliation_incomplete"
             } = Coordinator.recover_now()

      Fixture.unchanged(fixture)
      assert Process.whereis(Coordinator) == coordinator
      assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
      assert {:ok, %{status: :completed}} = Store.get(healthy.id)
      drop.()
      assert %{"status" => "completed", "active_jobs" => 1} = Coordinator.recover_now()
      assert {:ok, %{status: :completed}} = Store.get(fixture.id)
      assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
      assert runtime == fixture.runtime
    end
  end

  @tag skip: not Storage.memory?()
  test "a failed shared snapshot cannot report completed or change in-memory terminal state" do
    fixture = terminal_runtime("completed")
    healthy = terminal_runtime("completed")
    before = Coordinator.snapshot()
    coordinator = Process.whereis(Coordinator)
    path = Persistence.analysis_state_path()
    bytes = File.read!(path)
    pending = path <> ".next"
    assert :ok = File.mkdir(pending)
    File.write!(Path.join(pending, "blocker"), "injected fault")
    on_exit(fn -> File.rm_rf!(pending) end)

    assert %{
             "status" => "interrupted",
             "reason" => "workflow_terminal_reconciliation_incomplete",
             "active_jobs" => 2
           } = Coordinator.recover_now()

    assert Process.whereis(Coordinator) == coordinator
    assert File.read!(path) == bytes
    assert Coordinator.snapshot()["recovery_runs"] == before["recovery_runs"]
    Fixture.unchanged(fixture)
    Fixture.unchanged(healthy)
    File.rm_rf!(pending)
    assert %{"status" => "completed", "active_jobs" => 2} = Coordinator.recover_now()
    assert {:ok, %{status: :completed}} = Store.get(fixture.id)
    assert {:ok, %{status: :completed}} = Store.get(healthy.id)
  end

  defp terminal_runtime(terminal) do
    fixture = Fixture.claimed_job()
    key = Envelope.internal_key()
    ended = Envelope.transition(fixture.runtime[key], terminal)
    runtime = Map.put(fixture.runtime, key, ended)
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    Map.put(%{fixture | runtime: runtime}, :active_runtime, fixture.runtime)
  end

  defp mutate(fixture, mutation) do
    case mutation do
      :replace ->
        terminal = fixture.runtime[Envelope.internal_key()]["state"]
        replacement_state = if terminal == "cancelled", do: "completed", else: "cancelled"

        replacement =
          put_in(fixture.runtime, [Envelope.internal_key(), "state"], replacement_state)

        assert :ok =
                 AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, replacement)

      :annotate ->
        assert :ok =
                 AnalysisResultStore.compare_and_swap(
                   fixture.id,
                   fixture.runtime,
                   Map.put(fixture.runtime, "annotation", "newer result annotation")
                 )

      :reactivate ->
        assert :ok =
                 AnalysisResultStore.compare_and_swap(
                   fixture.id,
                   fixture.runtime,
                   fixture.active_runtime
                 )

      :delete_result ->
        assert {:ok, _} = AnalysisResultStore.delete(fixture.id)
    end

    snapshot(fixture)
  end

  defp snapshot(fixture),
    do: {Store.get(fixture.id), AnalysisResultStore.get(fixture.id)}
end
