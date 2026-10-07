defmodule KyuubikiWeb.Orchestra.OperatorDispatchInspection do
  @moduledoc "Read-only original-target observation; absence never proves completion or cancellation."
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Playground.AgentRpcTransport

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
      probe = Keyword.get(opts, :probe, &probe_endpoint/1)
      attempts = Enum.take(records, @max_attempts)

      {observations, _cache} =
        Enum.map_reduce(attempts, %{}, fn record, cache ->
          {observation, cache} = observe(record, endpoints, probe, cache)

          {Map.take(record, ~w(attempt_id request_id state created_at_ms updated_at_ms))
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

  defp observation(record, {:ok, %{"solver_control" => control, "lifecycle" => lifecycle}})
       when is_map(control) and is_map(lifecycle) do
    active = control["active"]
    process_id = lifecycle["process_instance_id"]

    if control["schema_version"] == "kyuubiki.agent-solver-control/v1" and
         control["available"] == true and is_list(active) and length(active) <= 1024 and
         is_binary(process_id) and byte_size(process_id) in 1..256 and process_id != "unavailable" do
      matching = Enum.filter(active, &(is_map(&1) and &1["request_id"] == record["request_id"]))

      case matching do
        [%{"generation" => generation, "cancel_requested" => cancelled} = entry]
        when is_integer(generation) and generation > 0 and is_boolean(cancelled) ->
          %{
            "status" => "original_endpoint_reports_active_request",
            "outcome" => "unknown",
            "process_instance_id" => process_id,
            "generation" => generation,
            "cancel_requested" => cancelled,
            "execution_target" => execution_target(record, entry, process_id, generation)
          }

        [] ->
          %{"status" => "request_not_observed_active", "outcome" => "unknown"}

        _ ->
          %{"status" => "agent_observation_invalid", "outcome" => "unknown"}
      end
    else
      %{"status" => "agent_observation_invalid", "outcome" => "unknown"}
    end
  end

  defp observation(_record, {:ok, _}),
    do: %{"status" => "agent_observation_invalid", "outcome" => "unknown"}

  defp observation(_record, _),
    do: %{"status" => "original_endpoint_unreachable", "outcome" => "unknown"}

  defp execution_target(record, entry, process_id, generation) do
    target = %{
      "process_instance_id" => process_id,
      "request_id" => record["request_id"],
      "generation" => generation,
      "job_id" => entry["job_id"]
    }

    if entry["job_id"] == record["task_id"] and
         Enum.all?(~w(process_instance_id request_id job_id), fn field ->
           value = target[field]

           is_binary(value) and byte_size(value) in 1..256 and String.valid?(value) and
             String.trim(value) != "" and
             not Regex.match?(~r/[\x{0000}-\x{001f}\x{007f}-\x{009f}]/u, value)
         end) and generation <= 18_446_744_073_709_551_615,
       do: target,
       else: nil
  end

  defp probe_endpoint(endpoint) do
    id = :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower)
    request = %{"rpc_version" => 1, "id" => id, "method" => "describe_agent", "params" => %{}}

    AgentRpcTransport.request(endpoint, id, request, fn _ -> :ok end,
      request_timeout_ms: 2_000,
      connect_timeout_ms: 1_500,
      max_rpc_frame_bytes: 1_048_576
    )
  end
end
