defmodule KyuubikiWeb.Playground.AgentClient do
  @moduledoc """
  TCP client for the Rust FEM agent RPC process.
  """

  alias KyuubikiWeb.Playground.AgentPool

  @rpc_version 1

  @spec solve_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_bar_1d", params, on_progress)
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

  @spec cancel_job(String.t()) :: {:ok, map()} | {:error, term()}
  def cancel_job(job_id) when is_binary(job_id) do
    request("cancel_job", %{job_id: job_id})
  end

  @spec request(String.t(), map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def request(method, params, on_progress \\ fn _progress -> :ok end)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) do
    with {:ok, result, _endpoint} <- request_with_agent(method, params, on_progress) do
      {:ok, result}
    end
  end

  @spec request_with_agent(String.t(), map(), (map() -> any())) ::
          {:ok, map(), AgentPool.endpoint()} | {:error, term()}
  def request_with_agent(method, params, on_progress \\ fn _progress -> :ok end)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) do
    request_id = request_id()
    request = build_request(request_id, method, params)
    endpoints = AgentPool.checkout_endpoints()

    attempt_request(endpoints, request_id, request, on_progress, [])
  end

  defp build_request(request_id, method, params) do
    %{
      "rpc_version" => @rpc_version,
      "id" => request_id,
      "method" => method,
      "params" => params
    }
  end

  defp attempt_request([], _request_id, _request, _on_progress, failures) do
    {:error, {:all_agents_failed, Enum.reverse(failures)}}
  end

  defp attempt_request([endpoint | rest], request_id, request, on_progress, failures) do
    case request_once(endpoint, request_id, request, on_progress) do
      {:ok, result} ->
        {:ok, result, endpoint}

      {:error, {:rpc_error, _code, _message} = reason} ->
        {:error, reason}

      {:error, reason} ->
        attempt_request(rest, request_id, request, on_progress, [
          %{agent: worker_id(endpoint), reason: inspect(reason)} | failures
        ])
    end
  end

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

  defp send_request(socket, request) do
    payload = Jason.encode!(request)
    :gen_tcp.send(socket, payload)
  end

  defp recv_response(socket, request_id, on_progress) do
    case :gen_tcp.recv(socket, 0, recv_timeout_ms()) do
      {:ok, payload} ->
        case Jason.decode(payload) do
          {:ok,
           %{"event" => event, "rpc_version" => @rpc_version, "id" => ^request_id} = frame}
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
