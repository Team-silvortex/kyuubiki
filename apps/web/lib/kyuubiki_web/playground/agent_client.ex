defmodule KyuubikiWeb.Playground.AgentClient do
  @moduledoc """
  TCP client for the Rust FEM agent RPC process.
  """

  @default_host "127.0.0.1"
  @default_port 5001

  @spec solve(map()) :: {:ok, map()} | {:error, term()}
  def solve(params) when is_map(params) do
    request = %{"method" => "solve_bar_1d", "params" => params}

    with {:ok, socket} <- connect(),
         :ok <- send_request(socket, request),
         {:ok, response_line} <- recv_response(socket),
         :ok <- :gen_tcp.close(socket) do
      decode_response(response_line)
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp connect do
    :gen_tcp.connect(String.to_charlist(host()), port(), [:binary, active: false])
  end

  defp send_request(socket, request) do
    payload = Jason.encode!(request) <> "\n"
    :gen_tcp.send(socket, payload)
  end

  defp recv_response(socket) do
    recv_response(socket, "")
  end

  defp recv_response(socket, buffer) do
    case :gen_tcp.recv(socket, 0, 5_000) do
      {:ok, chunk} ->
        combined = buffer <> chunk

        if String.ends_with?(combined, "\n") do
          {:ok, String.trim_trailing(combined, "\n")}
        else
          recv_response(socket, combined)
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp decode_response(raw_response) do
    with {:ok, decoded} <- Jason.decode(raw_response),
         true <- decoded["ok"] do
      {:ok, decoded["result"]}
    else
      false ->
        error = decoded_error(raw_response)
        {:error, {:rpc_error, error["code"], error["message"]}}

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
