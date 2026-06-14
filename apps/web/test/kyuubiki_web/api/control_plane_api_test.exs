defmodule KyuubikiWeb.Api.ControlPlaneApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "exposes orchestrator health" do
    conn =
      :get
      |> conn("/api/health")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)

    assert payload["service"] == "kyuubiki-orchestrator"
    assert payload["status"] == "ok"
    assert payload["protocol"]["program"] == "kyuubiki-orchestrator"
    assert payload["protocol"]["compatible_solver_rpc"]["name"] == "kyuubiki.solver-rpc/v1"
    assert payload["deployment"]["mode"] == "local"
    assert payload["deployment"]["discovery"] == "static"
    assert payload["remote_solver_registry"]["active_agents"] == 0
    assert payload["transport"]["http"] == 4000
    assert payload["transport"]["solver_agent_tcp"] == 5001
  end

  test "exposes decoupled protocol descriptors for control plane and solver rpc" do
    conn =
      :get
      |> conn("/api/v1/protocol")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    assert payload["program"] == "kyuubiki-orchestrator"
    assert payload["protocol"]["name"] == "kyuubiki.control-plane/http-v1"
    assert payload["compatible_solver_rpc"]["name"] == "kyuubiki.solver-rpc/v1"
    assert payload["authority"]["control_mode"] == "orch_managed"
    assert payload["authority"]["authority_mode"] == "single_orchestrator"
    assert payload["authority"]["session_state"] == "orch_bound_pending_session"
    assert payload["authority"]["accepts_multi_orchestrator_binding"] == false

    conn =
      :get
      |> conn("/api/v1/protocol/solver-rpc")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert "describe_agent" in payload["methods"]
    assert payload["transport"]["framing"] == "length_prefixed_u32"
  end

  test "describes reachable solver agents through the protocol endpoint" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => true,
          "result" => %{
            "program" => "kyuubiki-rust-agent",
            "role" => "solver_agent",
            "protocol" => %{
              "name" => "kyuubiki.solver-rpc/v1",
              "rpc_version" => 1,
              "transport" => %{
                "kind" => "tcp",
                "framing" => "length_prefixed_u32",
                "encoding" => "json"
              },
              "methods" => ["ping", "describe_agent", "solve_truss_2d"]
            },
            "capabilities" => [
              %{
                "id" => "truss-2d",
                "role" => "solver",
                "methods" => ["solve_truss_2d"],
                "tags" => ["truss", "cpu"]
              }
            ],
            "deployment_modes" => ["local", "distributed"]
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "protocol-agent", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :get
      |> conn("/api/v1/protocol/agents")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    [agent] = payload["agents"]
    assert agent["id"] == "protocol-agent"
    assert agent["descriptor"]["program"] == "kyuubiki-rust-agent"
    assert "describe_agent" in agent["descriptor"]["protocol"]["methods"]
    assert agent["descriptor"]["authority"]["control_mode"] == "standalone"
    assert agent["descriptor"]["authority"]["authority_mode"] == "self_directed"
  end

  test "registers, heartbeats, and removes remote agents through the API" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: nil,
      cluster_api_token: nil,
      protect_reads?: false
    )

    Application.put_env(:kyuubiki_web, AgentPool, discovery: :registry, endpoints: [])
    AgentPool.reload()
    cluster_ts = Integer.to_string(System.system_time(:millisecond))

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "solver-remote-a",
          "host" => "10.20.0.11",
          "port" => 6101,
          "orch_id" => "orch-alpha",
          "region" => "ap-shanghai"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-agent-id", "solver-remote-a")
      |> put_req_header("x-kyuubiki-cluster-ts", cluster_ts)
      |> put_req_header("x-kyuubiki-cluster-nonce", "nonce-solver-remote-a-register")
      |> Router.call(@opts)

    assert conn.status == 201
    payload = Jason.decode!(conn.resp_body)
    assert payload["agent"]["id"] == "solver-remote-a"
    assert payload["agent"]["authority"]["control_mode"] == "orch_managed"
    assert payload["agent"]["authority"]["authority_mode"] == "single_orchestrator"
    assert payload["agent"]["authority"]["session_state"] == "orch_bound_pending_session"
    assert payload["agent"]["last_session_transition"]["reason"] == "registered"

    conn =
      :post
      |> conn(
        "/api/v1/agents/solver-remote-a/heartbeat",
        Jason.encode!(%{
          "host" => "10.20.0.11",
          "port" => 6101,
          "orch_id" => "orch-alpha",
          "zone" => "rack-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-agent-id", "solver-remote-a")
      |> put_req_header("x-kyuubiki-cluster-ts", cluster_ts)
      |> put_req_header("x-kyuubiki-cluster-nonce", "nonce-solver-remote-a-heartbeat")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["agent"]["zone"] == "rack-a"
    assert Jason.decode!(conn.resp_body)["agent"]["authority"]["authority_mode"] == "single_orchestrator"
    assert Jason.decode!(conn.resp_body)["agent"]["authority"]["session_state"] == "orch_bound_pending_session"

    conn =
      :get
      |> conn("/api/v1/agents")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["summary"]["active_agents"] == 1
    assert payload["summary"]["control_modes"] == %{"orch_managed" => 1, "offline_mesh" => 0}
    assert payload["summary"]["session_states"] == %{"orch_bound_pending_session" => 1}
    assert hd(payload["summary"]["recent_session_transitions"])["agent_id"] == "solver-remote-a"
    assert Enum.map(payload["agents"], & &1["id"]) == ["solver-remote-a"]
    assert hd(payload["agents"])["authority"]["control_mode"] == "orch_managed"
    assert hd(payload["agents"])["authority"]["authority_mode"] == "single_orchestrator"
    assert hd(payload["agents"])["authority"]["session_state"] == "orch_bound_pending_session"
    assert payload["summary"]["mesh_topology"]["managed_orchestrators"] == [
             %{
               "orch_id" => "orch-alpha",
               "agent_count" => 1,
               "agent_ids" => ["solver-remote-a"],
               "session_ids" => []
             }
           ]

    conn =
      :delete
      |> conn("/api/v1/agents/solver-remote-a")
      |> put_req_header("x-kyuubiki-agent-id", "solver-remote-a")
      |> put_req_header("x-kyuubiki-cluster-ts", cluster_ts)
      |> put_req_header("x-kyuubiki-cluster-nonce", "nonce-solver-remote-a-delete")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["status"] == "removed"
  end

  test "rejects API registration that tries to rebind the same fingerprinted agent under a second id" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: nil,
      cluster_api_token: nil,
      protect_reads?: false
    )

    Application.put_env(:kyuubiki_web, AgentPool, discovery: :registry, endpoints: [])
    AgentPool.reload()
    cluster_ts = Integer.to_string(System.system_time(:millisecond))

    first_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "solver-entity-a",
          "host" => "10.20.0.31",
          "port" => 6131,
          "orch_id" => "orch-alpha",
          "fingerprint" => "fp-entity-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-agent-id", "solver-entity-a")
      |> put_req_header("x-kyuubiki-cluster-ts", cluster_ts)
      |> put_req_header("x-kyuubiki-cluster-nonce", "nonce-solver-entity-a-register")
      |> Router.call(@opts)

    assert first_conn.status == 201

    conflict_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "solver-entity-a-shadow",
          "host" => "10.20.0.88",
          "port" => 6888,
          "orch_id" => "orch-beta",
          "fingerprint" => "fp-entity-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-agent-id", "solver-entity-a-shadow")
      |> put_req_header("x-kyuubiki-cluster-ts", cluster_ts)
      |> put_req_header("x-kyuubiki-cluster-nonce", "nonce-solver-entity-a-shadow-register")
      |> Router.call(@opts)

    assert conflict_conn.status == 422

    payload = Jason.decode!(conflict_conn.resp_body)
    assert payload["error"] == "agent_identity_conflict"
    assert payload["conflict"]["agent_id"] == "solver-entity-a-shadow"
    assert payload["conflict"]["current_agent_id"] == "solver-entity-a"
    assert payload["conflict"]["entity_key"]["kind"] == "fingerprint"
    assert payload["conflict"]["entity_key"]["value"] == "fp-entity-a"
    assert payload["conflict"]["current"]["orch_id"] == "orch-alpha"
    assert payload["conflict"]["attempted"]["orch_id"] == "orch-beta"
  end

  test "serves a Hub-facing workload catalog with project download URLs" do
    previous_trust = Application.get_env(:kyuubiki_web, :trust_forwarded_headers, false)
    Application.put_env(:kyuubiki_web, :trust_forwarded_headers, true)

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, :trust_forwarded_headers, previous_trust)
    end)

    {:ok, project} =
      Library.create_project(%{"name" => "Bridge Pack", "description" => "starter"})

    {:ok, _model} =
      Library.create_model(project["project_id"], %{
        "name" => "Bridge Truss",
        "kind" => "truss_2d",
        "payload" => %{
          "model_schema_version" => "kyuubiki.model/v1",
          "kind" => "truss_2d",
          "name" => "Bridge Truss",
          "analysis_metadata" => %{
            "domain" => "thermo_mechanical",
            "family" => "trusses",
            "thermal_intent" => ["nodal_temperature_rise", "truss_thermal_response"]
          },
          "nodes" => [],
          "elements" => []
        }
      })

    conn =
      :get
      |> conn("/api/v1/workloads/catalog")
      |> put_req_header("x-forwarded-proto", "https")
      |> put_req_header("x-forwarded-host", "hub.example.com")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    assert payload["schema_version"] == "kyuubiki.workload-catalog/v1"
    assert payload["sourceLabel"] == "Kyuubiki Control Plane"

    project_id = project["project_id"]

    assert [
             %{
               "label" => "Bridge Pack",
               "project_id" => ^project_id,
               "schema" => "kyuubiki.project/v2",
               "layout" => "kyuubiki.project-layout/v1",
               "model_count" => 1,
               "analysis_domains" => ["thermo_mechanical"],
               "analysis_families" => ["trusses"],
               "thermal_intents" => ["nodal_temperature_rise", "truss_thermal_response"]
             } = workload
           ] = payload["workloads"]

    assert workload["download_url"] ==
             "https://hub.example.com/api/v1/projects/#{project_id}/bundle"
  end

  test "exports a project bundle for Hub workload download" do
    {:ok, project} =
      Library.create_project(%{"name" => "Downloadable Pack", "description" => "bundle"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Space Truss",
        "kind" => "truss_3d",
        "payload" => %{
          "model_schema_version" => "kyuubiki.model/v1",
          "kind" => "truss_3d",
          "name" => "Space Truss",
          "nodes" => [],
          "elements" => []
        }
      })

    {:ok, version} =
      Library.create_version(model["model_id"], %{
        "kind" => "truss_3d",
        "payload" => %{
          "model_schema_version" => "kyuubiki.model/v1",
          "kind" => "truss_3d",
          "name" => "Space Truss v1",
          "nodes" => [],
          "elements" => []
        }
      })

    conn =
      :get
      |> conn("/api/v1/projects/#{project["project_id"]}/bundle")
      |> Router.call(@opts)

    assert conn.status == 200
    assert get_resp_header(conn, "content-type") == ["application/json; charset=utf-8"]
    assert [disposition] = get_resp_header(conn, "content-disposition")
    assert disposition =~ "attachment;"
    assert disposition =~ ".kyuubiki.json"

    payload = Jason.decode!(conn.resp_body)
    model_id = model["model_id"]
    version_id = version["version_id"]
    assert payload["project_schema_version"] == "kyuubiki.project/v2"
    assert payload["project"]["project_id"] == project["project_id"]
    assert payload["project_file_manifest"]["layout_version"] == "kyuubiki.project-layout/v1"
    assert Enum.any?(payload["models"], &(&1["model_id"] == model_id))
    assert Enum.any?(payload["model_versions"], &(&1["version_id"] == version_id))
    assert payload["active_model_id"] == model_id
    assert payload["active_version_id"] == version_id
  end
end
