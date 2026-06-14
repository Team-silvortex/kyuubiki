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
               "orch_id" => "orch-alpha",
               "region" => "ap-shanghai"
             })

    assert agent.id == "solver-remote-a"
    assert agent.control_mode == "orch_managed"
    assert agent.orch_id == "orch-alpha"
    assert AgentRegistry.status_snapshot().active_agents == 1
    assert Enum.map(AgentRegistry.active_endpoints(), & &1.id) == ["solver-remote-a"]

    public_agent = AgentRegistry.public_agent(agent)
    assert public_agent["authority"]["control_mode"] == "orch_managed"
    assert public_agent["authority"]["authority_mode"] == "single_orchestrator"
    assert public_agent["authority"]["orchestrator_id"] == "orch-alpha"
  end

  test "refreshes last seen through heartbeat" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-b",
               "host" => "10.20.0.12",
               "port" => 6102,
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-1"
             })

    assert {:ok, heartbeat_agent} =
             AgentRegistry.heartbeat("solver-remote-b", %{
               "host" => "10.20.0.12",
               "port" => 6102,
               "orch_id" => "orch-alpha",
               "zone" => "rack-b"
             })

    assert heartbeat_agent.zone == "rack-b"
    assert heartbeat_agent.orch_session_id == "session-1"
  end

  test "keeps capability and health metadata for managed remote agents" do
    assert {:ok, agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-c",
               "host" => "10.20.0.13",
               "port" => 6103,
               "orch_id" => "orch-alpha",
               "methods" => ["solve_truss_2d", "cancel_job"],
               "health_score" => 88,
               "capabilities" => [
                 %{
                   "id" => "truss-2d",
                   "role" => "solver",
                   "methods" => ["solve_truss_2d"],
                   "tags" => ["truss", "2d"]
                 }
               ]
             })

    assert agent.health_score == 88
    assert agent.methods == ["solve_truss_2d", "cancel_job"]

    assert [%{id: "truss-2d", methods: ["solve_truss_2d"], tags: ["truss", "2d"]}] =
             agent.capabilities

    endpoint = hd(AgentRegistry.active_endpoints())
    assert endpoint.health_score == 88
    assert endpoint.methods == ["solve_truss_2d", "cancel_job"]
    assert endpoint.control_mode == "orch_managed"
    assert endpoint.orch_id == "orch-alpha"
    assert [%{id: "truss-2d"}] = endpoint.capabilities
  end

  test "rejects offline mesh registrations carrying orchestra identity" do
    assert {:error, {:invalid_agent_control, :offline_mesh_cannot_bind_orchestra}} =
             AgentRegistry.register(%{
               "id" => "solver-offline-a",
               "host" => "10.20.0.21",
               "port" => 6201,
               "control_mode" => "offline_mesh",
               "orch_id" => "orch-alpha"
             })
  end

  test "rejects heartbeats that try to switch the bound orchestra" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-d",
               "host" => "10.20.0.14",
               "port" => 6104,
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-1"
             })

    assert {:error, {:agent_control_conflict, conflict}} =
             AgentRegistry.heartbeat("solver-remote-d", %{
               "host" => "10.20.0.14",
               "port" => 6104,
               "orch_id" => "orch-beta",
               "orch_session_id" => "session-9"
             })

    assert conflict.agent_id == "solver-remote-d"
    assert conflict.current.orch_id == "orch-alpha"
    assert conflict.attempted.orch_id == "orch-beta"
  end

  test "rejects heartbeats that flip an orchestra-managed agent into offline mesh" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-e",
               "host" => "10.20.0.15",
               "port" => 6105,
               "orch_id" => "orch-alpha"
             })

    assert {:error, {:agent_control_conflict, conflict}} =
             AgentRegistry.heartbeat("solver-remote-e", %{
               "host" => "10.20.0.15",
               "port" => 6105,
               "control_mode" => "offline_mesh"
             })

    assert conflict.current.control_mode == "orch_managed"
    assert conflict.attempted.control_mode == "offline_mesh"
  end
end
