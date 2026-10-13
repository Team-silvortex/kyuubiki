defmodule KyuubikiWeb.Orchestra.OperatorDispatchCancellation do
  @moduledoc "Explicit observed-generation cancellation through one retained original endpoint."
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Playground.{AgentPool, AgentExecutionCancellation}

  @schema "kyuubiki.operator-task-dispatch-cancellation/v1"
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
      send_cancel = Keyword.get(opts, :send_cancel, &AgentExecutionCancellation.send_endpoint/2)
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
         AgentExecutionCancellation.valid_target?(target) and
         valid_task_id?(payload["task_id"]) and
         hex?(payload["task_digest"], 64) and hex?(payload["attempt_id"], 32) do
      :ok
    else
      {:error, :operator_task_dispatch_query_invalid}
    end
  end

  defp validate_query(_), do: {:error, :operator_task_dispatch_query_invalid}

  defp valid_task_id?(value) when is_binary(value) and byte_size(value) in 1..1024,
    do: String.valid?(value)

  defp valid_task_id?(_), do: false

  defp hex?(value, size) when is_binary(value),
    do: byte_size(value) == size and Regex.match?(~r/\A[0-9a-f]+\z/, value)

  defp hex?(_, _), do: false

  defp match_record(nil, _), do: :ok

  defp match_record(record, target) do
    cond do
      is_nil(Files.job_id(record)) ->
        {:error, :operator_task_dispatch_job_identity_unknown}

      record["request_id"] == target["request_id"] and
          Files.job_id(record) == target["job_id"] ->
        :ok

      true ->
        {:error, :operator_task_dispatch_target_mismatch}
    end
  end

  defp deliver(nil, _, _, _), do: not_sent("no_retained_dispatch")
  defp deliver(%{"state" => "not_dispatched"}, _, _, _), do: not_sent("not_dispatched")

  defp deliver(%{"state" => state}, _, _, _) when state in @terminal_states,
    do: not_sent("retained_terminal_dispatch")

  defp deliver(record, target, endpoints, send_cancel) do
    case Enum.find(endpoints, &(Files.fingerprint(&1) == record["endpoint_fingerprint"])) do
      nil -> not_sent("original_endpoint_not_configured")
      endpoint -> AgentExecutionCancellation.deliver(endpoint, target, send_cancel)
    end
  end

  defp not_sent(status), do: AgentExecutionCancellation.not_sent(status)
end
