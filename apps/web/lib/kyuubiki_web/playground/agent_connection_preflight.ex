defmodule KyuubikiWeb.Playground.AgentConnectionPreflight do
  @moduledoc false
  alias KyuubikiWeb.Playground.{AgentExecutionGate, AgentPool, AgentRpcTransport}

  @timeout_ms 1_500
  @max_frame_bytes 1024 * 1024
  @schema "kyuubiki.agent-connection-access/v1"

  def check(endpoint, lease_id, request, opts) do
    if required?(endpoint, request) do
      with :ok <- AgentExecutionGate.authorize_dispatch(lease_id, false) do
        result = probe(endpoint, opts)
        # A cancellation while probing must win even when the peer is ineligible.
        with :ok <- AgentExecutionGate.authorize_dispatch(lease_id, false), do: result
      end
    else
      :ok
    end
  end

  def required?(endpoint, %{"method" => method} = request) when is_binary(method) do
    new_work =
      String.starts_with?(method, "solve_") or
        (method == "run_operator_task_ir" and get_in(request, ["params", "mode"]) == "execute")

    new_work and not literal_loopback?(Map.get(endpoint, :host))
  end

  def required?(_endpoint, _request), do: false

  # Do not trust localhost spelling, DNS results or advertised authority as a bypass.
  defp literal_loopback?(host) when is_binary(host) do
    case :inet.parse_strict_address(String.to_charlist(host)) do
      {:ok, {127, _, _, _}} -> true
      {:ok, {0, 0, 0, 0, 0, 0, 0, 1}} -> true
      {:ok, {0, 0, 0, 0, 0, 65_535, high, _}} when high in 32_512..32_767 -> true
      _ -> false
    end
  end

  defp literal_loopback?(_host), do: false

  defp probe(endpoint, opts) do
    timeout = min(@timeout_ms, AgentRpcTransport.request_timeout_ms(opts))

    timeout =
      case Keyword.get(opts, :capacity_retry_deadline_ms) do
        nil -> timeout
        deadline -> min(timeout, deadline - System.monotonic_time(:millisecond))
      end

    result =
      if timeout > 0 do
        id = "access:" <> Base.encode16(:crypto.strong_rand_bytes(16), case: :lower)
        request = %{"rpc_version" => 1, "id" => id, "method" => "describe_agent", "params" => %{}}

        AgentRpcTransport.request(
          endpoint,
          id,
          request,
          fn _ -> {:error, :unexpected_preflight_progress} end,
          request_timeout_ms: timeout,
          connect_timeout_ms: timeout,
          max_rpc_frame_bytes: @max_frame_bytes
        )
        |> validate_reply()
      else
        {:error, :connection_preflight_timeout}
      end

    case result do
      :ok -> :ok
      {:error, reason} -> {:error, {:agent_connection_preflight_failed, reason}}
    end
  end

  defp validate_reply({:ok, descriptor}), do: validate_descriptor(descriptor)

  defp validate_reply({:error, {:agent_transport_failure, _, reason}})
       when reason in [:timeout, :request_timeout], do: {:error, :connection_preflight_timeout}

  defp validate_reply(_reply), do: {:error, :connection_preflight_unavailable}

  def validate_descriptor(%{"connection_access" => access}) when is_map(access) do
    mode =
      case {access["peer_scope"], access["control_mode"]} do
        {"host_loopback", "host_loopback"} -> {true, true}
        {"non_loopback", "trusted_lan_control"} -> {true, false}
        {"non_loopback", "discovery_only"} -> {false, false}
        _ -> nil
      end

    case mode do
      {allowed, mutations} ->
        expected = %{
          "schema_version" => @schema,
          "scope" => "native_socket_connection",
          "peer_scope" => access["peer_scope"],
          "control_mode" => access["control_mode"],
          "control_allowed" => allowed,
          "lifecycle_mutations_allowed" => mutations,
          "authenticated" => false,
          "encrypted" => false
        }

        cond do
          access != expected -> {:error, :invalid_connection_access}
          not allowed -> {:error, :connection_control_not_permitted}
          true -> :ok
        end

      _ ->
        {:error, :invalid_connection_access}
    end
  end

  def validate_descriptor(_descriptor), do: {:error, :invalid_connection_access}

  def reject_candidate(endpoint, reason, opts) do
    if reason == :connection_control_not_permitted,
      do: AgentPool.report_success(endpoint),
      else: AgentPool.report_failure(endpoint, reason)

    # A capacity retry in this same call must not resurrect a rejected candidate.
    candidates = Enum.reject(Keyword.fetch!(opts, :capacity_candidates), &(&1.id == endpoint.id))

    receipt = %{
      agent: "rust-agent-rpc@#{endpoint.id}",
      agent_id: endpoint.id,
      failure_stage: "preflight",
      reason_code: Atom.to_string(reason),
      task_dispatched: false,
      retryable: false,
      next_action: "check_agent_control_access"
    }

    {Keyword.put(opts, :capacity_candidates, candidates), receipt}
  end
end
