ExUnit.start()

defmodule KyuubikiWeb.TestSupport.FakePlaygroundAgent do
  @moduledoc false

  def start_link(response) when is_map(response) do
    parent = self()

    Task.start_link(fn ->
      {:ok, listen_socket} =
        :gen_tcp.listen(0, [
          :binary,
          packet: :line,
          active: false,
          reuseaddr: true,
          ip: {127, 0, 0, 1}
        ])

      {:ok, port} = :inet.port(listen_socket)
      send(parent, {:fake_agent_ready, port})

      {:ok, socket} = :gen_tcp.accept(listen_socket)
      {:ok, _request} = :gen_tcp.recv(socket, 0, 1_000)
      :ok = :gen_tcp.send(socket, Jason.encode!(response) <> "\n")
      :gen_tcp.close(socket)
      :gen_tcp.close(listen_socket)
    end)
  end
end
