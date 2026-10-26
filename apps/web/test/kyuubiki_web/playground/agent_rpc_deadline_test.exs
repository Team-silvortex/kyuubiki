defmodule KyuubikiWeb.Playground.AgentRpcDeadlineTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentRpcTransport, as: Transport

  test "a peer that stops reading cannot hold a large send past the request budget" do
    endpoint =
      serve(fn socket, owner ->
        send(owner, :peer_connected)

        receive do
          :release -> :ok
        after
          5_000 -> :ok
        end

        :gen_tcp.close(socket)
      end)

    request = query(%{"payload" => String.duplicate("x", 16 * 1024 * 1024)})
    call = Task.async(fn -> request(endpoint, request, request_timeout_ms: 150) end)
    on_exit(fn -> Process.exit(call.pid, :kill) end)
    assert_receive :peer_connected, 2_000

    result = Task.yield(call, 1_000)
    assert result != nil, inspect(Process.info(call.pid, :current_stacktrace))
    assert {:ok, {:error, {:agent_transport_failure, stage, reason}}} = result
    assert stage in [:send, :receive]
    assert reason in [:timeout, :request_timeout]
  end

  test "complete heartbeat frames do not renew the exchange budget" do
    endpoint =
      serve(fn socket, _owner ->
        read_request(socket)
        heartbeat = packet(%{"event" => "heartbeat", "progress" => %{"stage" => "solving"}})
        stream(socket, List.duplicate(heartbeat, 30), 30)
      end)

    owner = self()
    callback = fn value -> send(owner, {:progress, value}) end
    call = Task.async(fn -> request(endpoint, query(), [request_timeout_ms: 150], callback) end)
    on_exit(fn -> Process.exit(call.pid, :kill) end)
    assert {:ok, result} = Task.yield(call, 1_000)
    assert_timeout(result)
    assert_received {:progress, %{"stage" => "solving"}}
  end

  test "partial frame bytes cannot keep a request alive" do
    endpoint =
      serve(fn socket, _owner ->
        read_request(socket)
        packet = packet(%{"ok" => true, "result" => %{"value" => 42}})
        stream(socket, for(<<byte <- packet>>, do: <<byte>>), 30)
      end)

    call = Task.async(fn -> request(endpoint, query(), request_timeout_ms: 150) end)
    on_exit(fn -> Process.exit(call.pid, :kill) end)
    assert {:ok, result} = Task.yield(call, 1_000)
    assert_timeout(result)
  end

  test "a slow local callback cannot expose an already buffered late result" do
    endpoint =
      serve(fn socket, _owner ->
        read_request(socket)

        :gen_tcp.send(socket, [
          packet(%{"event" => "progress", "progress" => %{"stage" => "solving"}}),
          packet(%{"ok" => true, "result" => %{"value" => 42}})
        ])
      end)

    callback = fn _ -> Process.sleep(200) end
    result = request(endpoint, query(), [request_timeout_ms: 100], callback)
    assert_timeout(result)
  end

  test "valid fragmented results and later independent requests still succeed" do
    endpoint =
      serve(fn socket, _owner ->
        read_request(socket)

        <<first::binary-size(2), next::binary-size(7), rest::binary>> =
          packet(%{"ok" => true, "result" => %{"value" => 42}})

        stream(socket, [first, next, rest], 10)
      end)

    assert request(endpoint, query(), request_timeout_ms: 1_000) == {:ok, %{"value" => 42}}

    failing =
      serve(fn socket, _owner ->
        read_request(socket)
        :gen_tcp.send(socket, packet(%{"ok" => true, "result" => nil}))
      end)

    assert {:error, {:agent_transport_failure, :protocol, _}} =
             request(failing, query(), request_timeout_ms: 1_000)

    healthy =
      serve(fn socket, _owner ->
        read_request(socket)
        :gen_tcp.send(socket, packet(%{"ok" => true, "result" => %{"value" => 43}}))
      end)

    assert request(healthy, query(), request_timeout_ms: 1_000) == {:ok, %{"value" => 43}}
  end

  defp assert_timeout(result) do
    assert {:error, {:agent_transport_failure, :receive, reason}} = result
    assert reason in [:timeout, :request_timeout]
  end

  defp read_request(socket) do
    :ok = :inet.setopts(socket, packet: 4)
    assert {:ok, _} = :gen_tcp.recv(socket, 0, 2_000)
    :ok = :inet.setopts(socket, packet: :raw)
  end

  defp packet(frame) do
    payload =
      frame |> Map.merge(%{"rpc_version" => 1, "id" => "deadline-query"}) |> Jason.encode!()

    <<byte_size(payload)::32, payload::binary>>
  end

  defp stream(socket, chunks, delay) do
    Enum.reduce_while(chunks, :ok, fn chunk, :ok ->
      Process.sleep(delay)

      case :gen_tcp.send(socket, chunk) do
        :ok -> {:cont, :ok}
        {:error, _} -> {:halt, :ok}
      end
    end)
  end

  defp query(params \\ %{}),
    do: %{"rpc_version" => 1, "id" => "deadline-query", "method" => "ping", "params" => params}

  defp request(endpoint, query, opts, callback \\ fn _ -> :ok end),
    do: Transport.request(endpoint, query["id"], query, callback, opts)

  defp serve(fun) do
    {:ok, listener} =
      :gen_tcp.listen(0, [
        :binary,
        packet: :raw,
        active: false,
        ip: {127, 0, 0, 1},
        recbuf: 1_024
      ])

    {:ok, port} = :inet.port(listener)
    owner = self()

    peer =
      Task.async(fn ->
        {:ok, socket} = :gen_tcp.accept(listener, 2_000)

        try do
          fun.(socket, owner)
        after
          :gen_tcp.close(socket)
          :gen_tcp.close(listener)
        end
      end)

    on_exit(fn ->
      Process.exit(peer.pid, :kill)
      :gen_tcp.close(listener)
    end)

    %{host: "127.0.0.1", port: port}
  end
end
