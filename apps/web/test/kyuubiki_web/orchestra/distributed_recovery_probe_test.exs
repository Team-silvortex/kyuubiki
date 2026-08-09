defmodule KyuubikiWeb.Orchestra.DistributedRecoveryProbeTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.DistributedRecoveryProbe

  test "retains process loss, safe failover, and replay blocking observations" do
    report = DistributedRecoveryProbe.run!()

    assert report["status"] == "pass"
    assert report["scenario_count"] == 3

    idempotent = scenario(report, "idempotent_task_process_loss_failover")
    assert idempotent["observations"]["result_retained"]
    assert idempotent["observations"]["recovery"].reason_code == "agent_process_lost"
    assert idempotent["observations"]["recovery"].retryable

    blocked = scenario(report, "side_effect_replay_blocked_without_checkpoint")
    assert blocked["observations"]["duplicate_side_effect_prevented"]
    refute blocked["observations"]["recovery"].retryable

    checkpointed = scenario(report, "checkpointed_side_effect_process_loss_failover")
    assert checkpointed["observations"]["checkpointed_result_retained"]
    assert checkpointed["observations"]["recovery"].retry_safety == "checkpointed"
    assert is_binary(checkpointed["observations"]["recovery"].checkpoint_digest)
  end

  defp scenario(report, id) do
    Enum.find(report["scenarios"], &(&1["id"] == id))
  end
end
