defmodule KyuubikiWeb.Playground.AgentExecutionGateTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentExecutionGate

  test "queues static endpoint work until the active capacity lease is released" do
    endpoint = %{id: "static-agent-a", host: "127.0.0.1", port: 5001, capacity: 1}

    assert {:ok, ^endpoint, %{waited_ms: 0}} =
             AgentExecutionGate.acquire([endpoint], "lease-a", 1_000)

    parent = self()

    queued =
      Task.async(fn ->
        result = AgentExecutionGate.acquire([endpoint], "lease-b", 1_000)
        send(parent, {:queued_result, result})
        receive do: (:release -> AgentExecutionGate.release("lease-b"))
      end)

    Process.sleep(20)
    assert %{active_lease_count: 1, queued_request_count: 1} = AgentExecutionGate.snapshot()

    assert :ok = AgentExecutionGate.release("lease-a")
    assert_receive {:queued_result, {:ok, ^endpoint, %{waited_ms: waited_ms}}}, 1_000
    assert waited_ms >= 10

    send(queued.pid, :release)
    Task.await(queued)
  end

  test "honors explicit endpoint capacity" do
    endpoint = %{id: "static-agent-capacity", host: "127.0.0.1", port: 5002, capacity: 2}

    assert {:ok, ^endpoint, _metadata} = AgentExecutionGate.acquire([endpoint], "lease-c", 500)
    assert {:ok, ^endpoint, _metadata} = AgentExecutionGate.acquire([endpoint], "lease-d", 500)
    assert %{active_lease_count: 2} = AgentExecutionGate.snapshot()

    assert :ok = AgentExecutionGate.release("lease-c")
    assert :ok = AgentExecutionGate.release("lease-d")
  end

  test "returns an observable queue timeout instead of opening another connection" do
    endpoint = %{id: "static-agent-timeout", host: "127.0.0.1", port: 5003}

    assert {:ok, ^endpoint, _metadata} = AgentExecutionGate.acquire([endpoint], "lease-e", 500)

    assert {:error,
            {:agent_queue_timeout,
             %{timeout_ms: 25, queue_position: 1, candidate_agent_ids: ["static-agent-timeout"]}}} =
             AgentExecutionGate.acquire([endpoint], "lease-f", 25)

    assert :ok = AgentExecutionGate.release("lease-e")
  end
end
