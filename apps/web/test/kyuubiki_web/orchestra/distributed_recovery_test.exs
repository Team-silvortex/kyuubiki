defmodule KyuubikiWeb.Orchestra.DistributedRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.DistributedRecovery

  @endpoint %{id: "agent-a", host: "127.0.0.1", port: 5001}

  test "retries a connect failure before a side-effecting task was dispatched" do
    receipt =
      DistributedRecovery.failure_receipt(
        @endpoint,
        "run_operator_task_ir",
        {:agent_transport_failure, :connect, :econnrefused},
        [retry_safety: "checkpoint_required"],
        1,
        1
      )

    assert receipt.reason_code == "agent_process_unavailable"
    assert receipt.failure_stage == "connect"
    assert receipt.process_loss
    assert receipt.retryable
    assert receipt.next_action == "retry_next_agent"
  end

  test "blocks blind replay after a side-effecting task loses its agent" do
    receipt =
      DistributedRecovery.failure_receipt(
        @endpoint,
        "run_operator_task_ir",
        {:agent_transport_failure, :receive, :closed},
        [retry_safety: "checkpoint_required"],
        1,
        1
      )

    assert receipt.reason_code == "agent_process_lost"
    assert receipt.process_loss
    refute receipt.retryable
    assert receipt.next_action == "checkpoint_before_retry"
  end

  test "allows idempotent and checkpointed work to fail over after dispatch" do
    policies = [
      [retry_safety: "idempotent"],
      [retry_safety: "checkpointed", replay_checkpoint: verified_checkpoint()]
    ]

    for policy <- policies do
      receipt =
        DistributedRecovery.failure_receipt(
          @endpoint,
          "run_operator_task_ir",
          {:agent_transport_failure, :receive, :closed},
          policy,
          1,
          2
        )

      assert receipt.retryable
      assert receipt.attempt == 2
      assert receipt.next_action == "retry_next_agent"
    end
  end

  test "rejects an unverified checkpoint assertion" do
    receipt =
      DistributedRecovery.failure_receipt(
        @endpoint,
        "run_operator_task_ir",
        {:agent_transport_failure, :receive, :closed},
        [retry_safety: "checkpointed"],
        1,
        1
      )

    refute receipt.retryable
    assert receipt.retry_safety == "checkpoint_required"
    assert receipt.checkpoint_digest == nil
  end

  test "keeps unrelated dispatch rejection out of agent health cooldown" do
    receipt =
      DistributedRecovery.failure_receipt(
        @endpoint,
        "solve_bar_1d",
        :dispatch_not_authorized,
        [],
        1,
        1
      )

    refute DistributedRecovery.agent_health_failure?(receipt)
    refute receipt.retryable
    assert receipt.reason_code == "orchestra_dispatch_rejected"
  end

  test "reports when safe work must await agent recovery" do
    receipt =
      DistributedRecovery.failure_receipt(
        @endpoint,
        "solve_bar_1d",
        {:agent_transport_failure, :receive, :timeout},
        [],
        0,
        1
      )

    assert receipt.retryable
    assert receipt.next_action == "await_agent_recovery"
  end

  defp verified_checkpoint do
    %{
      "operator_task_batch_checkpoint_verification_contract" =>
        "kyuubiki.operator_task_batch_checkpoint_verification/v1",
      "status" => "verified",
      "checkpoint_digest" => String.duplicate("a", 64)
    }
  end
end
