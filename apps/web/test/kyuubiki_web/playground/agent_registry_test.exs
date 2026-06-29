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
    assert public_agent["authority"]["session_state"] == "orch_bound_pending_session"
    assert public_agent["authority"]["orchestrator_id"] == "orch-alpha"
    assert public_agent["last_session_transition"]["from"] == "unregistered"
    assert public_agent["last_session_transition"]["to"] == "orch_bound_pending_session"
    assert public_agent["last_session_transition"]["reason"] == "registered"
  end

  test "preserves agent watchdog failure reports" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-watchdog-a",
               "host" => "10.20.0.21",
               "port" => 6121,
               "orch_id" => "orch-alpha",
               "watchdog" => %{
                 "state" => "watch",
                 "active_execution_count" => 0,
                 "recent_failure_count" => 1,
                 "recent_failures" => [
                   %{
                     "job_id" => "job-failed-a",
                     "reason_code" => "solve_failed",
                     "message" => "singular stiffness matrix"
                   }
                 ]
               }
             })

    [public_agent] = AgentRegistry.public_agents()
    assert public_agent["watchdog"]["state"] == "watch"
    assert public_agent["watchdog"]["recent_failure_count"] == 1
    assert [%{"job_id" => "job-failed-a"}] = public_agent["watchdog"]["recent_failures"]
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
    public_agent = AgentRegistry.public_agent(heartbeat_agent)
    assert public_agent["session_state"] == "orch_session_bound"
    assert public_agent["last_session_transition"]["source"] == "register"
    assert public_agent["last_session_transition"]["reason"] == "registered"
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

  test "rejects a second agent id that reuses the same fingerprint under another orchestra" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-f",
               "host" => "10.20.0.16",
               "port" => 6106,
               "orch_id" => "orch-alpha",
               "fingerprint" => "fp-shared-a"
             })

    assert {:error, {:agent_identity_conflict, conflict}} =
             AgentRegistry.register(%{
               "id" => "solver-remote-f-shadow",
               "host" => "10.20.0.99",
               "port" => 6999,
               "orch_id" => "orch-beta",
               "fingerprint" => "fp-shared-a"
             })

    assert conflict.current_agent_id == "solver-remote-f"
    assert conflict.entity_key == {:fingerprint, "fp-shared-a"}
    assert conflict.current.orch_id == "orch-alpha"
    assert conflict.attempted.orch_id == "orch-beta"
  end

  test "rejects a second agent id that reuses the same host and port when no fingerprint is present" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-remote-g",
               "host" => "10.20.0.17",
               "port" => 6107,
               "orch_id" => "orch-alpha"
             })

    assert {:error, {:agent_identity_conflict, conflict}} =
             AgentRegistry.register(%{
               "id" => "solver-remote-g-shadow",
               "host" => "10.20.0.17",
               "port" => 6107,
               "control_mode" => "offline_mesh"
             })

    assert conflict.current_agent_id == "solver-remote-g"
    assert conflict.entity_key == {:endpoint, "10.20.0.17", 6107}
    assert conflict.current.control_mode == "orch_managed"
    assert conflict.attempted.control_mode == "offline_mesh"
  end

  test "claims and releases an execution lease for an orchestrated agent" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-lease-a",
               "host" => "10.20.0.18",
               "port" => 6108,
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a"
             })

    assert {:ok, lease} =
             AgentRegistry.claim_execution("solver-lease-a", %{
               "lease_id" => "lease-a",
               "control_mode" => "orch_managed",
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a",
               "job_id" => "job-a",
               "method" => "solve_bar_1d"
             })

    assert lease.agent_id == "solver-lease-a"

    endpoint = hd(AgentRegistry.active_endpoints())
    assert endpoint.active_lease["lease_id"] == "lease-a"
    assert is_integer(endpoint.active_lease["age_ms"])
    assert endpoint.active_lease["is_stale"] == false

    assert length(AgentRegistry.status_snapshot().active_execution_leases) == 1
    assert AgentRegistry.status_snapshot().stale_execution_lease_count == 0

    assert :ok = AgentRegistry.release_execution("solver-lease-a", "lease-a")
    endpoint = hd(AgentRegistry.active_endpoints())
    assert endpoint.active_lease == nil
    assert AgentRegistry.status_snapshot().active_execution_leases == []
  end

  test "rejects a second execution lease while the agent is already claimed" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-lease-b",
               "host" => "10.20.0.19",
               "port" => 6109,
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a"
             })

    assert {:ok, _lease} =
             AgentRegistry.claim_execution("solver-lease-b", %{
               "lease_id" => "lease-a",
               "control_mode" => "orch_managed",
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a"
             })

    assert {:error, {:agent_execution_conflict, conflict}} =
             AgentRegistry.claim_execution("solver-lease-b", %{
               "lease_id" => "lease-b",
               "control_mode" => "orch_managed",
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a"
             })

    assert conflict.agent_id == "solver-lease-b"
    assert conflict.current["lease_id"] == "lease-a"
    assert conflict.attempted["lease_id"] == "lease-b"
  end

  test "marks old execution leases as stale in public views" do
    original_config = Application.get_env(:kyuubiki_web, AgentRegistry, [])

    Application.put_env(
      :kyuubiki_web,
      AgentRegistry,
      Keyword.merge(original_config, execution_lease_stale_after_ms: 1)
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentRegistry, original_config)
    end)

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-lease-stale",
               "host" => "10.20.0.20",
               "port" => 6110,
               "orch_id" => "orch-alpha"
             })

    assert {:ok, _lease} =
             AgentRegistry.claim_execution("solver-lease-stale", %{
               "lease_id" => "lease-stale-a",
               "control_mode" => "orch_managed",
               "orch_id" => "orch-alpha"
             })

    Process.sleep(5)

    [agent] = AgentRegistry.public_agents()
    assert agent["execution_state"] == "lease_stale"
    assert agent["active_lease"]["is_stale"] == true
    assert agent["active_lease"]["age_ms"] >= 1

    snapshot = AgentRegistry.status_snapshot()
    assert snapshot.stale_execution_lease_count == 1
    assert hd(snapshot.active_execution_leases)["is_stale"] == true
  end

  test "summarizes mesh topology across managed and offline agents" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-a",
               "host" => "10.20.1.10",
               "port" => 6301,
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a1"
             })

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-b",
               "host" => "10.20.1.11",
               "port" => 6302,
               "orch_id" => "orch-alpha"
             })

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-c",
               "host" => "10.20.1.12",
               "port" => 6303,
               "orch_id" => "orch-beta",
               "orch_session_id" => "session-b1"
             })

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-d",
               "host" => "10.20.1.13",
               "port" => 6304,
               "control_mode" => "offline_mesh"
             })

    snapshot = AgentRegistry.status_snapshot()

    assert snapshot.control_modes == %{orch_managed: 3, offline_mesh: 1}

    assert snapshot.session_states == %{
             "offline_mesh" => 1,
             "orch_bound_pending_session" => 1,
             "orch_session_bound" => 2
           }

    assert length(snapshot.recent_session_transitions) == 4

    assert Enum.any?(snapshot.recent_session_transitions, fn event ->
             event["agent_id"] == "solver-mesh-d" and
               event["to"] == "offline_mesh" and
               event["reason"] == "registered"
           end)

    assert Enum.any?(snapshot.authority_groups, fn group ->
             group.control_mode == "orch_managed" and
               group.orch_id == "orch-alpha" and
               group.orch_session_id == nil and
               group.agent_ids == ["solver-mesh-b"]
           end)

    assert Enum.any?(snapshot.authority_groups, fn group ->
             group.control_mode == "offline_mesh" and
               group.orch_id == nil and
               group.agent_ids == ["solver-mesh-d"]
           end)

    assert snapshot.mesh_topology.offline_mesh.agent_count == 1
    assert snapshot.mesh_topology.offline_mesh.agent_ids == ["solver-mesh-d"]
    assert snapshot.mesh_topology.offline_mesh.clustered_meshes == []
    assert snapshot.mesh_topology.offline_mesh.unclustered_agent_ids == ["solver-mesh-d"]

    assert snapshot.mesh_topology.managed_orchestrators == [
             %{
               orch_id: "orch-alpha",
               agent_count: 2,
               agent_ids: ["solver-mesh-a", "solver-mesh-b"],
               session_ids: ["session-a1"]
             },
             %{
               orch_id: "orch-beta",
               agent_count: 1,
               agent_ids: ["solver-mesh-c"],
               session_ids: ["session-b1"]
             }
           ]
  end

  test "publishes offline mesh peer groups and clustered relay candidates" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-peer-a",
               "host" => "10.20.2.10",
               "port" => 6401,
               "control_mode" => "offline_mesh",
               "cluster_id" => "mesh-alpha",
               "health_score" => 91,
               "capacity" => 2,
               "tags" => ["mesh", "relay"]
             })

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-peer-b",
               "host" => "10.20.2.11",
               "port" => 6402,
               "control_mode" => "offline_mesh",
               "cluster_id" => "mesh-alpha",
               "health_score" => 72
             })

    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-mesh-peer-c",
               "host" => "10.20.2.12",
               "port" => 6403,
               "control_mode" => "offline_mesh",
               "cluster_id" => "mesh-beta",
               "health_score" => 44
             })

    agents = AgentRegistry.public_agents()
    peer_a = Enum.find(agents, &(&1["id"] == "solver-mesh-peer-a"))
    peer_b = Enum.find(agents, &(&1["id"] == "solver-mesh-peer-b"))

    assert peer_a["mesh"]["cluster_id"] == "mesh-alpha"
    assert peer_a["mesh"]["peer_group_id"] == "offline_mesh:mesh-alpha"
    assert peer_a["mesh"]["peer_count"] == 1
    assert peer_a["mesh"]["relay_candidate"] == true
    assert peer_a["mesh"]["topology_role"] == "relay_candidate"
    assert Enum.map(peer_a["mesh"]["peers"], & &1["id"]) == ["solver-mesh-peer-b"]
    assert hd(peer_a["mesh"]["peers"])["status"] == "degraded"

    assert peer_b["mesh"]["peer_group_id"] == "offline_mesh:mesh-alpha"
    assert Enum.map(peer_b["mesh"]["peers"], & &1["id"]) == ["solver-mesh-peer-a"]

    snapshot = AgentRegistry.status_snapshot()

    assert snapshot.mesh_topology.offline_mesh.clustered_meshes == [
             %{
               cluster_id: "mesh-alpha",
               agent_count: 2,
               agent_ids: ["solver-mesh-peer-a", "solver-mesh-peer-b"],
               relay_candidate_ids: ["solver-mesh-peer-a"]
             },
             %{
               cluster_id: "mesh-beta",
               agent_count: 1,
               agent_ids: ["solver-mesh-peer-c"],
               relay_candidate_ids: []
             }
           ]
  end
end
