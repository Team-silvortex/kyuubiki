defmodule KyuubikiWeb.Orchestra.WorkflowCompletionPhaseTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key Envelope.internal_key()

  setup do
    Outage.setup()
  end

  test "staging atomically enters durable finalization without changing the execution claim" do
    f = staged()
    recovery = f.runtime[@key]
    assert recovery["state"] == "finalizing"
    assert recovery["generation"] == f.claim["generation"]
    assert recovery["attempt"] == f.claim["attempt"]
    assert List.last(recovery["history"])["event"] == "finalizing"
    refute Envelope.replayable?(recovery)

    for reason <- [:initial, :process_restart, :runner_loss] do
      assert {:error, {:workflow_recovery_not_active, "finalizing"}} =
               Envelope.claim(recovery, "replacement-owner", reason)
    end

    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    Fixture.unchanged(f)
    assert :ok = Coordinator.commit_result(f.id, f.claim, Fixture.result())
    assert_completed(f)
  end

  for safety <- ["idempotent", "checkpoint_required"], damage <- [:missing, :null] do
    test "restart never replays #{safety} finalization after #{damage} checkpoint" do
      f = staged(unquote(safety))
      changed = damage(f.runtime, unquote(damage))
      assert :ok = AnalysisResultStore.put(f.id, changed)
      Fixture.restart_coordinator()
      assert %{"lease" => %{"status" => "owner"}} = Coordinator.snapshot()
      assert {:ok, %{status: :failed, message: message}} = Store.get(f.id)
      assert message =~ "workflow_completion_checkpoint_invalid"
      assert {:ok, final} = AnalysisResultStore.get(f.id)
      assert final[@key]["state"] == "recovery_blocked"
      assert final[@key]["generation"] == f.claim["generation"]
      assert final[@key]["attempt"] == f.claim["attempt"]
      assert final["artifacts"] == %{}
      assert Map.has_key?(final[@key], Checkpoint.key()) == (unquote(damage) == :null)
    end
  end

  test "a removed finalizing checkpoint cannot be silently restaged or committed from memory" do
    f = staged()
    runtime = damage(f.runtime, :missing)
    assert :ok = AnalysisResultStore.put(f.id, runtime)
    f = %{f | runtime: runtime}

    for action <- [&Coordinator.stage_result/3, &Coordinator.commit_result/3] do
      assert {:error, :workflow_completion_checkpoint_invalid} =
               action.(f.id, f.claim, Fixture.result())

      Fixture.unchanged(f)
    end
  end

  test "late execution progress and failure cannot mutate finalizing evidence" do
    f = staged()

    assert {:error, :stale_workflow_execution_claim} =
             Coordinator.record_progress(f.id, f.claim, Fixture.progress())

    assert {:error, :stale_workflow_execution_claim} =
             Coordinator.fail(f.id, f.claim, "late execution failure")

    Fixture.unchanged(f)
    assert :ok = Coordinator.commit_result(f.id, f.claim, Fixture.result())
    assert_completed(f)
  end

  for action <- [:publish, :restage] do
    test "a retained legacy running checkpoint supports #{action} without graph execution" do
      f = staged()
      runtime = put_in(f.runtime, [@key, "state"], "running")
      assert :ok = AnalysisResultStore.put(f.id, runtime)

      if unquote(action) == :restage do
        assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
        assert {:ok, updated} = AnalysisResultStore.get(f.id)
        assert updated[@key]["state"] == "finalizing"
      end

      Fixture.restart_coordinator()
      assert_completed(f)
    end
  end

  for target <- [:job, :result] do
    @tag skip: not Storage.sqlite?()
    test "rejected SQLite #{target} write cannot persist half a finalization phase" do
      f = Fixture.claimed_job()

      drop =
        if unquote(target) == :job,
          do: AnalysisCommitFault.reject_job_update(f.id),
          else: AnalysisCommitFault.reject_result_update(f.id)

      assert {:error, _} = Coordinator.stage_result(f.id, f.claim, Fixture.result())
      Fixture.unchanged(f)
      drop.()
      assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
      assert {:ok, runtime} = AnalysisResultStore.get(f.id)
      assert runtime[@key]["state"] == "finalizing"
      assert {:ok, _, _} = Checkpoint.read(f.job, runtime[@key])
    end
  end

  @tag skip: not Storage.memory?()
  test "rejected JSON stage leaves both disk and execution phase unchanged" do
    f = Fixture.claimed_job()
    path = Persistence.analysis_state_path()
    bytes = File.read!(path)
    pending = path <> ".next"
    assert :ok = File.mkdir(pending)
    File.write!(Path.join(pending, "blocker"), "injected fault")
    on_exit(fn -> File.rm_rf!(pending) end)
    assert {:error, _} = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    Fixture.unchanged(f)
    assert File.read!(path) == bytes
    File.rm_rf!(pending)
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    assert runtime[@key]["state"] == "finalizing"
  end

  defp staged(safety \\ "idempotent") do
    f =
      Fixture.claimed_job(
        graph: %{
          "id" => "durable-finalization",
          "nodes" => [],
          "recovery_policy" => %{"retry_safety" => safety}
        }
      )

    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert {:ok, job} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    %{f | job: job, runtime: runtime}
  end

  defp damage(runtime, :missing),
    do: Map.update!(runtime, @key, &Map.delete(&1, Checkpoint.key()))

  defp damage(runtime, :null), do: put_in(runtime, [@key, Checkpoint.key()], nil)

  defp assert_completed(f) do
    Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["artifacts"] == Fixture.result()["artifacts"]
    assert final[@key]["generation"] == f.claim["generation"]
    assert final[@key]["attempt"] == f.claim["attempt"]
  end
end
