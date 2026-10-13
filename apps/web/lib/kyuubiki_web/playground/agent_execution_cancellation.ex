defmodule KyuubikiWeb.Playground.AgentExecutionCancellation do
  @moduledoc "Exact observed-generation cancellation; never creates a pending job cancellation."
  alias KyuubikiWeb.Playground.AgentRpcTransport
  alias KyuubikiWeb.Storage.DurableJson

  @schema "kyuubiki.agent-execution-cancellation/v1"
  @target_fields ~w(generation job_id process_instance_id request_id)
  @ack_fields ~w(automatic_replay_authorized cancel_registered execution_target execution_terminal_confirmed operator_package_cleanup_performed pending_cancellation_created schema_version status)

  def endpoint_fingerprint(endpoint) do
    bytes =
      DurableJson.encode!([
        endpoint[:id],
        endpoint[:host],
        endpoint[:port],
        endpoint[:agent_session_id]
      ])

    :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)
  end

  def valid_target?(target) when is_map(target) do
    Enum.sort(Map.keys(target)) == @target_fields and
      Enum.all?(~w(job_id process_instance_id request_id), &valid_identity?(target[&1])) and
      target["process_instance_id"] != "unavailable" and is_integer(target["generation"]) and
      target["generation"] in 1..18_446_744_073_709_551_615
  end

  def valid_target?(_), do: false

  def valid_identity?(value) when is_binary(value) and byte_size(value) in 1..256,
    do:
      String.valid?(value) and String.trim(value) != "" and
        not Regex.match?(~r/[\x{0000}-\x{001f}\x{007f}-\x{009f}]/u, value)

  def valid_identity?(_), do: false

  def observe(job_id, request_id, {:ok, %{"solver_control" => control, "lifecycle" => lifecycle}})
      when is_map(control) and is_map(lifecycle) do
    active = control["active"]
    process_id = lifecycle["process_instance_id"]

    if control["schema_version"] == "kyuubiki.agent-solver-control/v1" and
         control["available"] == true and is_list(active) and length(active) <= 1024 and
         valid_identity?(process_id) and process_id != "unavailable" do
      case Enum.filter(active, &(is_map(&1) and &1["request_id"] == request_id)) do
        [%{"generation" => generation, "cancel_requested" => cancelled} = entry]
        when is_integer(generation) and generation > 0 and is_boolean(cancelled) ->
          target = %{
            "process_instance_id" => process_id,
            "request_id" => request_id,
            "job_id" => entry["job_id"],
            "generation" => generation
          }

          %{
            "status" => "original_endpoint_reports_active_request",
            "outcome" => "unknown",
            "process_instance_id" => process_id,
            "generation" => generation,
            "cancel_requested" => cancelled,
            "execution_target" =>
              if(entry["job_id"] == job_id and valid_target?(target), do: target)
          }

        [] ->
          observation("request_not_observed_active")

        _ ->
          observation("agent_observation_invalid")
      end
    else
      observation("agent_observation_invalid")
    end
  end

  def observe(_, _, {:ok, _}), do: observation("agent_observation_invalid")
  def observe(_, _, _), do: observation("original_endpoint_unreachable")
  defp observation(status), do: %{"status" => status, "outcome" => "unknown"}

  def deliver(endpoint, target, sender \\ &send_endpoint/2) do
    if valid_target?(target),
      do: verify(target, sender.(endpoint, target)),
      else: not_sent("execution_target_invalid")
  catch
    :exit, _ -> unknown("original_endpoint_unreachable")
  end

  defp verify(target, {:ok, %{} = ack}) do
    status = ack["status"]

    if Enum.sort(Map.keys(ack)) == @ack_fields and ack["schema_version"] == @schema and
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

  def not_sent(status),
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

  def describe(endpoint, timeout_ms \\ 2_000),
    do: rpc(endpoint, "describe_agent", %{}, min(timeout_ms, 2_000), 1_048_576)

  def send_endpoint(endpoint, target, timeout_ms \\ 5_000),
    do: rpc(endpoint, "cancel_execution", target, min(timeout_ms, 5_000), 65_536)

  defp rpc(endpoint, method, params, timeout_ms, max_bytes) do
    id = :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower)
    request = %{"rpc_version" => 1, "id" => id, "method" => method, "params" => params}

    AgentRpcTransport.request(endpoint, id, request, fn _ -> :ok end,
      request_timeout_ms: timeout_ms,
      connect_timeout_ms: min(timeout_ms, 1_500),
      max_rpc_frame_bytes: max_bytes
    )
  end
end
