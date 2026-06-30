ExUnit.start()

Path.wildcard(Path.join(__DIR__, "support/**/*.exs"))
|> Enum.reject(&String.ends_with?(&1, "/headless_live_server.exs"))
|> Enum.each(&Code.require_file/1)

defmodule KyuubikiWeb.TestSupport.FakePlaygroundAgent do
  @moduledoc false

  def start_link({:capture, capture_pid, frames})
      when is_pid(capture_pid) and is_list(frames) do
    start_link(frames, capture_pid)
  end

  def start_link(frames) when is_list(frames) do
    start_link(frames, nil)
  end

  defp start_link(frames, capture_pid) when is_list(frames) do
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

      if is_pid(capture_pid) do
        send(capture_pid, {:fake_agent_request, request})
      end

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

defmodule KyuubikiWeb.TestSupport.FakeStallingAgent do
  @moduledoc false

  def start_link(opts \\ []) do
    parent = self()
    delay_ms = Keyword.get(opts, :delay_ms, 2_000)

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
      {:ok, _request_payload} = :gen_tcp.recv(socket, 0, 1_000)
      Process.sleep(delay_ms)
      :gen_tcp.close(socket)
      :gen_tcp.close(listen_socket)
    end)
  end
end
