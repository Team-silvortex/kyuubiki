defmodule KyuubikiWeb.Orchestra.OperatorDispatchResult do
  @moduledoc "Read-only receipt retrieval for one retained dispatch; never executes or publishes a job."
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorTaskCompletion
  alias KyuubikiWeb.Orchestra.OperatorTaskFailure
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Playground.AgentRpcTransport

  @schema "kyuubiki.operator-task-dispatch-result/v1"
  @agent_schema "kyuubiki.agent-task-result-retention/v1"
  @identities ~w(attempt_id request_id task_id task_digest operator_id program_id)

  def fetch(payload, opts \\ []) do
    with :ok <- validate_query(payload),
         {:ok, records} <-
           Journal.lookup(
             payload["task_id"],
             payload["task_digest"],
             Keyword.get(opts, :journal, Journal)
           ) do
      record = Enum.find(records, &(&1["attempt_id"] == payload["attempt_id"]))
      endpoints = Keyword.get_lazy(opts, :endpoints, &AgentPool.inspection_endpoints/0)
      probe = Keyword.get(opts, :probe, &probe_endpoint/2)
      result = retrieve(record, endpoints, probe)

      {:ok,
       Map.merge(result, %{
         "schema_version" => @schema,
         "task_id" => payload["task_id"],
         "task_digest" => payload["task_digest"],
         "attempt_id" => payload["attempt_id"],
         "automatic_replay_authorized" => false,
         "publication_performed" => false
       })}
    end
  catch
    :exit, _ -> {:error, :operator_task_dispatch_result_unavailable}
  end

  defp validate_query(
         %{"task_id" => id, "task_digest" => digest, "attempt_id" => attempt} = payload
       )
       when is_binary(id) and byte_size(id) in 1..1024 and is_binary(digest) and
              is_binary(attempt) do
    if Enum.sort(Map.keys(payload)) == ~w(attempt_id task_digest task_id) and String.valid?(id) and
         Regex.match?(~r/\A[0-9a-f]{64}\z/, digest) and
         Regex.match?(~r/\A[0-9a-f]{32}\z/, attempt),
       do: :ok,
       else: {:error, :operator_task_dispatch_query_invalid}
  end

  defp validate_query(_), do: {:error, :operator_task_dispatch_query_invalid}

  defp retrieve(nil, _, _), do: unknown("no_retained_dispatch")
  defp retrieve(%{"state" => "not_dispatched"}, _, _), do: unknown("not_dispatched")

  defp retrieve(record, endpoints, probe) do
    case Enum.find(endpoints, &(Files.fingerprint(&1) == record["endpoint_fingerprint"])) do
      nil -> unknown("original_endpoint_not_configured")
      endpoint -> verify(record, probe.(endpoint, Map.take(record, @identities)))
    end
  end

  defp verify(record, {:ok, %{} = payload}) do
    identity_ok = Enum.all?(@identities, &(payload[&1] == record[&1]))
    process = payload["process_instance_id"]
    generation = payload["generation"]

    cond do
      payload["schema_version"] != @agent_schema or not identity_ok or
        payload["automatic_replay_authorized"] != false or not is_binary(process) or
        byte_size(process) not in 1..256 or process == "unavailable" ->
        unknown("agent_receipt_invalid")

      payload["status"] == "receipt_retained" and is_integer(generation) and generation > 0 ->
        completed(record, payload)

      payload["status"] in ~w(pending not_retained result_not_retained attempt_identity_ambiguous) and
          is_nil(payload["response"]) ->
        unknown(payload["status"])

      true ->
        unknown("agent_receipt_invalid")
    end
  end

  defp verify(_, _), do: unknown("original_endpoint_unreachable")

  defp completed(record, payload) do
    summary = Map.take(record, ~w(task_id task_digest operator_id program_id))
    response = payload["response"]

    with %{"rpc_version" => 1, "id" => id} <- response,
         true <- id == record["request_id"],
         {:ok, receipt} <- unwrap(summary, response),
         {:ok, completion} <- OperatorTaskCompletion.agent_receipt(summary, receipt) do
      %{
        "status" => "receipt_recovered",
        "outcome" => completion["status"],
        "request_id" => record["request_id"],
        "process_instance_id" => payload["process_instance_id"],
        "generation" => payload["generation"],
        "completion" => completion
      }
    else
      _ -> unknown("agent_receipt_invalid")
    end
  end

  defp unwrap(_, %{"ok" => true, "result" => %{} = result} = response) do
    if is_nil(response["error"]), do: {:ok, result}, else: {:error, :invalid}
  end

  defp unwrap(
         summary,
         %{
           "ok" => false,
           "error" => %{"code" => code, "message" => message, "details" => details}
         } = response
       )
       when is_map(details) do
    if is_nil(response["result"]),
      do:
        OperatorTaskFailure.agent_rpc_result(
          summary,
          code,
          message,
          details["operator_task_failure_receipt"]
        ),
      else: {:error, :invalid}
  end

  defp unwrap(_, _), do: {:error, :invalid}

  defp unknown(status), do: %{"status" => status, "outcome" => "unknown", "completion" => nil}

  defp probe_endpoint(endpoint, query) do
    id = :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower)

    request = %{
      "rpc_version" => 1,
      "id" => id,
      "method" => "fetch_operator_task_result",
      "params" => query
    }

    AgentRpcTransport.request(endpoint, id, request, fn _ -> :ok end,
      request_timeout_ms: 10_000,
      connect_timeout_ms: 1_500,
      max_rpc_frame_bytes: 9 * 1024 * 1024
    )
  end
end
