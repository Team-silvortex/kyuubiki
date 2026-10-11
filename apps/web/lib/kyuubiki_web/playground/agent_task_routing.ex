defmodule KyuubikiWeb.Playground.AgentTaskRouting do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  @control_opts [
    :job_id,
    :queue_timeout_ms,
    :request_timeout_ms,
    :orchestration,
    :before_dispatch
  ]

  def for_task(task_ir, opts) do
    task_ir
    |> OperatorTaskIR.agent_routing_opts()
    |> Keyword.merge(Keyword.take(opts, @control_opts))
    |> maybe_override_retry_policy(opts)
    |> maybe_require_operator_package_runtime(task_ir, Keyword.get(opts, :mode))
  end

  defp maybe_override_retry_policy(routing_opts, opts) do
    retry_safety = Keyword.get(opts, :retry_safety)
    replay_checkpoint = Keyword.get(opts, :replay_checkpoint)

    cond do
      retry_safety in [:checkpoint_required, "checkpoint_required"] ->
        Keyword.put(routing_opts, :retry_safety, retry_safety)

      retry_safety in [:idempotent, "idempotent"] ->
        Keyword.put(routing_opts, :retry_safety, retry_safety)

      retry_safety in [:checkpointed, "checkpointed"] and
          verified_replay_checkpoint?(replay_checkpoint) ->
        routing_opts
        |> Keyword.put(:retry_safety, retry_safety)
        |> Keyword.put(:replay_checkpoint, replay_checkpoint)

      true ->
        routing_opts
    end
  end

  defp verified_replay_checkpoint?(%{
         "operator_task_batch_checkpoint_verification_contract" =>
           "kyuubiki.operator_task_batch_checkpoint_verification/v1",
         "status" => "verified",
         "checkpoint_digest" => digest
       })
       when is_binary(digest),
       do: Regex.match?(~r/\A[0-9a-f]{64}\z/, digest)

  defp verified_replay_checkpoint?(_checkpoint), do: false

  defp maybe_require_operator_package_runtime(routing_opts, task_ir, mode)
       when mode in [:execute, "execute"] do
    unbundled_native? =
      get_in(task_ir, ["runtime_hints", "execution_mode"]) == "agent_native" and
        get_in(task_ir, ["runtime_hints", "agent_fetchable"]) == false and
        get_in(task_ir, ["execution_program", "package_ref"]) in [nil, ""]

    if unbundled_native?,
      do: routing_opts,
      else: Keyword.put(routing_opts, :requires_operator_package_runtime, true)
  end

  defp maybe_require_operator_package_runtime(routing_opts, _task_ir, _mode), do: routing_opts
end
