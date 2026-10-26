defmodule KyuubikiWeb.TestSupport.ControlledSolverAgent do
  @moduledoc false

  def child_spec(args) do
    %{id: __MODULE__, start: {__MODULE__, :start_link, [args]}, restart: :temporary}
  end

  def start_link({owner, progress}) do
    Task.start_link(fn -> serve(owner, progress) end)
  end

  defp serve(owner, progress) do
    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 4, active: false, reuseaddr: true, ip: {127, 0, 0, 1}])

    {:ok, port} = :inet.port(listener)
    send(owner, {:fake_agent_ready, port})
    {:ok, socket} = :gen_tcp.accept(listener, 5_000)

    try do
      serve_request(socket, owner, progress)
    after
      :gen_tcp.close(socket)
      :gen_tcp.close(listener)
    end
  end

  defp serve_request(socket, owner, :stall_before_read) do
    :ok = :inet.setopts(socket, recbuf: 1_024)
    send(owner, {:stalled_agent, self()})

    receive do
      :disconnect -> :ok
    after
      5_000 -> :ok
    end
  end

  defp serve_request(socket, owner, progress) do
    {:ok, payload} = :gen_tcp.recv(socket, 0, 2_000)
    request = Jason.decode!(payload)
    send_frame(socket, request, %{"event" => "progress", "progress" => progress})
    send(owner, {:solver_request, self(), request})

    receive do
      {:reply, frame} -> send_frame(socket, request, frame)
      {:raw_reply, payload} when is_binary(payload) -> :gen_tcp.send(socket, payload)
      :disconnect -> :ok
    after
      5_000 -> :ok
    end
  end

  defp send_frame(socket, request, frame) do
    payload =
      frame
      |> Map.put("rpc_version", request["rpc_version"])
      |> Map.put("id", request["id"])
      |> Jason.encode!()

    :ok = :gen_tcp.send(socket, payload)
  end
end
