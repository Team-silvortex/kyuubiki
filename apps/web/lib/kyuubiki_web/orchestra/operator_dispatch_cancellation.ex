defmodule KyuubikiWeb.Orchestra.OperatorDispatchCancellation do
  @moduledoc "Explicit observed-generation cancellation through one retained original endpoint."
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Playground.{AgentPool, AgentRpcTransport}

  @schema "kyuubiki.operator-task-dispatch-cancellation/v1"
  @agent_schema "kyuubiki.agent-execution-cancellation/v1"
  @target_fields ~w(generation job_id process_instance_id request_id)
  @ack_fields ~w(automatic_replay_authorized cancel_registered execution_target execution_terminal_confirmed operator_package_cleanup_performed pending_cancellation_created schema_version status)
  @terminal_states ~w(observed_executed observed_failed observed_blocked)

  def cancel(payload, opts \\ []) do
    with :ok <- validate_query(payload),
         {:ok, records} <-
           Journal.lookup(
             payload["task_id"],
             payload["task_digest"],
             Keyword.get(opts, :journal, Journal)
           ),
         record <- Enum.find(records, &(&1["attempt_id"] == payload["attempt_id"])),
         :ok <- match_record(record, payload["execution_target"]) do
      endpoints = Keyword.get_lazy(opts, :endpoints, &AgentPool.inspection_endpoints/0)
      send_cancel = Keyword.get(opts, :send_cancel, &send_endpoint/2)
      delivery = deliver(record, payload["execution_target"], endpoints, send_cancel)

      {:ok,
       Map.merge(delivery, %{
         "schema_version" => @schema,
         "task_id" => payload["task_id"],
         "task_digest" => payload["task_digest"],
         "attempt_id" => payload["attempt_id"],
         "execution_target" => payload["execution_target"],
         "execution_terminal_confirmed" => false,
         "automatic_replay_authorized" => false,
         "journal_mutation_performed" => false,
         "publication_performed" => false,
         "job_wide_fallback_performed" => false
       })}
    end
  catch
    :exit, _ -> {:error, :operator_task_dispatch_cancellation_unavailable}
  end

  defp validate_query(%{"execution_target" => target} = payload) when is_map(target) do
    if Enum.sort(Map.keys(payload)) == ~w(attempt_id execution_target task_digest task_id) and
         Enum.sort(Map.keys(target)) == @target_fields and
         Enum.all?(~w(job_id process_instance_id request_id), &valid_identity?(target[&1])) and
         target["process_instance_id"] != "unavailable" and
         is_integer(target["generation"]) and
         target["generation"] in 1..18_446_744_073_709_551_615 and
         target["job_id"] == payload["task_id"] and
         hex?(payload["task_digest"], 64) and hex?(payload["attempt_id"], 32) do
      :ok
    else
      {:error, :operator_task_dispatch_query_invalid}
    end
  end

  defp validate_query(_), do: {:error, :operator_task_dispatch_query_invalid}

  defp valid_identity?(value) when is_binary(value) and byte_size(value) in 1..256 do
    String.valid?(value) and String.trim(value) != "" and
      not Regex.match?(~r/[\x{0000}-\x{001f}\x{007f}-\x{009f}]/u, value)
  end

  defp valid_identity?(_), do: false

  defp hex?(value, size) when is_binary(value),
    do: byte_size(value) == size and Regex.match?(~r/\A[0-9a-f]+\z/, value)

  defp hex?(_, _), do: false

  defp match_record(nil, _), do: :ok

  defp match_record(record, target) do
    if record["request_id"] == target["request_id"] and record["task_id"] == target["job_id"],
      do: :ok,
      else: {:error, :operator_task_dispatch_target_mismatch}
  end

  defp deliver(nil, _, _, _), do: not_sent("no_retained_dispatch")
  defp deliver(%{"state" => "not_dispatched"}, _, _, _), do: not_sent("not_dispatched")

  defp deliver(%{"state" => state}, _, _, _) when state in @terminal_states,
    do: not_sent("retained_terminal_dispatch")

  defp deliver(record, target, endpoints, send_cancel) do
    case Enum.find(endpoints, &(Files.fingerprint(&1) == record["endpoint_fingerprint"])) do
      nil -> not_sent("original_endpoint_not_configured")
      endpoint -> verify(target, send_cancel.(endpoint, target))
    end
  end

  defp verify(target, {:ok, %{} = ack}) do
    status = ack["status"]

    if Enum.sort(Map.keys(ack)) == @ack_fields and ack["schema_version"] == @agent_schema and
         ack["execution_target"] == target and status in ~w(requested target_not_observed) and
         ack["cancel_registered"] == (status == "requested") and
         Enum.all?(
           ~w(execution_terminal_confirmed pending_cancellation_created operator_package_cleanup_performed automatic_replay_authorized),
           &(ack[&1] == false)
         ) do
      %{
        "status" => status,
        "cancel_registered" => ack["cancel_registered"],
        "delivery_attempted" => true,
        "agent_acknowledgement" => ack,
        "uncertainty_reason" => nil
      }
    else
      unknown("agent_acknowledgement_invalid")
    end
  end

  defp verify(_, _), do: unknown("original_endpoint_unreachable")

  defp not_sent(status),
    do: %{
      "status" => status,
      "cancel_registered" => false,
      "delivery_attempted" => false,
      "agent_acknowledgement" => nil,
      "uncertainty_reason" => nil
    }

  defp unknown(reason),
    do: %{
      "status" => "cancellation_outcome_unknown",
      "cancel_registered" => nil,
      "delivery_attempted" => true,
      "agent_acknowledgement" => nil,
      "uncertainty_reason" => reason
    }

  defp send_endpoint(endpoint, target) do
    id = :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower)

    request = %{
      "rpc_version" => 1,
      "id" => id,
      "method" => "cancel_execution",
      "params" => target
    }

    AgentRpcTransport.request(endpoint, id, request, fn _ -> :ok end,
      request_timeout_ms: 5_000,
      connect_timeout_ms: 1_500,
      max_rpc_frame_bytes: 65_536
    )
  end
end
