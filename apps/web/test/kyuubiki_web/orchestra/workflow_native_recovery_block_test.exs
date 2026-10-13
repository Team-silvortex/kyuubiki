defmodule KyuubikiWeb.Orchestra.WorkflowNativeRecoveryBlockTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner

  setup do
    Fixture.setup_coordinator()
  end

  for reason <-
        ~w(journal_unavailable dispatch_unconfirmed input_changed dispatch_identity_invalid dispatch_attempt_ambiguous dispatch_observation_changed dispatch_evidence_missing dispatch_evidence_invalid dispatch_evidence_commit_failed receipt_unavailable receipt_invalid)a do
    test "#{reason} atomically blocks the current claim and retains its execution envelope" do
      fixture = Fixture.claimed_job()
      reason = unquote(reason)

      assert :ok =
               WorkflowRecoveryCoordinator.block_native_recovery(
                 fixture.id,
                 fixture.claim,
                 reason
               )

      assert {:ok, %{status: :failed}} = Store.get(fixture.id)
      assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
      key = WorkflowRecoveryEnvelope.internal_key()
      recovery = runtime[key]
      assert recovery["state"] == "recovery_blocked"
      assert recovery["envelope"] == fixture.runtime[key]["envelope"]
      assert recovery["envelope_sha256"] == fixture.runtime[key]["envelope_sha256"]
      assert recovery["generation"] == fixture.claim["generation"]
      assert recovery["attempt"] == fixture.claim["attempt"]

      assert List.last(recovery["history"])["next_action"] ==
               "restore_original_native_receipt_or_explicit_resubmit"

      assert WorkflowRecoveryCoordinator.recover_now()["recovered"] == 0
      assert runtime["artifacts"] == %{}
    end
  end

  for status <- [:completed, :cancelled, :failed] do
    test "a #{status} job cannot be relabeled as a blocked native recovery" do
      fixture = Fixture.terminal_job(unquote(status))

      assert {:error, {:workflow_job_terminal, unquote(status)}} =
               WorkflowRecoveryCoordinator.block_native_recovery(
                 fixture.id,
                 fixture.claim,
                 :receipt_unavailable
               )

      Fixture.unchanged(fixture)
    end
  end

  test "stale generations and attempts cannot attach a recovery failure to another claim" do
    fixture = Fixture.claimed_job()

    for field <- ~w(generation attempt owner_session_id) do
      stale =
        Map.update!(fixture.claim, field, fn value ->
          if is_integer(value), do: value + 1, else: value <> "-stale"
        end)

      assert {:error, :stale_workflow_execution_claim} =
               WorkflowRecoveryCoordinator.block_native_recovery(
                 fixture.id,
                 stale,
                 :receipt_unavailable
               )

      Fixture.unchanged(fixture)
    end
  end

  test "lease takeover cannot commit a blocked node for the previous owner" do
    fixture = Fixture.claimed_job()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)

    assert {:error, :orchestra_lease_lost} =
             WorkflowRecoveryCoordinator.block_native_recovery(
               fixture.id,
               fixture.claim,
               :receipt_unavailable
             )

    Fixture.unchanged(fixture)
  end

  test "invalid failure reasons cannot mutate or expose supplied content" do
    fixture = Fixture.claimed_job()

    for reason <- ["secret", %{token: "secret"}, nil, :unknown] do
      assert {:error, :invalid_native_workflow_recovery_reason} =
               WorkflowRecoveryCoordinator.block_native_recovery(
                 fixture.id,
                 fixture.claim,
                 reason
               )

      Fixture.unchanged(fixture)
    end
  end

  test "a rejected blocked commit cannot discard the retained execution envelope" do
    fixture = Fixture.claimed_job()

    {:ok, future_job} =
      Store.apply_progress(%{
        job_id: fixture.id,
        stage: :solving,
        progress: 0.2,
        emitted_at: DateTime.add(DateTime.utc_now(), 60, :second)
      })

    assert {:error, _} =
             WorkflowRecoveryCoordinator.block_native_recovery(
               fixture.id,
               fixture.claim,
               :receipt_unavailable
             )

    Fixture.unchanged(%{fixture | job: future_job})
  end

  test "on_error skip cannot turn an unavailable recovery receipt into a partial success" do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}

    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.blocked-recovery-skip",
      "nodes" => [
        %{"id" => "input", "kind" => "input", "inputs" => [], "outputs" => [port]},
        %{
          "id" => "native",
          "kind" => "extract",
          "operator_id" => "extract.test",
          "config" => %{"on_error" => "skip"},
          "inputs" => [port],
          "outputs" => [port]
        },
        %{"id" => "output", "kind" => "output", "inputs" => [port], "outputs" => []}
      ],
      "edges" => [edge("input", "native"), edge("native", "output")]
    }

    for reason <- [
          :receipt_unavailable,
          :dispatch_attempt_ambiguous,
          :dispatch_observation_changed,
          :result_checkpoint_invalid,
          :result_checkpoint_unavailable,
          :result_checkpoint_commit_failed
        ] do
      error = {:native_workflow_recovery_blocked, reason}

      assert {:error, {:workflow_node_error, "native", ^error}} =
               WorkflowGraphRunner.run(graph, %{"input" => %{"value" => 42}},
                 execute_solve: fn _, _, _ -> flunk("no solve") end,
                 execute_transform: fn _, _, _ -> flunk("no transform") end,
                 execute_extract: fn _, _, _ -> {:error, error} end,
                 execute_export: fn _, _, _ -> flunk("no export") end
               )
    end
  end

  defp edge(from, to),
    do: %{
      "id" => from <> "-" <> to,
      "from" => %{"node" => from, "port" => "payload"},
      "to" => %{"node" => to, "port" => "payload"},
      "artifact_type" => "payload/json"
    }
end
