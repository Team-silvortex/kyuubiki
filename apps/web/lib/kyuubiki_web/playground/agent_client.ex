defmodule KyuubikiWeb.Playground.AgentClient do
  @moduledoc """
  TCP client for the Rust FEM agent RPC process.
  """

  @default_host "127.0.0.1"
  @default_port 5001
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

  @spec request(String.t(), map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def request(method, params, on_progress \\ fn _progress -> :ok end)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) do
    request_id = request_id()

    request = %{
      "rpc_version" => @rpc_version,
      "id" => request_id,
      "method" => method,
      "params" => params
    }

    with {:ok, socket} <- connect(),
         :ok <- send_request(socket, request),
         {:ok, response_payload} <- recv_response(socket, request_id, on_progress),
         :ok <- :gen_tcp.close(socket) do
      decode_response(response_payload, request_id)
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp connect do
    :gen_tcp.connect(String.to_charlist(host()), port(), [:binary, packet: 4, active: false])
  end

  defp send_request(socket, request) do
    payload = Jason.encode!(request)
    :gen_tcp.send(socket, payload)
  end

  defp recv_response(socket, request_id, on_progress) do
    case :gen_tcp.recv(socket, 0, 5_000) do
      {:ok, payload} ->
        case Jason.decode(payload) do
          {:ok,
           %{"event" => "progress", "rpc_version" => @rpc_version, "id" => ^request_id} = frame} ->
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

  defp request_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp host do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get_lazy(:host, fn -> System.get_env("KYUUBIKI_AGENT_HOST", @default_host) end)
  end

  defp port do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get_lazy(:port, fn ->
      System.get_env("KYUUBIKI_AGENT_PORT", Integer.to_string(@default_port))
      |> String.to_integer()
    end)
  end
end
