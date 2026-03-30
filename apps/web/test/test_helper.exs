ExUnit.start()

defmodule KyuubikiWeb.TestSupport.FakePlaygroundAgent do
  @moduledoc false

  def start_link(frames) when is_list(frames) do
    parent = self()

    Task.start_link(fn ->
      {:ok, listen_socket} =
        :gen_tcp.listen(0, [
          :binary,
          packet: 4,
          active: false,
          reuseaddr: true,
          ip: {127, 0, 0, 1}
        ])

      {:ok, port} = :inet.port(listen_socket)
      send(parent, {:fake_agent_ready, port})

      {:ok, socket} = :gen_tcp.accept(listen_socket)
      {:ok, request_payload} = :gen_tcp.recv(socket, 0, 1_000)
      request = Jason.decode!(request_payload)

      Enum.each(frames, fn frame ->
        response_payload =
          frame
          |> Map.put_new("rpc_version", request["rpc_version"])
          |> Map.put_new("id", request["id"])
          |> Jason.encode!()

        :ok = :gen_tcp.send(socket, response_payload)
      end)

      :gen_tcp.close(socket)
      :gen_tcp.close(listen_socket)
    end)
  end
end
