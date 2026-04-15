defmodule KyuubikiSdk.SolverRpcClient do
  @moduledoc "TCP RPC client for kyuubiki.solver-rpc/v1."

  alias KyuubikiSdk.Error

  defstruct [:host, :port, :timeout]

  def new(host, port, opts \\ []) do
    %__MODULE__{
      host: to_charlist(host),
      port: port,
      timeout: Keyword.get(opts, :timeout, 15_000)
    }
  end

  def ping(client), do: call(client, "ping", %{})
  def describe_agent(client), do: call(client, "describe_agent", %{})
  def solve_bar_1d(client, payload), do: call(client, "solve_bar_1d", payload)
  def solve_truss_2d(client, payload), do: call(client, "solve_truss_2d", payload)
  def solve_truss_3d(client, payload), do: call(client, "solve_truss_3d", payload)
  def solve_plane_triangle_2d(client, payload), do: call(client, "solve_plane_triangle_2d", payload)
  def cancel_job(client, job_id), do: call(client, "cancel_job", %{"job_id" => job_id})

  def call(client, method, params) do
    with {:ok, socket} <-
           :gen_tcp.connect(client.host, client.port, [:binary, active: false, packet: 0], client.timeout),
         :ok <- :gen_tcp.send(socket, encode_frame(method, params)),
         {:ok, response} <- recv_until_response(socket, []) do
      :gen_tcp.close(socket)
      {:ok, response}
    else
      {:error, %Error{} = error} -> {:error, error}
      {:error, reason} -> {:error, Error.transport(inspect(reason))}
      error -> {:error, Error.transport(inspect(error))}
    end
  end

  defp encode_frame(method, params) do
    payload =
      Jason.encode!(%{
        rpc_version: 1,
        id: Integer.to_string(System.unique_integer([:positive])),
        method: method,
        params: params
      })

    <<byte_size(payload)::unsigned-big-32, payload::binary>>
  end

  defp recv_until_response(socket, progress_frames) do
    with {:ok, <<size::unsigned-big-32>>} <- :gen_tcp.recv(socket, 4),
         {:ok, payload} <- :gen_tcp.recv(socket, size) do
      frame = Jason.decode!(payload)

      cond do
        Map.has_key?(frame, "event") ->
          recv_until_response(socket, [frame | progress_frames])

        frame["ok"] == true ->
          {:ok, %{result: frame["result"], progress_frames: Enum.reverse(progress_frames)}}

        true ->
          error = frame["error"] || %{}
          {:error, Error.rpc(error["message"] || "rpc failed", code: error["code"])}
      end
    end
  end
end
