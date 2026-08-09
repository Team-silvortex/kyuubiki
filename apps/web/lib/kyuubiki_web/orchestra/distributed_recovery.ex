defmodule KyuubikiWeb.Orchestra.DistributedRecovery do
  @moduledoc """
  Classifies agent transport failures and decides whether Orchestra may replay work.
  """

  @schema_version "kyuubiki.orchestra-agent-recovery/v1"
  @idempotent_methods ["ping", "describe_agent", "cancel_job"]
  @process_loss_reasons [:closed, :econnrefused, :econnreset, :enetdown, :enetunreach, :enotconn]

  @spec failure_receipt(map(), String.t(), term(), keyword(), non_neg_integer(), pos_integer()) ::
          map()
  def failure_receipt(endpoint, method, reason, opts, remaining_count, attempt)
      when is_map(endpoint) and is_binary(method) and is_list(opts) do
    {stage, transport_reason} = failure_stage(reason)
    checkpoint_digest = verified_checkpoint_digest(opts)
    retry_safety = retry_safety(method, opts, checkpoint_digest)
    retryable = retryable?(stage, retry_safety)

    %{
      schema_version: @schema_version,
      agent: "rust-agent-rpc@#{endpoint.id}",
      agent_id: endpoint.id,
      method: method,
      attempt: attempt,
      failure_stage: Atom.to_string(stage),
      reason_code: reason_code(stage, transport_reason),
      reason: inspect(transport_reason),
      process_loss: process_loss?(stage, transport_reason),
      retry_safety: Atom.to_string(retry_safety),
      checkpoint_digest: checkpoint_digest,
      retryable: retryable,
      remaining_agent_count: remaining_count,
      safe_to_continue_other_tasks: true,
      next_action: next_action(retryable, remaining_count)
    }
  end

  @spec retryable?(map()) :: boolean()
  def retryable?(%{retryable: retryable}), do: retryable == true
  def retryable?(_receipt), do: false

  @spec agent_health_failure?(map()) :: boolean()
  def agent_health_failure?(%{failure_stage: stage}) when stage in ["connect", "send", "receive"],
    do: true

  def agent_health_failure?(%{failure_stage: "protocol"}), do: true
  def agent_health_failure?(_receipt), do: false

  @spec health_reason(term()) :: term()
  def health_reason({:agent_transport_failure, _stage, reason}), do: reason
  def health_reason(reason), do: reason

  defp failure_stage({:agent_transport_failure, stage, reason})
       when stage in [:connect, :send, :receive, :protocol],
       do: {stage, reason}

  defp failure_stage(reason), do: {:dispatch, reason}

  defp retry_safety(method, opts, checkpoint_digest) do
    case Keyword.get(opts, :retry_safety) do
      value when value in [:idempotent, "idempotent"] ->
        :idempotent

      value when value in [:checkpointed, "checkpointed"] and is_binary(checkpoint_digest) ->
        :checkpointed

      value when value in [:checkpoint_required, "checkpoint_required"] ->
        :checkpoint_required

      _ ->
        default_retry_safety(method)
    end
  end

  defp verified_checkpoint_digest(opts) do
    case Keyword.get(opts, :replay_checkpoint) do
      %{
        "operator_task_batch_checkpoint_verification_contract" =>
          "kyuubiki.operator_task_batch_checkpoint_verification/v1",
        "status" => "verified",
        "checkpoint_digest" => digest
      }
      when is_binary(digest) ->
        if valid_digest?(digest), do: digest, else: nil

      _ ->
        nil
    end
  end

  defp valid_digest?(digest), do: Regex.match?(~r/\A[0-9a-f]{64}\z/, digest)

  defp default_retry_safety("solve_" <> _method), do: :idempotent
  defp default_retry_safety(method) when method in @idempotent_methods, do: :idempotent
  defp default_retry_safety(_method), do: :checkpoint_required

  defp retryable?(:connect, _retry_safety), do: true

  defp retryable?(stage, retry_safety)
       when stage in [:send, :receive, :protocol] and retry_safety in [:idempotent, :checkpointed],
       do: true

  defp retryable?(_stage, _retry_safety), do: false

  defp process_loss?(:connect, reason), do: reason in @process_loss_reasons

  defp process_loss?(stage, reason) when stage in [:send, :receive],
    do: reason in @process_loss_reasons

  defp process_loss?(_stage, _reason), do: false

  defp reason_code(:connect, reason) when reason in @process_loss_reasons,
    do: "agent_process_unavailable"

  defp reason_code(stage, reason)
       when stage in [:send, :receive] and reason in @process_loss_reasons,
       do: "agent_process_lost"

  defp reason_code(stage, reason)
       when stage in [:connect, :send, :receive] and reason in [:timeout, :request_timeout],
       do: "agent_transport_timeout"

  defp reason_code(:protocol, _reason), do: "agent_protocol_failure"
  defp reason_code(:dispatch, _reason), do: "orchestra_dispatch_rejected"
  defp reason_code(stage, _reason), do: "agent_transport_#{stage}_failed"

  defp next_action(false, _remaining_count), do: "checkpoint_before_retry"
  defp next_action(true, 0), do: "await_agent_recovery"
  defp next_action(true, _remaining_count), do: "retry_next_agent"
end
