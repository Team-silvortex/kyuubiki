defmodule KyuubikiWeb.Playground.AgentCapacityRetryTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.Playground.{AgentClient, AgentExecutionGate, AgentPool}

  setup do
    original = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, original)
      AgentPool.reload()
    end)

    :ok
  end

  test "capacity rejection can wait without weakening checkpoint-required execution policy" do
    agent = agent("capacity-transient", 2)
    configure([agent])

    assert {:ok, %{"accepted_by" => "capacity-transient"}} =
             request(retry_safety: "checkpoint_required")

    for _ <- 1..3, do: assert_receive({:capacity_request, "capacity-transient", _})
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  test "full native capacity selects an available peer without replaying accepted work" do
    first = agent("capacity-first", :always)
    second = agent("capacity-second", 0)
    configure([first, second])

    assert {:ok, %{"accepted_by" => "capacity-second"}} =
             request(retry_safety: "checkpoint_required")

    assert_receive {:capacity_request, "capacity-first", _}
    assert_receive {:capacity_request, "capacity-second", _}
    refute_receive {:capacity_request, "capacity-first", _}, 20
  end

  test "permanent saturation expires without leaking a lease or penalizing agent health" do
    configure([agent("capacity-timeout", :always)])
    started = System.monotonic_time(:millisecond)

    assert {:error, {:agent_capacity_timeout, %{timeout_ms: 120}}} =
             request(queue_timeout_ms: 120)

    assert System.monotonic_time(:millisecond) - started < 1_000
    assert AgentExecutionGate.snapshot().active_lease_count == 0
    assert AgentExecutionGate.snapshot().queued_request_count == 0
    [endpoint] = AgentPool.checkout_endpoints("solve_bar_1d", [])
    assert Map.get(endpoint, :consecutive_failures, 0) == 0
  end

  test "ownership is rechecked before dispatch after capacity becomes available" do
    configure([agent("capacity-fenced", 1)])
    Process.put(:capacity_authorizations, 0)

    authorize = fn ->
      calls = Process.get(:capacity_authorizations)
      Process.put(:capacity_authorizations, calls + 1)
      if calls == 0, do: :ok, else: {:error, :stale_execution_owner}
    end

    assert {:error, {:agent_retry_blocked, %{reason_code: "orchestra_dispatch_rejected"}}} =
             request(before_dispatch: authorize)

    assert_receive {:capacity_request, "capacity-fenced", _}
    refute_receive {:capacity_request, "capacity-fenced", _}, 50
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  defp request(opts) do
    AgentClient.request(
      "solve_bar_1d",
      %{},
      fn _ -> :ok end,
      Keyword.merge([queue_timeout_ms: 500], opts)
    )
  end

  defp configure(endpoints) do
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
    AgentPool.reload()
  end

  defp agent(id, busy) do
    owner = self()

    {:ok, pid} =
      Task.start(fn ->
        {:ok, listener} =
          :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])

        {:ok, port} = :inet.port(listener)
        send(owner, {:capacity_ready, id, port})
        serve(listener, owner, id, busy)
      end)

    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    assert_receive {:capacity_ready, ^id, port}, 2_000
    %{id: id, host: "127.0.0.1", port: port, capacity: 1}
  end

  defp serve(listener, owner, id, busy) do
    {:ok, socket} = :gen_tcp.accept(listener)
    {:ok, bytes} = :gen_tcp.recv(socket, 0, 1_000)
    request = Jason.decode!(bytes)
    send(owner, {:capacity_request, id, request})

    response =
      if busy == 0 do
        %{"ok" => true, "result" => %{"accepted_by" => id}}
      else
        %{"ok" => false, "error" => %{"code" => "agent_at_capacity", "message" => "not admitted"}}
      end

    :ok =
      :gen_tcp.send(
        socket,
        Jason.encode!(Map.merge(response, %{"rpc_version" => 1, "id" => request["id"]}))
      )

    :gen_tcp.close(socket)
    serve(listener, owner, id, if(busy == :always, do: :always, else: max(busy - 1, 0)))
  end
end
