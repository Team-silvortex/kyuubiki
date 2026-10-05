defmodule KyuubikiWeb.Playground.AgentJobCancellation do
  @moduledoc false

  alias KyuubikiWeb.Playground.AgentExecutionGate

  def finish_dispatch(lease_id, result) do
    case AgentExecutionGate.authorize_dispatch(lease_id, false) do
      :ok ->
        result

      {:error, _} = cancelled ->
        case result do
          {:error, {:operator_task_rpc_error, _, _, _}} -> result
          _ -> cancelled
        end
    end
  end

  def stop_dispatch(task, job_id) do
    with {:ok, targets} <- AgentExecutionGate.cancel_job_targets(job_id) do
      # Capture ownership before the caller DOWN signal removes its capacity lease.
      {:ok, Task.shutdown(task, :brutal_kill), targets}
    end
  end

  def cancel(job_id, send_cancel) do
    with {:ok, targets} <- AgentExecutionGate.cancel_job_targets(job_id) do
      cancel_targets(job_id, targets, send_cancel)
    end
  end

  def cancel_targets(job_id, targets, send_cancel) do
    if AgentExecutionGate.valid_job_id?(job_id) do
      results = Enum.map(targets.endpoints, &deliver(job_id, &1, send_cancel))
      registered = Enum.count(results, & &1["cancel_registered"])
      local = targets.queued_cancelled_count + targets.reserved_cancelled_count

      status =
        cond do
          results == [] and local > 0 -> "cancelled_before_dispatch"
          results == [] -> "no_active_dispatch"
          registered == length(results) -> "requested"
          registered > 0 -> "partially_requested"
          true -> "delivery_failed"
        end

      {:ok,
       %{
         "schema_version" => "kyuubiki.orchestra-job-cancellation/v1",
         "job_id" => job_id,
         "status" => status,
         "execution_terminal_confirmed" => false,
         "target_count" => length(results),
         "registered_count" => registered,
         "queued_cancelled_count" => targets.queued_cancelled_count,
         "reserved_cancelled_count" => targets.reserved_cancelled_count,
         "targets" => results
       }}
    else
      {:error, :invalid_execution_job_id}
    end
  end

  defp deliver(job_id, endpoint, send_cancel) do
    base = %{"agent_id" => endpoint.id, "cancel_registered" => false}

    case send_cancel.(endpoint) do
      {:ok, result} when is_map(result) ->
        registered = Map.get(result, "cancel_registered", result["cancelled"]) == true

        identity_matches =
          case result["schema_version"] do
            nil -> Map.get(result, "job_id", job_id) == job_id
            "kyuubiki.agent-job-cancellation/v1" -> result["job_id"] == job_id
            _ -> false
          end

        if registered and identity_matches do
          Map.merge(base, %{
            "cancel_registered" => true,
            "operator_package_job_release" => result["operator_package_job_release"]
          })
        else
          Map.put(base, "error_code", "invalid_cancellation_acknowledgement")
        end

      {:error, {:rpc_error, code, _message}} ->
        Map.put(base, "error_code", code)

      _ ->
        Map.put(base, "error_code", "cancellation_delivery_failed")
    end
  end
end
