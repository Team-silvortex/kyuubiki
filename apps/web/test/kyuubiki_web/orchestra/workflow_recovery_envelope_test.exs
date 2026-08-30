defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelopeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope

  test "digest-binds an idempotent workflow and fences stale generations" do
    assert {:ok, recovery} =
             WorkflowRecoveryEnvelope.new(idempotent_graph(), input_artifacts(), %{}, %{})

    assert recovery["retry_safety"] == "idempotent"
    assert :ok = WorkflowRecoveryEnvelope.verify(recovery)
    assert WorkflowRecoveryEnvelope.replayable?(recovery)

    assert {:ok, first, first_claim} =
             WorkflowRecoveryEnvelope.claim(recovery, "session-a", :initial)

    assert first["generation"] == 1
    assert first["attempt"] == 1
    assert WorkflowRecoveryEnvelope.fenced?(first, first_claim)

    assert {:ok, replayed, replay_claim} =
             WorkflowRecoveryEnvelope.claim(first, "session-b", :process_restart)

    assert replayed["generation"] == 2
    assert replayed["attempt"] == 2
    refute WorkflowRecoveryEnvelope.fenced?(replayed, first_claim)
    assert WorkflowRecoveryEnvelope.fenced?(replayed, replay_claim)
  end

  test "allows a side-effect workflow once but blocks blind replay" do
    graph =
      idempotent_graph()
      |> Map.put("recovery_policy", %{"retry_safety" => "checkpoint_required"})

    assert {:ok, recovery} = WorkflowRecoveryEnvelope.new(graph, input_artifacts(), %{}, %{})
    refute WorkflowRecoveryEnvelope.replayable?(recovery)

    assert {:ok, running, _claim} =
             WorkflowRecoveryEnvelope.claim(recovery, "session-a", :initial)

    assert {:error, {:workflow_replay_blocked, "checkpoint_required"}} =
             WorkflowRecoveryEnvelope.claim(running, "session-b", :process_restart)
  end

  test "accepts checkpointed replay only with a verified checkpoint" do
    checkpoint = %{
      "operator_task_batch_checkpoint_verification_contract" =>
        "kyuubiki.operator_task_batch_checkpoint_verification/v1",
      "status" => "verified",
      "checkpoint_digest" => String.duplicate("a", 64)
    }

    graph =
      idempotent_graph()
      |> Map.put("recovery_policy", %{
        "retry_safety" => "checkpointed",
        "checkpoint" => checkpoint
      })

    assert {:ok, recovery} = WorkflowRecoveryEnvelope.new(graph, input_artifacts(), %{}, %{})
    assert :ok = WorkflowRecoveryEnvelope.verify(recovery)
    assert WorkflowRecoveryEnvelope.replayable?(recovery)
  end

  test "rejects execution payload and recovery policy tampering" do
    assert {:ok, recovery} =
             WorkflowRecoveryEnvelope.new(idempotent_graph(), input_artifacts(), %{}, %{})

    payload_tamper = put_in(recovery, ["envelope", "input_artifacts", "input", "value"], 99)

    assert {:error, :workflow_recovery_digest_mismatch} =
             WorkflowRecoveryEnvelope.verify(payload_tamper)

    policy_tamper = Map.put(recovery, "retry_safety", "checkpoint_required")

    assert {:error, :workflow_recovery_policy_mismatch} =
             WorkflowRecoveryEnvelope.verify(policy_tamper)
  end

  test "normalizes JSON numbers before binding the durable envelope" do
    artifacts = %{
      "input" => %{
        "integral_float" => 1000.0,
        "fraction" => 0.01,
        "nested" => [2.0, 2.5]
      }
    }

    assert {:ok, recovery} =
             WorkflowRecoveryEnvelope.new(idempotent_graph(), artifacts, %{}, %{})

    normalized = recovery["envelope"]["input_artifacts"]["input"]
    assert normalized["integral_float"] === 1000
    assert normalized["fraction"] === 0.01
    assert normalized["nested"] === [2, 2.5]
    assert :ok = WorkflowRecoveryEnvelope.verify(recovery)
  end

  test "public results expose audit state but never the durable execution payload" do
    assert {:ok, recovery} =
             WorkflowRecoveryEnvelope.new(idempotent_graph(), input_artifacts(), %{}, %{})

    completed = WorkflowRecoveryEnvelope.transition(recovery, "completed")
    refute Map.has_key?(completed, "envelope")

    public =
      WorkflowRecoveryEnvelope.public_result(%{
        "value" => 7,
        WorkflowRecoveryEnvelope.internal_key() => completed
      })

    assert public["value"] == 7
    assert public["recovery"]["state"] == "completed"
    assert public["recovery"]["envelope_retained"] == false
    refute Map.has_key?(public, WorkflowRecoveryEnvelope.internal_key())
    refute Map.has_key?(public["recovery"], "envelope")
  end

  defp idempotent_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.recovery-envelope-test",
      "nodes" => [
        %{"id" => "input", "kind" => "input"},
        %{"id" => "solve", "kind" => "solve", "operator_id" => "solve.bar_1d"},
        %{"id" => "output", "kind" => "output"}
      ],
      "edges" => []
    }
  end

  defp input_artifacts, do: %{"input" => %{"value" => 1}}
end
