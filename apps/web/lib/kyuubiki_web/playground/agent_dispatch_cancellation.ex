defmodule KyuubikiWeb.Playground.AgentDispatchCancellation do
  @moduledoc "Bounded cancellation of captured RPC requests, not every execution sharing a job ID."
  alias KyuubikiWeb.Playground.{AgentExecutionCancellation, AgentExecutionGate, AgentPool}

  @max_requests 128
  @max_endpoints 32
  @budget_ms 15_000

  def policy do
    %{
      max_requests: @max_requests,
      max_endpoints: @max_endpoints,
      delivery_start_budget_ms: @budget_ms,
      scope: "captured_request_and_observed_process_generation",
      job_wide_fallback: false,
      durable_delivery: false,
      execution_terminal_confirmed: false
    }
  end

  def cancel(job_id, targets, opts \\ []) do
    with :ok <- validate_snapshot(job_id, targets) do
      configured = Keyword.get_lazy(opts, :endpoints, &AgentPool.inspection_endpoints/0)
      clock = Keyword.get(opts, :clock, fn -> System.monotonic_time(:millisecond) end)
      deadline = clock.() + @budget_ms
      probe = Keyword.get(opts, :probe, &AgentExecutionCancellation.describe/2)
      sender = Keyword.get(opts, :send_cancel, &AgentExecutionCancellation.send_endpoint/3)
      configured = MapSet.new(configured, &fingerprint/1)

      {results, _} =
        Enum.map_reduce(targets.requests, %{}, fn request, cache ->
          endpoint = request.endpoint
          key = fingerprint(endpoint)
          remaining = deadline - clock.()

          {delivery, cache} =
            cond do
              remaining <= 0 ->
                {AgentExecutionCancellation.not_sent("delivery_budget_exhausted"), cache}

              not MapSet.member?(configured, key) ->
                {AgentExecutionCancellation.not_sent("original_endpoint_not_configured"), cache}

              true ->
                description =
                  Map.get_lazy(cache, key, fn -> probe_safely(probe, endpoint, remaining) end)

                observed =
                  AgentExecutionCancellation.observe(job_id, request.request_id, description)

                delivery = deliver(endpoint, observed, sender, deadline - clock.())
                {delivery, Map.put(cache, key, description)}
            end

          {Map.merge(delivery, %{"agent_id" => endpoint.id, "request_id" => request.request_id}),
           cache}
        end)

      {:ok, receipt(job_id, targets, results)}
    end
  catch
    :exit, _ -> {:error, :execution_cancellation_unavailable}
  end

  defp deliver(_, _, _, remaining) when remaining <= 0,
    do: AgentExecutionCancellation.not_sent("delivery_budget_exhausted")

  defp deliver(endpoint, %{"execution_target" => %{} = target}, sender, remaining),
    do:
      AgentExecutionCancellation.deliver(endpoint, target, fn endpoint, target ->
        sender.(endpoint, target, remaining)
      end)

  defp deliver(_, %{"execution_target" => nil}, _, _),
    do: AgentExecutionCancellation.not_sent("execution_target_invalid")

  defp deliver(_, observation, _, _),
    do: AgentExecutionCancellation.not_sent(observation["status"])

  defp probe_safely(probe, endpoint, remaining) do
    probe.(endpoint, remaining)
  catch
    :exit, _ -> {:error, :execution_observation_unavailable}
  end

  def validate_snapshot(job_id, %{
        requests: requests,
        queued_cancelled_count: queued,
        reserved_cancelled_count: reserved
      })
      when is_list(requests) do
    cond do
      not AgentExecutionGate.valid_job_id?(job_id) ->
        {:error, :invalid_execution_job_id}

      length(requests) > @max_requests ->
        {:error, :execution_cancellation_capacity_exceeded}

      not (is_integer(queued) and queued >= 0 and is_integer(reserved) and reserved >= 0) ->
        {:error, :invalid_execution_cancellation_snapshot}

      not Enum.all?(requests, &valid_request?(&1, job_id)) ->
        {:error, :invalid_execution_cancellation_snapshot}

      MapSet.size(MapSet.new(requests, &{fingerprint(&1.endpoint), &1.request_id})) !=
          length(requests) ->
        {:error, :invalid_execution_cancellation_snapshot}

      MapSet.size(MapSet.new(requests, &fingerprint(&1.endpoint))) > @max_endpoints ->
        {:error, :execution_cancellation_capacity_exceeded}

      true ->
        :ok
    end
  end

  def validate_snapshot(_, _), do: {:error, :invalid_execution_cancellation_snapshot}

  defp valid_request?(
         %{
           endpoint: %{id: id, host: host, port: port} = endpoint,
           request_id: request,
           job_id: job
         },
         job_id
       ),
       do:
         job == job_id and AgentExecutionCancellation.valid_identity?(request) and
           AgentExecutionCancellation.valid_identity?(id) and
           AgentExecutionCancellation.valid_identity?(host) and is_integer(port) and
           port in 1..65535 and
           (is_nil(endpoint[:agent_session_id]) or
              AgentExecutionCancellation.valid_identity?(endpoint[:agent_session_id]))

  defp valid_request?(_, _), do: false
  defp fingerprint(endpoint), do: AgentExecutionCancellation.endpoint_fingerprint(endpoint)

  defp receipt(job_id, targets, results) do
    registered = Enum.count(results, &(&1["cancel_registered"] == true))

    status =
      cond do
        results == [] and targets.queued_cancelled_count + targets.reserved_cancelled_count > 0 ->
          "cancelled_before_dispatch"

        results == [] ->
          "no_active_dispatch"

        registered == length(results) ->
          "requested"

        registered > 0 ->
          "partially_requested"

        Enum.any?(results, &(&1["status"] == "cancellation_outcome_unknown")) ->
          "cancellation_outcome_unknown"

        true ->
          "not_requested"
      end

    %{
      "schema_version" => "kyuubiki.orchestra-dispatch-cancellation/v1",
      "job_id" => job_id,
      "status" => status,
      "request_count" => length(results),
      "registered_count" => registered,
      "queued_cancelled_count" => targets.queued_cancelled_count,
      "reserved_cancelled_count" => targets.reserved_cancelled_count,
      "targets" => results,
      "execution_terminal_confirmed" => false,
      "automatic_replay_authorized" => false,
      "pending_cancellation_created" => false,
      "job_wide_fallback_performed" => false,
      "policy" => policy()
    }
  end
end
