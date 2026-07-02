defmodule KyuubikiWeb.Playground.AgentClient do
  @moduledoc """
  TCP client for the Rust FEM agent RPC process.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Playground.AgentRegistry

  @rpc_version 1

  @spec solve_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_bar_1d", params, on_progress)
  end

  @spec solve_acoustic_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_acoustic_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_acoustic_bar_1d", params, on_progress)
  end

  @spec solve_thermal_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_bar_1d", params, on_progress)
  end

  @spec solve_heat_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_heat_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_heat_bar_1d", params, on_progress)
  end

  @spec solve_transient_heat_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_transient_heat_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_transient_heat_bar_1d", params, on_progress)
  end

  @spec solve_electrostatic_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_electrostatic_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electrostatic_bar_1d", params, on_progress)
  end

  @spec solve_electrostatic_plane_triangle_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_electrostatic_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electrostatic_plane_triangle_2d", params, on_progress)
  end

  @spec solve_electrostatic_plane_quad_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_electrostatic_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electrostatic_plane_quad_2d", params, on_progress)
  end

  @spec solve_heat_plane_triangle_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_heat_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_heat_plane_triangle_2d", params, on_progress)
  end

  @spec solve_heat_plane_quad_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_heat_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_heat_plane_quad_2d", params, on_progress)
  end

  @spec solve_thermal_truss_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_truss_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_truss_2d", params, on_progress)
  end

  @spec solve_thermal_truss_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_truss_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_truss_3d", params, on_progress)
  end

  @spec solve_beam_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_beam_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_beam_1d", params, on_progress)
  end

  @spec solve_thermal_beam_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_beam_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_beam_1d", params, on_progress)
  end

  @spec solve_thermal_frame_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_frame_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_frame_2d", params, on_progress)
  end

  @spec solve_thermal_frame_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_frame_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_frame_3d", params, on_progress)
  end

  @spec solve_torsion_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_torsion_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_torsion_1d", params, on_progress)
  end

  @spec solve_spring_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_spring_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_spring_1d", params, on_progress)
  end

  @spec solve_transient_spring_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_transient_spring_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_transient_spring_1d", params, on_progress)
  end

  @spec solve_harmonic_spring_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_harmonic_spring_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_harmonic_spring_1d", params, on_progress)
  end

  @spec solve_spring_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_spring_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_spring_2d", params, on_progress)
  end

  @spec solve_spring_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_spring_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_spring_3d", params, on_progress)
  end

  @spec solve_truss_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_truss_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_truss_2d", params, on_progress)
  end

  @spec solve_truss_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_truss_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_truss_3d", params, on_progress)
  end

  @spec solve_plane_triangle_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_plane_triangle_2d", params, on_progress)
  end

  @spec solve_thermal_plane_triangle_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_thermal_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_plane_triangle_2d", params, on_progress)
  end

  @spec solve_plane_quad_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_plane_quad_2d", params, on_progress)
  end

  @spec solve_thermal_plane_quad_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_plane_quad_2d", params, on_progress)
  end

  @spec solve_frame_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_frame_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_frame_2d", params, on_progress)
  end

  @spec solve_frame_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_frame_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_frame_3d", params, on_progress)
  end

  @spec solve_solid_tetra_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_solid_tetra_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_solid_tetra_3d", params, on_progress)
  end

  @spec cancel_job(String.t()) :: {:ok, map()} | {:error, term()}
  def cancel_job(job_id) when is_binary(job_id) do
    request("cancel_job", %{job_id: job_id})
  end

  @spec run_operator_task_ir(map(), keyword() | (map() -> any()), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def run_operator_task_ir(
        task_ir,
        opts_or_progress \\ [],
        on_progress \\ fn _progress -> :ok end
      )
      when is_map(task_ir) do
    {opts, progress_handler} = normalize_operator_task_rpc_opts(opts_or_progress, on_progress)

    request(
      OperatorTaskIR.agent_rpc_method(),
      OperatorTaskIR.agent_rpc_params(task_ir, opts),
      progress_handler,
      operator_task_routing_opts(task_ir, opts)
    )
  end

  @spec ping(AgentPool.endpoint() | nil) :: {:ok, map()} | {:error, term()}
  def ping(endpoint \\ nil) do
    request_to_target("ping", %{}, endpoint)
  end

  @spec describe_agent(AgentPool.endpoint() | nil) :: {:ok, map()} | {:error, term()}
  def describe_agent(endpoint \\ nil) do
    request_to_target("describe_agent", %{}, endpoint)
  end

  @spec request(String.t(), map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def request(method, params, on_progress \\ fn _progress -> :ok end)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) do
    with {:ok, result, _endpoint} <- request_with_agent(method, params, on_progress) do
      {:ok, result}
    end
  end

  @spec request(String.t(), map(), (map() -> any()), keyword()) :: {:ok, map()} | {:error, term()}
  def request(method, params, on_progress, opts)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) and is_list(opts) do
    with {:ok, result, _endpoint} <- request_with_agent(method, params, on_progress, opts) do
      {:ok, result}
    end
  end

  defp normalize_operator_task_rpc_opts(on_progress, _default_progress)
       when is_function(on_progress, 1),
       do: {[], on_progress}

  defp normalize_operator_task_rpc_opts(opts, on_progress)
       when is_list(opts) and is_function(on_progress, 1),
       do: {opts, on_progress}

  defp operator_task_routing_opts(task_ir, opts) do
    task_ir
    |> OperatorTaskIR.agent_routing_opts()
    |> maybe_require_operator_package_runtime(Keyword.get(opts, :mode))
  end

  defp maybe_require_operator_package_runtime(routing_opts, mode)
       when mode in [:execute, "execute"] do
    Keyword.put(routing_opts, :requires_operator_package_runtime, true)
  end

  defp maybe_require_operator_package_runtime(routing_opts, _mode), do: routing_opts

  @spec request_with_agent(String.t(), map(), (map() -> any()), keyword()) ::
          {:ok, map(), AgentPool.endpoint()} | {:error, term()}
  def request_with_agent(method, params, on_progress \\ fn _progress -> :ok end, opts \\ [])
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) and is_list(opts) do
    request_id = request_id()
    request = build_request(request_id, method, params)
    opts = put_execution_lease(opts, request_id, method)
    endpoints = AgentPool.checkout_endpoints(method, opts)

    case endpoints do
      [] -> {:error, no_matching_agent_error(method, opts)}
      _ -> attempt_request(endpoints, request_id, request, on_progress, opts, [])
    end
  end

  defp build_request(request_id, method, params) do
    %{
      "rpc_version" => @rpc_version,
      "id" => request_id,
      "method" => method,
      "params" => params
    }
  end

  defp request_to_target(method, params, nil) do
    request(method, params)
  end

  defp request_to_target(method, params, endpoint) when is_map(endpoint) do
    request_id = request_id()
    request = build_request(request_id, method, params)
    normalized = normalize_endpoint(endpoint)

    case request_once(normalized, request_id, request, fn _ -> :ok end) do
      {:ok, result} ->
        :ok = AgentPool.report_success(normalized)
        {:ok, result}

      {:error, {:rpc_error, _code, _message} = reason} ->
        :ok = AgentPool.report_success(normalized)
        {:error, reason}

      {:error, reason} ->
        :ok = AgentPool.report_failure(normalized, reason)
        {:error, reason}
    end
  end

  defp attempt_request([], _request_id, _request, _on_progress, _opts, failures) do
    {:error, {:all_agents_failed, Enum.reverse(failures)}}
  end

  defp attempt_request([endpoint | rest], request_id, request, on_progress, opts, failures) do
    with_claimed_endpoint(endpoint, opts, fn ->
      case request_once(endpoint, request_id, request, on_progress) do
        {:ok, result} ->
          :ok = AgentPool.report_success(endpoint)
          {:ok, result, endpoint}

        {:error, {:rpc_error, _code, _message} = reason} ->
          :ok = AgentPool.report_success(endpoint)
          {:error, reason}

        {:error, reason} ->
          :ok = AgentPool.report_failure(endpoint, reason)

          attempt_request(rest, request_id, request, on_progress, opts, [
            %{agent: worker_id(endpoint), reason: inspect(reason)} | failures
          ])
      end
    end)
    |> case do
      {:error, {:agent_execution_conflict, _conflict} = reason} ->
        attempt_request(rest, request_id, request, on_progress, opts, [
          %{agent: worker_id(endpoint), reason: inspect(reason)} | failures
        ])

      other ->
        other
    end
  end

  defp no_matching_agent_error(method, opts) do
    {:no_matching_agent,
     %{}
     |> Map.put(:method, method)
     |> Map.put(
       :required_capabilities,
       opts |> Keyword.get(:required_capabilities, []) |> Enum.filter(&is_binary/1)
     )
     |> Map.put(
       :placement_tags,
       opts |> Keyword.get(:placement_tags, []) |> Enum.filter(&is_binary/1)
     )
     |> maybe_put_required_package_runtime(opts)}
  end

  defp maybe_put_required_package_runtime(error, opts) do
    if Keyword.get(opts, :requires_operator_package_runtime, false),
      do: Map.put(error, :required_operator_package_runtime, true),
      else: error
  end

  defp with_claimed_endpoint(endpoint, opts, fun)
       when is_map(endpoint) and is_list(opts) and is_function(fun, 0) do
    lease = Keyword.get(opts, :execution_lease, %{})

    case claim_execution(endpoint, opts) do
      :ok ->
        try do
          fun.()
        after
          release_execution(endpoint, lease)
        end

      {:error, _reason} = error ->
        error
    end
  end

  defp claim_execution(endpoint, opts) do
    lease = Keyword.get(opts, :execution_lease, %{})

    cond do
      not is_binary(Map.get(endpoint, :control_mode)) ->
        :ok

      not is_binary(Map.get(lease, :lease_id)) ->
        :ok

      true ->
        AgentRegistry.claim_execution(endpoint.id, Map.new(lease))
        |> case do
          {:ok, _lease} -> :ok
          {:error, _reason} = error -> error
        end
    end
  end

  defp release_execution(endpoint, lease) when is_map(endpoint) and is_map(lease) do
    if is_binary(Map.get(endpoint, :control_mode)) and is_binary(Map.get(lease, :lease_id)) do
      AgentRegistry.release_execution(endpoint.id, Map.get(lease, :lease_id))
    else
      :ok
    end
  end

  defp put_execution_lease(opts, request_id, method) do
    orchestration = Keyword.get(opts, :orchestration, %{})

    lease =
      %{
        "lease_id" => "lease:" <> request_id,
        "method" => method,
        "job_id" => Keyword.get(opts, :job_id)
      }
      |> maybe_put_lease_value(
        "control_mode",
        Map.get(orchestration, :control_mode) || Map.get(orchestration, "control_mode")
      )
      |> maybe_put_lease_value(
        "orch_id",
        Map.get(orchestration, :orch_id) || Map.get(orchestration, "orch_id")
      )
      |> maybe_put_lease_value(
        "orch_session_id",
        Map.get(orchestration, :orch_session_id) || Map.get(orchestration, "orch_session_id")
      )
      |> maybe_put_lease_value(
        "cluster_id",
        Map.get(orchestration, :cluster_id) || Map.get(orchestration, "cluster_id")
      )

    Keyword.put(opts, :execution_lease, lease)
  end

  defp maybe_put_lease_value(lease, _key, nil), do: lease

  defp maybe_put_lease_value(lease, key, value) when is_binary(value),
    do: Map.put(lease, key, value)

  defp maybe_put_lease_value(lease, key, value) when is_binary(key) and is_binary(value),
    do: Map.put(lease, key, value)

  defp maybe_put_lease_value(lease, _key, _value), do: lease

  defp request_once(endpoint, request_id, request, on_progress) do
    with {:ok, socket} <- connect(endpoint),
         :ok <- send_request(socket, request),
         {:ok, response_payload} <- recv_response(socket, request_id, on_progress),
         :ok <- :gen_tcp.close(socket) do
      decode_response(response_payload, request_id)
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp connect(endpoint) do
    :gen_tcp.connect(
      String.to_charlist(endpoint.host),
      endpoint.port,
      [
        :binary,
        packet: 4,
        active: false
      ],
      connect_timeout_ms()
    )
  end

  defp normalize_endpoint(%{id: _id, host: _host, port: _port} = endpoint), do: endpoint

  defp normalize_endpoint(%{"host" => host, "port" => port} = endpoint) do
    %{
      id: Map.get(endpoint, "id", "#{host}:#{port}"),
      host: host,
      port: port
    }
  end

  defp send_request(socket, request) do
    payload = Jason.encode!(request)
    :gen_tcp.send(socket, payload)
  end

  defp recv_response(socket, request_id, on_progress) do
    case :gen_tcp.recv(socket, 0, recv_timeout_ms()) do
      {:ok, payload} ->
        case Jason.decode(payload) do
          {:ok, %{"event" => event, "rpc_version" => @rpc_version, "id" => ^request_id} = frame}
          when event in ["progress", "heartbeat"] ->
            _ = on_progress.(frame["progress"])
            recv_response(socket, request_id, on_progress)

          {:ok, %{"rpc_version" => @rpc_version, "id" => ^request_id}} ->
            {:ok, payload}

          {:ok, _frame} ->
            {:error, {:invalid_response, :unexpected_rpc_frame}}

          {:error, reason} ->
            {:error, {:invalid_response, reason}}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp decode_response(raw_response, request_id) do
    case Jason.decode(raw_response) do
      {:ok, %{"rpc_version" => @rpc_version, "id" => ^request_id, "ok" => true} = decoded} ->
        {:ok, decoded["result"]}

      {:ok, %{"rpc_version" => @rpc_version, "id" => ^request_id, "ok" => false}} ->
        error = decoded_error(raw_response)
        {:error, {:rpc_error, error["code"], error["message"]}}

      {:ok, _decoded} ->
        {:error, {:invalid_response, :malformed_rpc_response}}

      {:error, reason} ->
        {:error, {:invalid_response, reason}}
    end
  end

  defp decoded_error(raw_response) do
    case Jason.decode(raw_response) do
      {:ok, %{"error" => error}} when is_map(error) -> error
      _ -> %{"code" => "invalid_response", "message" => "agent returned malformed error payload"}
    end
  end

  @spec worker_id(AgentPool.endpoint()) :: String.t()
  def worker_id(endpoint), do: "rust-agent-rpc@#{endpoint.id}"

  defp request_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp connect_timeout_ms do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:connect_timeout_ms, 1_500)
  end

  defp recv_timeout_ms do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:recv_timeout_ms, 15_000)
  end
end
