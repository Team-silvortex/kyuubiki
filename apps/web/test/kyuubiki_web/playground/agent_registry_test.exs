defmodule KyuubikiWeb.Playground.AgentRegistryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentRegistry

  setup do
    Enum.each(AgentRegistry.agents(), fn agent ->
      AgentRegistry.unregister(agent.id)
    end)

    :ok
  end

  test "registers and reports active remote agents" do
    assert {:ok, agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-a",
               "host" => "10.20.0.11",
               "port" => 6101,
               "region" => "ap-shanghai"
             })

    assert agent.id == "solver-remote-a"
    assert AgentRegistry.status_snapshot().active_agents == 1
    assert Enum.map(AgentRegistry.active_endpoints(), & &1.id) == ["solver-remote-a"]
  end

  test "refreshes last seen through heartbeat" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-b",
               "host" => "10.20.0.12",
               "port" => 6102
             })

    assert {:ok, heartbeat_agent} =
             AgentRegistry.heartbeat("solver-remote-b", %{
               "host" => "10.20.0.12",
               "port" => 6102,
               "zone" => "rack-b"
             })

    assert heartbeat_agent.zone == "rack-b"
  end
end
