defmodule KyuubikiWeb.Orchestra.OperatorDispatchInspection do
  @moduledoc "Read-only original-target observation; absence never proves completion or cancellation."
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Playground.AgentExecutionCancellation

  @schema "kyuubiki.operator-task-dispatch-inspection/v1"
  @max_probes 4
  @max_attempts 128

  def inspect_task(payload, opts \\ []) do
    with :ok <- validate_query(payload),
         {:ok, records} <-
           Journal.lookup(
             payload["task_id"],
             payload["task_digest"],
             Keyword.get(opts, :journal, Journal)
           ) do
      endpoints = Keyword.get_lazy(opts, :endpoints, &AgentPool.inspection_endpoints/0)
      probe = Keyword.get(opts, :probe, &AgentExecutionCancellation.describe/1)
      attempts = Enum.take(records, @max_attempts)

      {observations, _cache} =
        Enum.map_reduce(attempts, %{}, fn record, cache ->
          {observation, cache} = observe(record, endpoints, probe, cache)

          {Map.take(record, ~w(attempt_id request_id state created_at_ms updated_at_ms))
           |> Map.put("job_id", Files.job_id(record))
           |> Map.put("observation", observation), cache}
        end)

      {:ok,
       %{
         "schema_version" => @schema,
         "task_id" => payload["task_id"],
         "task_digest" => payload["task_digest"],
         "status" =>
           if(records == [], do: "no_retained_dispatch", else: "dispatch_records_found"),
         "retained_attempt_count" => length(records),
         "truncated" => length(records) > @max_attempts,
         "attempts" => observations,
         "automatic_replay_authorized" => false,
         "terminal_result_available" => false,
         "journal_policy" => Journal.policy(),
         "inspection_policy" => %{
           max_probes: @max_probes,
           max_attempts: @max_attempts,
           request_timeout_ms: 2_000,
           connect_timeout_ms: 1_500,
           max_rpc_frame_bytes: 1_048_576
         }
       }}
    end
  catch
    :exit, _reason -> {:error, :operator_task_dispatch_inspection_unavailable}
  end

  defp validate_query(%{"task_id" => id, "task_digest" => digest} = payload)
       when is_binary(id) and byte_size(id) in 1..1024 and is_binary(digest) do
    if Enum.sort(Map.keys(payload)) == ~w(task_digest task_id) and String.valid?(id) and
         byte_size(digest) == 64 and Regex.match?(~r/\A[0-9a-f]{64}\z/, digest) do
      :ok
    else
      {:error, :operator_task_dispatch_query_invalid}
    end
  end

  defp validate_query(_payload), do: {:error, :operator_task_dispatch_query_invalid}

  defp observe(%{"state" => state}, _endpoints, _probe, cache)
       when state in ~w(observed_executed observed_failed observed_blocked not_dispatched),
       do: {%{"status" => "retained_dispatch_observation", "outcome" => state}, cache}

  defp observe(record, endpoints, probe, cache) do
    fingerprint = record["endpoint_fingerprint"]
    endpoint = Enum.find(endpoints, &(Files.fingerprint(&1) == fingerprint))

    cond do
      is_nil(endpoint) ->
        {%{"status" => "original_endpoint_not_configured", "outcome" => "unknown"}, cache}

      Map.has_key?(cache, fingerprint) ->
        {observation(record, cache[fingerprint]), cache}

      map_size(cache) >= @max_probes ->
        {%{"status" => "probe_limit_reached", "outcome" => "unknown"}, cache}

      true ->
        result = probe.(endpoint)
        {observation(record, result), Map.put(cache, fingerprint, result)}
    end
  end

  defp observation(record, result),
    do: AgentExecutionCancellation.observe(Files.job_id(record), record["request_id"], result)
end
