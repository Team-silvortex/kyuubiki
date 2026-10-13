defmodule KyuubikiWeb.Orchestra.WorkflowCompletionCheckpointTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage

  setup do
    Outage.setup()
  end

  test "staging is durable private and idempotent; publication prunes the handoff" do
    f = Fixture.claimed_job()
    result = Map.put(Fixture.result(), "precision", [1.0, 1.234567890123456e-12])
    assert :ok = Coordinator.stage_result(f.id, f.claim, result)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    recovery = runtime[Envelope.internal_key()]
    assert {:ok, claim, ^result} = Checkpoint.read(f.job, recovery)
    assert claim == f.claim
    assert {:ok, staged_job} = Store.get(f.id)
    assert staged_job.status == :solving
    assert staged_job.progress == f.job.progress
    assert Envelope.public_result(runtime)["artifacts"] == %{}
    refute inspect(Envelope.public_result(runtime)) =~ "result_json"
    assert :ok = Coordinator.stage_result(f.id, f.claim, result)
    Fixture.unchanged(%{f | job: staged_job, runtime: runtime})

    assert {:error, :workflow_completion_checkpoint_conflict} =
             Coordinator.commit_result(f.id, f.claim, Fixture.result())

    Fixture.unchanged(%{f | job: staged_job, runtime: runtime})
    assert :ok = Coordinator.commit_result(f.id, f.claim, result)
    assert {:ok, %{status: :completed}} = Store.get(f.id)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["precision"] === result["precision"]
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
  end

  test "completed computation can publish after restart even when replay is forbidden" do
    f = claimed_checkpoint_required()
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    Fixture.restart_coordinator()
    Outage.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(f.id)) end)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final["artifacts"] == Fixture.result()["artifacts"]
    assert final[Envelope.internal_key()]["generation"] == 1
    assert final[Envelope.internal_key()]["attempt"] == 1
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
  end

  for field <-
        ~w(result_json sha256 job_id job_created_at_us envelope_sha256 producer_claim schema_version) do
    test "a changed #{field} blocks recovery rather than recomputing" do
      f = claimed_checkpoint_required()
      assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
      assert {:ok, runtime} = AnalysisResultStore.get(f.id)
      path = [Envelope.internal_key(), Checkpoint.key(), unquote(field)]
      changed = put_in(runtime, path, "corrupt")
      assert :ok = AnalysisResultStore.compare_and_swap(f.id, runtime, changed)
      Fixture.restart_coordinator()
      Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(f.id)) end)
      assert {:ok, final} = AnalysisResultStore.get(f.id)
      assert final[Envelope.internal_key()]["state"] == "recovery_blocked"
      assert final[Envelope.internal_key()]["generation"] == 1
      assert {:ok, job} = Store.get(f.id)
      assert job.message =~ "workflow_completion_checkpoint_invalid"
    end
  end

  test "cancellation wins over a staged completion and removes it" do
    f = Fixture.claimed_job()
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert :ok = Coordinator.cancel(f.id)
    Fixture.restart_coordinator()
    assert {:ok, %{status: :cancelled}} = Store.get(f.id)
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    refute Map.has_key?(final[Envelope.internal_key()], Checkpoint.key())
  end

  test "invalid and over-budget output is rejected without writing a pending record" do
    f = Fixture.claimed_job()

    for result <- [
          %{"bad" => self()},
          %{"failed_nodes" => nil},
          %{"long" => String.duplicate("x", 1_000_001)}
        ] do
      assert {:error, :workflow_completion_checkpoint_invalid} =
               Coordinator.stage_result(f.id, f.claim, result)

      Fixture.unchanged(f)
    end
  end

  test "a different result cannot overwrite an already staged completion" do
    f = Fixture.claimed_job()
    assert :ok = Coordinator.stage_result(f.id, f.claim, Fixture.result())
    assert {:ok, job} = Store.get(f.id)
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)

    assert {:error, :workflow_completion_checkpoint_conflict} =
             Coordinator.stage_result(f.id, f.claim, %{"artifacts" => %{"output" => 999}})

    Fixture.unchanged(%{f | job: job, runtime: runtime})
  end

  @tag skip: not Storage.sqlite?()
  test "the real SQLite connection uses WAL with FULL commit synchronization" do
    repo = Storage.repo_module!()
    assert Ecto.Adapters.SQL.query!(repo, "PRAGMA journal_mode", []).rows == [["wal"]]
    assert Ecto.Adapters.SQL.query!(repo, "PRAGMA synchronous", []).rows == [[2]]
  end

  defp claimed_checkpoint_required do
    Fixture.claimed_job(
      graph: %{
        "id" => "publication-only",
        "nodes" => [],
        "recovery_policy" => %{"retry_safety" => "checkpoint_required"}
      }
    )
  end
end
