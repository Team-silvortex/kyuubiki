defmodule KyuubikiWeb.Orchestra.WorkflowNativeFailureRetentionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidence, as: Evidence
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.{OperatorTaskExecutionSummary, OperatorTaskIR}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @operator "extract.thermal_result_diagnostics"
  @key "_workflow_recovery"

  setup do
    Fixture.setup_coordinator()
    root = Path.join(System.tmp_dir!(), "native-failure-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})

    on_exit(fn ->
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    %{root: root}
  end

  for state <- ~w(dispatch_boundary_unconfirmed outcome_unknown) do
    test "ordinary failure preserves #{state} without rewriting journal evidence", ctx do
      f = fixture(unquote(state))
      path = Path.join(ctx.root, f.attempt <> ".json")
      bytes = File.read!(path)
      assert :ok = Coordinator.fail(f.id, f.claim, "workflow runner stopped")
      retained = assert_blocked(f, "workflow_dispatch_outcome_unconfirmed")
      assert File.read!(path) == bytes
      assert List.last(retained[@key]["history"])["message"] == "workflow runner stopped"

      assert :ok = stop_supervised(Journal)
      start_supervised!({Journal, root: ctx.root})
      Fixture.restart_coordinator()
      assert Coordinator.recover_now()["recovered"] == 0
      assert {:ok, ^retained} = AnalysisResultStore.get(f.id)
      assert File.read!(path) == bytes
    end
  end

  for state <- ~w(not_dispatched observed_executed observed_failed observed_blocked) do
    test "confirmed #{state} allows ordinary failure cleanup" do
      f = fixture(unquote(state))
      assert :ok = Coordinator.fail(f.id, f.claim, "downstream failure")
      assert {:ok, %{status: :failed, message: "downstream failure"}} = Store.get(f.id)
      assert {:ok, after_failure} = AnalysisResultStore.get(f.id)
      assert after_failure[@key]["state"] == "failed"
      refute Map.has_key?(after_failure[@key], "envelope")
      refute Map.has_key?(after_failure[@key], Evidence.key())
      assert {:ok, [%{"state" => unquote(state)}]} = Journal.lookup_task(f.task_id)
    end
  end

  for damage <- [:missing_directory, :empty_directory, :unavailable, :corrupt_record] do
    @tag damage: damage
    test "#{damage} cannot erase native evidence while finalizing a generic error", ctx do
      f = fixture("observed_executed")
      assert :ok = stop_supervised(Journal)

      case ctx.damage do
        :missing_directory ->
          File.rm_rf!(ctx.root)

        :empty_directory ->
          File.rm_rf!(ctx.root)
          File.mkdir_p!(ctx.root)

        :corrupt_record ->
          File.write!(Path.join(ctx.root, f.attempt <> ".json"), "{}")

        :unavailable ->
          :ok
      end

      if ctx.damage != :unavailable, do: start_supervised!({Journal, root: ctx.root})
      assert :ok = Coordinator.fail(f.id, f.claim, "workflow stopped")
      assert_blocked(f, "workflow_dispatch_journal_unavailable")

      if ctx.damage == :missing_directory, do: refute(File.exists?(ctx.root))
      if ctx.damage == :empty_directory, do: assert(File.ls!(ctx.root) == [])
      assert Process.alive?(Process.whereis(Coordinator))
    end
  end

  for corruption <- [:missing_bucket, :null_bucket, :bad_digest, :bad_envelope] do
    @tag corruption: corruption
    test "#{corruption} blocks cleanup instead of laundering invalid recovery evidence", ctx do
      f = fixture("observed_failed")

      damaged =
        case ctx.corruption do
          :missing_bucket ->
            Map.delete(f.runtime[@key], Evidence.key())

          :null_bucket ->
            Map.put(f.runtime[@key], Evidence.key(), nil)

          :bad_digest ->
            put_in(f.runtime[@key], [Evidence.key(), "sha256"], String.duplicate("0", 64))

          :bad_envelope ->
            Map.put(f.runtime[@key], "envelope_sha256", String.duplicate("0", 64))
        end

      runtime = Map.put(f.runtime, @key, damaged)
      assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)
      assert :ok = Coordinator.fail(f.id, f.claim, "workflow stopped")

      reason =
        if ctx.corruption == :bad_envelope,
          do: "workflow_recovery_digest_mismatch",
          else: "workflow_native_dispatch_evidence_invalid"

      assert_blocked(%{f | runtime: runtime}, reason)
    end
  end

  test "pure local failure is independent of an unavailable dispatch journal" do
    f = Fixture.claimed_job()
    assert :ok = stop_supervised(Journal)
    assert :ok = Coordinator.fail(f.id, f.claim, "local failure")
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final[@key]["state"] == "failed"
    refute Map.has_key?(final[@key], "envelope")
  end

  test "an undispatched native graph with an empty journal can fail normally" do
    f = fixture(:no_dispatch)
    assert :ok = Coordinator.fail(f.id, f.claim, "invalid input")
    assert {:ok, final} = AnalysisResultStore.get(f.id)
    assert final[@key]["state"] == "failed"
    refute Map.has_key?(final[@key], Evidence.key())
  end

  test "stale claim cannot trigger failure cleanup or change uncertain evidence" do
    f = fixture("outcome_unknown")
    claim = Map.put(f.claim, "generation", f.claim["generation"] + 1)
    assert :ok = stop_supervised(Journal)
    assert {:error, :stale_workflow_execution_claim} = Coordinator.fail(f.id, claim, "late error")
    Fixture.unchanged(f)
  end

  test "invalid failure text still rejects the whole transaction with uncertain evidence" do
    f = fixture("outcome_unknown")
    assert {:error, _} = Coordinator.fail(f.id, f.claim, String.duplicate("x", 4097))
    Fixture.unchanged(f)
    assert :ok = Coordinator.fail(f.id, f.claim, "valid failure")
    assert_blocked(f, "workflow_dispatch_outcome_unconfirmed")
  end

  test "rejected job progress cannot publish blocked recovery ahead of the job" do
    f = fixture("outcome_unknown")

    assert {:ok, future} =
             Store.apply_progress(%{
               job_id: f.id,
               stage: :solving,
               progress: 0.2,
               emitted_at: DateTime.add(DateTime.utc_now(), 60, :second)
             })

    assert {:error, _} = Coordinator.fail(f.id, f.claim, "stale failure")
    Fixture.unchanged(%{f | job: future})
  end

  defp fixture(state) do
    node = %{"id" => "diagnose", "kind" => "extract", "operator_id" => @operator}
    f = Fixture.claimed_job(graph: %{"id" => "native-failure", "nodes" => [node]})
    task_id = "#{f.id}:diagnose:#{@operator}"
    {:ok, task} = OperatorTaskIR.build(@operator, %{}, %{}, task_id: task_id)
    {:ok, summary} = OperatorTaskExecutionSummary.build(task)
    request = %{"id" => f.id, "params" => %{"task_ir" => task, "job_id" => f.id}}

    attempt =
      if state != :no_dispatch do
        {:ok, attempt} =
          Journal.begin_dispatch(request, %{id: "failure-agent", host: "127.0.0.1", port: 5001})

        identity = Map.take(summary, ~w(task_id task_digest operator_id program_id))

        assert :ok =
                 Coordinator.native_dispatch_evidence(f.id, f.claim, {:commit, identity, attempt})

        if state != "dispatch_boundary_unconfirmed",
          do: assert(:ok = Journal.finish_dispatch(attempt, state))

        attempt
      end

    {:ok, runtime} = AnalysisResultStore.get(f.id)
    {:ok, job} = Store.get(f.id)
    Map.merge(f, %{job: job, runtime: runtime, task_id: task_id, attempt: attempt})
  end

  defp assert_blocked(f, reason) do
    assert {:ok, %{status: :failed}} = Store.get(f.id)
    assert {:ok, retained} = AnalysisResultStore.get(f.id)
    recovery = retained[@key]
    assert recovery["state"] == "recovery_blocked"
    assert recovery["generation"] == f.claim["generation"]
    assert recovery["attempt"] == f.claim["attempt"]
    assert recovery["envelope"] == f.runtime[@key]["envelope"]
    assert recovery[Evidence.key()] == f.runtime[@key][Evidence.key()]
    assert List.last(recovery["history"])["reason"] == reason
    assert is_binary(List.last(recovery["history"])["next_action"])
    retained
  end
end
