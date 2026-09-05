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

  test "balances fleet leases by normalized capacity and exposes the decision" do
    high = %{id: "fleet-high", host: "127.0.0.1", port: 5011, capacity: 4}
    low = %{id: "fleet-low", host: "127.0.0.1", port: 5012, capacity: 1}

    assert {:ok, ^high, first} =
             AgentExecutionGate.acquire([high, low], "fleet-lease-a", 500)

    assert first == %{
             active_slots_after: 1,
             active_slots_before: 0,
             capacity_slots: 4,
             queue_position: 0,
             selected_agent_id: "fleet-high",
             selection_policy: "least_utilized_capacity_v1",
             utilization_after: 0.25,
             utilization_before: 0.0,
             waited_ms: 0
           }

    assert {:ok, ^low, second} =
             AgentExecutionGate.acquire([high, low], "fleet-lease-b", 500)

    assert second.selected_agent_id == "fleet-low"
    assert second.utilization_after == 1.0

    assert {:ok, ^high, third} =
             AgentExecutionGate.acquire([high, low], "fleet-lease-c", 500)

    assert third.active_slots_before == 1
    assert third.utilization_after == 0.5

    assert %{
             selection_policy: "least_utilized_capacity_v1",
             active_lease_count: 3,
             saturated_endpoint_count: 1,
             active_by_endpoint: %{"fleet-high" => 2, "fleet-low" => 1},
             capacity_by_endpoint: %{"fleet-high" => 4, "fleet-low" => 1},
             utilization_by_endpoint: %{"fleet-high" => 0.5, "fleet-low" => 1.0}
           } = AgentExecutionGate.snapshot([high, low])

    assert :ok = AgentExecutionGate.release("fleet-lease-a")
    assert :ok = AgentExecutionGate.release("fleet-lease-b")
    assert :ok = AgentExecutionGate.release("fleet-lease-c")
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

  test "rejects duplicate lease ids and prevents foreign release" do
    endpoint = %{id: "static-agent-owned", host: "127.0.0.1", port: 5004}

    assert {:ok, ^endpoint, _metadata} =
             AgentExecutionGate.acquire([endpoint], "lease-owned", 500)

    foreign =
      Task.async(fn ->
        {
          AgentExecutionGate.acquire([endpoint], "lease-owned", 500),
          AgentExecutionGate.release("lease-owned")
        }
      end)

    assert {
             {:error, {:duplicate_execution_lease, "lease-owned"}},
             {:error, {:execution_lease_not_owned, "lease-owned"}}
           } = Task.await(foreign)

    assert %{active_lease_count: 1} = AgentExecutionGate.snapshot()
    assert :ok = AgentExecutionGate.release("lease-owned")
  end

  test "rejects a duplicate id while its original request is queued" do
    endpoint = %{id: "static-agent-queued", host: "127.0.0.1", port: 5005}

    assert {:ok, ^endpoint, _metadata} =
             AgentExecutionGate.acquire([endpoint], "lease-capacity-owner", 500)

    parent = self()

    queued =
      Task.async(fn ->
        result = AgentExecutionGate.acquire([endpoint], "lease-queued", 1_000)
        send(parent, {:original_queued_result, result})
        receive do: (:release -> AgentExecutionGate.release("lease-queued"))
      end)

    Process.sleep(20)

    assert {:error, {:duplicate_execution_lease, "lease-queued"}} =
             AgentExecutionGate.acquire([endpoint], "lease-queued", 500)

    assert %{active_lease_count: 1, queued_request_count: 1} =
             AgentExecutionGate.snapshot()

    assert :ok = AgentExecutionGate.release("lease-capacity-owner")
    assert_receive {:original_queued_result, {:ok, ^endpoint, _metadata}}, 1_000
    send(queued.pid, :release)
    Task.await(queued)
  end

  test "rejects malformed lease ids without crashing the caller" do
    endpoint = %{id: "static-agent-invalid", host: "127.0.0.1", port: 5006}

    assert {:error, :invalid_execution_lease} =
             AgentExecutionGate.acquire([endpoint], "", 500)

    assert {:error, :invalid_execution_lease} = AgentExecutionGate.release("")
  end
end
