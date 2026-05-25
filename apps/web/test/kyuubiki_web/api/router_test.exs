defmodule KyuubikiWeb.Playground.RouterTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Playground.AgentRegistry
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Router
  alias KyuubikiWeb.SecurityEvents.Store, as: SecurityEventStore
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  @opts Router.init([])

  setup do
    Store.reset()
    AnalysisResultStore.reset()
    Library.reset()
    SecurityEventStore.reset()
    Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)

    original_config = Application.get_env(:kyuubiki_web, AgentPool, [])
    original_security = Application.get_env(:kyuubiki_web, KyuubikiWeb.Security, [])

    on_exit(fn ->
      Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)
      Application.put_env(:kyuubiki_web, AgentPool, original_config)
      Application.put_env(:kyuubiki_web, KyuubikiWeb.Security, original_security)
      AgentPool.reload()
    end)

    :ok
  end

  test "rejects mutating routes without an API token when control-plane protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Locked Project", "description" => "protected"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
  end

  test "accepts mutating routes with a valid bearer token when control-plane protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Protected Project", "description" => "authorized"})
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("authorization", "Bearer secret-token")
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["project"]["name"] == "Protected Project"
  end

  test "rejects read routes without an API token when read protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: true
    )

    conn =
      :get
      |> conn("/api/v1/projects")
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
  end

  test "accepts read routes with a valid token when read protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: true
    )

    conn =
      :get
      |> conn("/api/health")
      |> put_req_header("x-kyuubiki-token", "secret-token")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["status"] == "ok"
  end

  test "keeps read routes open when read protection is disabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :get
      |> conn("/api/v1/export/database")
      |> Router.call(@opts)

    assert conn.status == 200
    assert is_map(Jason.decode!(conn.resp_body))
  end

  test "submits a spring 2d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/spring-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 1000.0,
              "load_y" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "s0",
              "node_i" => 0,
              "node_j" => 1,
              "stiffness" => 25_000.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a spring 3d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/spring-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 1000.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "s0",
              "node_i" => 0,
              "node_j" => 1,
              "stiffness" => 25_000.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal truss 2d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-truss-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            }
          ],
          "elements" => [
            %{
              "id" => "tt0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.01,
              "youngs_modulus" => 210.0e9,
              "thermal_expansion" => 12.0e-6
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal truss 3d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-truss-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n2",
              "x" => 0.0,
              "y" => 1.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0,
              "temperature_delta" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "tt0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.01,
              "youngs_modulus" => 210.0e9,
              "thermal_expansion" => 12.0e-6
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal beam 1d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-beam-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "tb0",
              "node_i" => 0,
              "node_j" => 1,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4,
              "thermal_expansion" => 1.2e-5,
              "section_depth" => 0.2,
              "distributed_load_y" => 0.0,
              "temperature_gradient_y" => 40.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal frame 2d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-frame-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_rz" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "moment_z" => 0.0,
              "temperature_delta" => 35.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_rz" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "moment_z" => 0.0,
              "temperature_delta" => 35.0
            }
          ],
          "elements" => [
            %{
              "id" => "tf0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4,
              "thermal_expansion" => 1.2e-5,
              "section_depth" => 0.2,
              "temperature_gradient_y" => 30.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["status"] in ["queued", "running", "completed"]
  end

  test "protects cluster registration routes with a dedicated cluster token" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "cluster-secret",
      cluster_api_token: "cluster-only-secret",
      protect_reads?: false
    )

    unauthorized_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-a",
          "host" => "127.0.0.1",
          "port" => 5001,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert unauthorized_conn.status == 401

    authorized_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-a",
          "host" => "127.0.0.1",
          "port" => 5001,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-a")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert authorized_conn.status == 201
    assert Jason.decode!(authorized_conn.resp_body)["agent"]["id"] == "remote-a"
  end

  test "falls back to control-plane token for cluster routes when no dedicated cluster token is configured" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-b",
          "host" => "127.0.0.1",
          "port" => 5002,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "shared-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-b")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["agent"]["id"] == "remote-b"
  end

  test "rejects stale cluster timestamps on protected cluster routes" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_timestamp_window_ms: 1_000,
      protect_reads?: false
    )

    stale_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-c",
          "host" => "127.0.0.1",
          "port" => 5003,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-c")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond) - 10_000)
      )
      |> Router.call(@opts)

    assert stale_conn.status == 401
    assert Jason.decode!(stale_conn.resp_body)["error"] == "stale_cluster_request"
  end

  test "allows protected cluster registration when agent and cluster IDs are allowlisted" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_allowed_agent_ids: MapSet.new(["remote-d"]),
      cluster_allowed_cluster_ids: MapSet.new(["lan-a"]),
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-d",
          "host" => "127.0.0.1",
          "port" => 5004,
          "role" => "solver",
          "cluster_id" => "lan-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-d")
      |> put_req_header("x-kyuubiki-cluster-id", "lan-a")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["agent"]["id"] == "remote-d"
  end

  test "rejects protected cluster registration when agent ID is not allowlisted" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_allowed_agent_ids: MapSet.new(["remote-allowed"]),
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-denied",
          "host" => "127.0.0.1",
          "port" => 5005,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-denied")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "rejects protected cluster heartbeat when cluster ID is not allowlisted" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_allowed_agent_ids: MapSet.new(["remote-e"]),
      cluster_allowed_cluster_ids: MapSet.new(["lan-ok"]),
      protect_reads?: false
    )

    register_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-e",
          "host" => "127.0.0.1",
          "port" => 5006,
          "role" => "solver",
          "cluster_id" => "lan-ok"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-e")
      |> put_req_header("x-kyuubiki-cluster-id", "lan-ok")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert register_conn.status == 201

    heartbeat_conn =
      :post
      |> conn(
        "/api/v1/agents/remote-e/heartbeat",
        Jason.encode!(%{"cluster_id" => "lan-other"})
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-e")
      |> put_req_header("x-kyuubiki-cluster-id", "lan-other")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert heartbeat_conn.status == 401
    assert Jason.decode!(heartbeat_conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "rejects protected cluster registration when fingerprint is required but missing" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_require_fingerprint?: true,
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-fp-a",
          "host" => "127.0.0.1",
          "port" => 5007,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-fp-a")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "rejects protected cluster heartbeat when fingerprint does not match the registered agent" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_require_fingerprint?: true,
      protect_reads?: false
    )

    register_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-fp-b",
          "host" => "127.0.0.1",
          "port" => 5008,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-fp-b")
      |> put_req_header("x-kyuubiki-agent-fingerprint", "fp-good")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert register_conn.status == 201

    heartbeat_conn =
      :post
      |> conn("/api/v1/agents/remote-fp-b/heartbeat", Jason.encode!(%{}))
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-fp-b")
      |> put_req_header("x-kyuubiki-agent-fingerprint", "fp-bad")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert heartbeat_conn.status == 401
    assert Jason.decode!(heartbeat_conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "supports CRUD for projects, models, and model versions" do
    create_project_conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Bridge Study", "description" => "macOS local"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_project_conn.status == 201
    project = Jason.decode!(create_project_conn.resp_body)["project"]
    project_id = project["project_id"]

    create_model_conn =
      :post
      |> conn(
        "/api/v1/projects/#{project_id}/models",
        Jason.encode!(%{
          "name" => "Roof Truss",
          "kind" => "truss_2d",
          "material" => "Steel",
          "model_schema_version" => "kyuubiki.model/v1",
          "payload" => %{
            "kind" => "truss_2d",
            "model_schema_version" => "kyuubiki.model/v1",
            "name" => "Roof Truss",
            "material" => "Steel",
            "youngs_modulus_gpa" => 210,
            "nodes" => [],
            "elements" => []
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_model_conn.status == 201
    model = Jason.decode!(create_model_conn.resp_body)["model"]
    model_id = model["model_id"]
    assert model["latest_version_number"] == 1

    create_version_conn =
      :post
      |> conn(
        "/api/v1/models/#{model_id}/versions",
        Jason.encode!(%{
          "name" => "Checkpoint A",
          "payload" => %{
            "kind" => "truss_2d",
            "model_schema_version" => "kyuubiki.model/v1",
            "name" => "Roof Truss v2",
            "material" => "Steel",
            "youngs_modulus_gpa" => 210,
            "nodes" => [%{"id" => "n0", "x" => 0.0, "y" => 0.0}],
            "elements" => []
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_version_conn.status == 201
    version = Jason.decode!(create_version_conn.resp_body)["version"]
    assert version["version_number"] == 2

    projects_conn =
      :get
      |> conn("/api/v1/projects")
      |> Router.call(@opts)

    assert projects_conn.status == 200
    projects = Jason.decode!(projects_conn.resp_body)["projects"]
    assert length(projects) == 1
    assert hd(projects)["models"] |> length() == 1

    get_model_conn =
      :get
      |> conn("/api/v1/models/#{model_id}")
      |> Router.call(@opts)

    assert get_model_conn.status == 200
    returned_model = Jason.decode!(get_model_conn.resp_body)["model"]
    assert length(returned_model["versions"]) == 2

    update_project_conn =
      :patch
      |> conn(
        "/api/v1/projects/#{project_id}",
        Jason.encode!(%{"name" => "Bridge Study Updated"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_project_conn.status == 200

    assert Jason.decode!(update_project_conn.resp_body)["project"]["name"] ==
             "Bridge Study Updated"

    delete_version_conn =
      :delete
      |> conn("/api/v1/model-versions/#{version["version_id"]}")
      |> Router.call(@opts)

    assert delete_version_conn.status == 200

    delete_model_conn =
      :delete
      |> conn("/api/v1/models/#{model_id}")
      |> Router.call(@opts)

    assert delete_model_conn.status == 200

    delete_project_conn =
      :delete
      |> conn("/api/v1/projects/#{project_id}")
      |> Router.call(@opts)

    assert delete_project_conn.status == 200
  end

  test "cancels an active job through the API" do
    {:ok, job} =
      Store.create(%{
        job_id: "job-cancel",
        project_id: "project-cancel",
        simulation_case_id: "case-cancel"
      })

    assert job.status == :queued

    Store.apply_progress(%{
      job_id: "job-cancel",
      stage: "solving",
      progress: 0.6,
      message: "solving structural system"
    })

    cancel_conn =
      :post
      |> conn("/api/v1/jobs/job-cancel/cancel")
      |> Router.call(@opts)

    assert cancel_conn.status == 200

    payload = Jason.decode!(cancel_conn.resp_body)
    assert payload["job"]["status"] == "cancelled"
    assert payload["job"]["message"] == "job cancelled by operator"

    assert {:ok, cancelled_job} = Store.get("job-cancel")
    assert cancelled_job.status == :cancelled
  end

  test "runs an axial bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving axial system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "tip_displacement" => 4.761904761904762e-7,
            "reaction_force" => -1000.0,
            "max_displacement" => 4.761904761904762e-7,
            "max_stress" => 100_000.0,
            "nodes" => [
              %{"index" => 0, "x" => 0.0, "displacement" => 0.0},
              %{"index" => 1, "x" => 1.0, "displacement" => 4.761904761904762e-7}
            ],
            "elements" => [
              %{
                "index" => 0,
                "x1" => 0.0,
                "x2" => 1.0,
                "strain" => 4.761904761904762e-7,
                "stress" => 100_000.0,
                "axial_force" => 1000.0
              }
            ],
            "input" => %{
              "length" => 1.0,
              "area" => 0.01,
              "youngs_modulus" => 210.0e9,
              "elements" => 1,
              "tip_force" => 1000.0
            }
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/axial-bar/jobs",
        Jason.encode!(%{
          "length" => 1.0,
          "area" => 0.01,
          "youngs_modulus_gpa" => 210,
          "elements" => 3,
          "tip_force" => 1000
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    job_id = payload["job"]["job_id"]

    assert payload["job"]["status"] == "queued"

    result_payload = wait_for_job(job_id)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["job"]["worker_id"] == "rust-agent-rpc@agent-a"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.65,
            "iteration" => 2,
            "message" => "solving thermal bar"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "ux" => 0.0, "temperature_delta" => 40.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "ux" => 0.0, "temperature_delta" => 40.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "average_temperature_delta" => 40.0,
                "thermal_strain" => 4.8e-4,
                "mechanical_strain" => -4.8e-4,
                "total_strain" => 0.0,
                "stress" => -1.008e8,
                "axial_force" => -1.008e6
              }
            ],
            "max_displacement" => 0.0,
            "max_stress" => 1.008e8,
            "max_axial_force" => 1.008e6,
            "max_temperature_delta" => 40.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-bar-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_x" => true,
              "load_x" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "fix_x" => true,
              "load_x" => 0.0,
              "temperature_delta" => 40.0
            }
          ],
          "elements" => [
            %{
              "id" => "tb0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.01,
              "youngs_modulus" => 2.1e11,
              "thermal_expansion" => 1.2e-5
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_axial_force"] > 0
    assert result_payload["result"]["max_temperature_delta"] == 40.0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a heat bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving heat bar"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "temperature" => 100.0, "heat_load" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "temperature" => 0.0, "heat_load" => 0.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "hb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "average_temperature" => 50.0,
                "temperature_gradient" => -100.0,
                "heat_flux" => 5000.0
              }
            ],
            "max_temperature" => 100.0,
            "max_heat_flux" => 5000.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/heat-bar-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_temperature" => true,
              "temperature" => 100.0,
              "heat_load" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "fix_temperature" => true,
              "temperature" => 0.0,
              "heat_load" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "hb0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "conductivity" => 50.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_temperature"] == 100.0
    assert result_payload["result"]["max_heat_flux"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "binds a submitted job to the selected model version" do
    {:ok, project} = Library.create_project(%{"name" => "Version-bound study"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Saved roof",
        "kind" => "truss_2d",
        "model_schema_version" => "kyuubiki.model/v1",
        "payload" => %{
          "kind" => "truss_2d",
          "model_schema_version" => "kyuubiki.model/v1",
          "name" => "Saved roof",
          "material" => "Steel",
          "youngs_modulus_gpa" => 210,
          "nodes" => [],
          "elements" => []
        }
      })

    {:ok, version} =
      Library.create_version(model["model_id"], %{
        "name" => "Frozen solve input",
        "kind" => "truss_2d",
        "model_schema_version" => "kyuubiki.model/v1",
        "payload" => %{
          "kind" => "truss_2d",
          "model_schema_version" => "kyuubiki.model/v1",
          "name" => "Saved roof",
          "material" => "Steel",
          "youngs_modulus_gpa" => 210,
          "nodes" => [],
          "elements" => []
        }
      })

    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [],
            "elements" => [],
            "max_displacement" => 0.0,
            "max_stress" => 0.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/truss-2d/jobs",
        Jason.encode!(%{
          "project_id" => "ignored-project-id",
          "model_version_id" => version["version_id"],
          "nodes" => [],
          "elements" => []
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["project_id"] == project["project_id"]
    assert payload["job"]["model_version_id"] == version["version_id"]
    assert payload["job"]["simulation_case_id"] == version["version_id"]
  end

  test "runs a two dimensional truss job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving structural system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 2, "id" => "n2", "x" => 0.5, "y" => 0.75, "ux" => 0.0, "uy" => -1.0e-6}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "e0",
                "node_i" => 0,
                "node_j" => 2,
                "length" => 0.9,
                "strain" => 1.0e-6,
                "stress" => 7.0e4,
                "axial_force" => 700.0
              }
            ],
            "max_displacement" => 1.0e-6,
            "max_stress" => 7.0e4,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/truss-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 0.5,
              "y" => 0.75,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0
            }
          ],
          "elements" => [
            %{
              "id" => "e0",
              "node_i" => 0,
              "node_j" => 2,
              "area" => 0.01,
              "youngs_modulus" => 7.0e10
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["nodes"]) == 3
  end

  test "serves chunked result windows for large payloads" do
    {:ok, _job} =
      Store.create(%{
        job_id: "job-chunked",
        project_id: "project-chunked",
        simulation_case_id: "case-chunked"
      })

    :ok =
      AnalysisResultStore.put("job-chunked", %{
        "nodes" => Enum.map(0..9, &%{"index" => &1, "id" => "n#{&1}"}),
        "elements" => Enum.map(0..4, &%{"index" => &1, "id" => "e#{&1}"}),
        "max_displacement" => 0.0,
        "max_stress" => 0.0
      })

    conn =
      :get
      |> conn("/api/v1/results/job-chunked/chunks/nodes?offset=3&limit=4")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    assert payload["kind"] == "nodes"
    assert payload["offset"] == 3
    assert payload["limit"] == 4
    assert payload["returned"] == 4
    assert payload["total"] == 10
    assert Enum.map(payload["items"], & &1["index"]) == [3, 4, 5, 6]
  end

  test "runs a two dimensional plane triangle job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving plane stress system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 1.0e-7, "uy" => 0.0},
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 1.0,
                "y" => 1.0,
                "ux" => 1.2e-7,
                "uy" => -2.0e-7
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "p0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "strain_x" => 1.0e-7,
                "strain_y" => -2.0e-7,
                "gamma_xy" => 0.0,
                "stress_x" => 5.0e3,
                "stress_y" => -3.0e3,
                "tau_xy" => 0.0,
                "von_mises" => 6.0e3
              }
            ],
            "max_displacement" => 2.3e-7,
            "max_stress" => 6.0e3,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/plane-triangle-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0
            }
          ],
          "elements" => [
            %{
              "id" => "p0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a plane quad job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.65,
            "iteration" => 2,
            "message" => "assembling quad plane system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 8.0e-8, "uy" => 0.0},
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 1.0,
                "y" => 1.0,
                "ux" => 1.1e-7,
                "uy" => -2.1e-7
              },
              %{
                "index" => 3,
                "id" => "n3",
                "x" => 0.0,
                "y" => 1.0,
                "ux" => 0.0,
                "uy" => -8.0e-8
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "q0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "strain_x" => 8.0e-8,
                "strain_y" => -2.1e-7,
                "gamma_xy" => 0.0,
                "stress_x" => 4.8e3,
                "stress_y" => -3.2e3,
                "tau_xy" => 0.0,
                "von_mises" => 6.4e3
              }
            ],
            "max_displacement" => 2.4e-7,
            "max_stress" => 6.4e3,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/plane-quad-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0
            },
            %{
              "id" => "n3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "q0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal plane triangle job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.6,
            "iteration" => 2,
            "message" => "assembling thermal plane triangle system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 2, "id" => "n2", "x" => 0.0, "y" => 1.0, "ux" => 0.0, "uy" => 0.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tp0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "temperature_delta" => 40.0,
                "average_temperature_delta" => 40.0,
                "thermal_strain_x" => 4.8e-4,
                "thermal_strain_y" => 4.8e-4,
                "mechanical_strain_x" => -4.8e-4,
                "mechanical_strain_y" => -4.8e-4,
                "mechanical_gamma_xy" => 0.0,
                "total_strain_x" => 0.0,
                "total_strain_y" => 0.0,
                "total_gamma_xy" => 0.0,
                "stress_x" => -5.0e7,
                "stress_y" => -5.0e7,
                "tau_xy" => 0.0,
                "von_mises" => 5.0e7
              }
            ],
            "max_displacement" => 0.0,
            "max_stress" => 5.0e7,
            "max_temperature_delta" => 40.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-plane-triangle-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n2",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            }
          ],
          "elements" => [
            %{
              "id" => "tp0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33,
              "thermal_expansion" => 12.0e-6
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_temperature_delta"] == 40.0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal plane quad job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.6,
            "iteration" => 2,
            "message" => "assembling thermal plane quad system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 2, "id" => "n2", "x" => 1.0, "y" => 1.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 3, "id" => "n3", "x" => 0.0, "y" => 1.0, "ux" => 0.0, "uy" => 0.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "temperature_delta" => 30.0,
                "average_temperature_delta" => 30.0,
                "thermal_strain_x" => 3.3e-4,
                "thermal_strain_y" => 3.3e-4,
                "mechanical_strain_x" => -3.3e-4,
                "mechanical_strain_y" => -3.3e-4,
                "mechanical_gamma_xy" => 0.0,
                "total_strain_x" => 0.0,
                "total_strain_y" => 0.0,
                "total_gamma_xy" => 0.0,
                "stress_x" => -3.4e7,
                "stress_y" => -3.4e7,
                "tau_xy" => 0.0,
                "von_mises" => 3.4e7
              }
            ],
            "max_displacement" => 0.0,
            "max_stress" => 3.4e7,
            "max_temperature_delta" => 30.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-plane-quad-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            },
            %{
              "id" => "n3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            }
          ],
          "elements" => [
            %{
              "id" => "tq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33,
              "thermal_expansion" => 11.0e-6
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_temperature_delta"] == 30.0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a frame 2d job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.55,
            "iteration" => 3,
            "message" => "assembling frame stiffness"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "y" => 0.0,
                "ux" => 0.0,
                "uy" => -0.0015873015873015873,
                "rz" => -0.0011904761904761906,
                "displacement_magnitude" => 0.0015873015873015873
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "f0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "axial_force_i" => 0.0,
                "shear_force_i" => 1000.0,
                "moment_i" => 2000.0,
                "axial_force_j" => 0.0,
                "shear_force_j" => -1000.0,
                "moment_j" => 0.0,
                "axial_stress" => 0.0,
                "max_bending_stress" => 1.25e7,
                "max_combined_stress" => 1.25e7
              }
            ],
            "max_displacement" => 0.0015873015873015873,
            "max_rotation" => 0.0011904761904761906,
            "max_moment" => 2000.0,
            "max_stress" => 1.25e7,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/frame-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_rz" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => false,
              "fix_rz" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "f0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a frame 3d job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.55,
            "iteration" => 3,
            "message" => "assembling frame stiffness"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0,
                "rx" => 0.0,
                "ry" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0,
                "rotation_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "y" => 0.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => -0.0015873015873015873,
                "uz" => 0.0,
                "rx" => 0.0,
                "ry" => 0.0,
                "rz" => -0.0011904761904761906,
                "displacement_magnitude" => 0.0015873015873015873,
                "rotation_magnitude" => 0.0011904761904761906
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "f0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "axial_force_i" => 0.0,
                "shear_force_y_i" => 1000.0,
                "shear_force_z_i" => 0.0,
                "torsion_i" => 0.0,
                "moment_y_i" => 0.0,
                "moment_z_i" => 2000.0,
                "axial_force_j" => 0.0,
                "shear_force_y_j" => -1000.0,
                "shear_force_z_j" => 0.0,
                "torsion_j" => 0.0,
                "moment_y_j" => 0.0,
                "moment_z_j" => 0.0,
                "axial_stress" => 0.0,
                "max_bending_stress" => 1.25e7,
                "max_combined_stress" => 1.25e7
              }
            ],
            "max_displacement" => 0.0015873015873015873,
            "max_rotation" => 0.0011904761904761906,
            "max_moment" => 2000.0,
            "max_stress" => 1.25e7,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/frame-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "fix_rx" => true,
              "fix_ry" => true,
              "fix_rz" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0,
              "moment_x" => 0.0,
              "moment_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => false,
              "fix_y" => false,
              "fix_z" => false,
              "fix_rx" => false,
              "fix_ry" => false,
              "fix_rz" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0,
              "load_z" => 0.0,
              "moment_x" => 0.0,
              "moment_y" => 0.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "f0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "youngs_modulus" => 2.1e11,
              "shear_modulus" => 8.0e10,
              "torsion_constant" => 5.0e-6,
              "moment_of_inertia_y" => 8.0e-6,
              "moment_of_inertia_z" => 8.0e-6,
              "section_modulus_y" => 1.6e-4,
              "section_modulus_z" => 1.6e-4
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_rotation"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a one dimensional beam job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.55,
            "iteration" => 3,
            "message" => "assembling beam stiffness"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "uy" => 0.0015873015873015873,
                "rz" => 0.0011904761904761906,
                "displacement_magnitude" => 0.0015873015873015873
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "b0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "shear_force_i" => 1000.0,
                "moment_i" => 2000.0,
                "shear_force_j" => -1000.0,
                "moment_j" => 0.0,
                "max_bending_stress" => 1.25e7
              }
            ],
            "max_displacement" => 0.0015873015873015873,
            "max_rotation" => 0.0011904761904761906,
            "max_moment" => 2000.0,
            "max_stress" => 1.25e7,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/beam-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "fix_y" => false,
              "fix_rz" => false,
              "load_y" => -1000.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "b0",
              "node_i" => 0,
              "node_j" => 1,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal beam 1d job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving thermal beam"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "temperature_gradient_y" => 40.0,
                "thermal_curvature" => 0.0024,
                "shear_force_i" => 0.0,
                "moment_i" => 4032.0,
                "shear_force_j" => 0.0,
                "moment_j" => -4032.0,
                "max_bending_stress" => 2.52e7
              }
            ],
            "max_displacement" => 0.0,
            "max_rotation" => 0.0,
            "max_moment" => 4032.0,
            "max_stress" => 2.52e7,
            "max_temperature_gradient" => 40.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-beam-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "tb0",
              "node_i" => 0,
              "node_j" => 1,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4,
              "thermal_expansion" => 1.2e-5,
              "section_depth" => 0.2,
              "distributed_load_y" => 0.0,
              "temperature_gradient_y" => 40.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_temperature_gradient"] == 40.0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a one dimensional torsion job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 2,
            "message" => "solving torsion shaft"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "rz" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 2.0, "rz" => 0.01}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "t0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "twist" => 0.01,
                "torque" => 1200.0,
                "shear_stress" => 6.0e6
              }
            ],
            "max_rotation" => 0.01,
            "max_torque" => 1200.0,
            "max_stress" => 6.0e6,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/torsion-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "fix_rz" => true, "torque_z" => 0.0},
            %{"id" => "n1", "x" => 2.0, "fix_rz" => false, "torque_z" => 1200.0}
          ],
          "elements" => [
            %{
              "id" => "t0",
              "node_i" => 0,
              "node_j" => 1,
              "shear_modulus" => 8.0e10,
              "polar_moment" => 3.0e-6,
              "section_modulus" => 2.0e-4
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_torque"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a one dimensional spring job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.75,
            "iteration" => 2,
            "message" => "solving spring chain"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "ux" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "ux" => 0.04}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "s0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "extension" => 0.04,
                "force" => 1000.0
              }
            ],
            "max_displacement" => 0.04,
            "max_force" => 1000.0
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/spring-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "fix_x" => true, "load_x" => 0.0},
            %{"id" => "n1", "x" => 1.0, "fix_x" => false, "load_x" => 1000.0}
          ],
          "elements" => [
            %{"id" => "s0", "node_i" => 0, "node_j" => 1, "stiffness" => 25_000.0}
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert result_payload["result"]["max_force"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a three dimensional truss job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving spatial truss system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 1.0,
                "y" => 0.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0
              },
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 0.0,
                "y" => 1.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0
              },
              %{
                "index" => 3,
                "id" => "n3",
                "x" => 0.2,
                "y" => 0.2,
                "z" => 1.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => -1.0e-6
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "e0",
                "node_i" => 0,
                "node_j" => 3,
                "length" => 1.0,
                "strain" => 1.0e-6,
                "stress" => 7.0e4,
                "axial_force" => 700.0
              }
            ],
            "max_displacement" => 1.0e-6,
            "max_stress" => 7.0e4,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/truss-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 0.0,
              "y" => 1.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n3",
              "x" => 0.2,
              "y" => 0.2,
              "z" => 1.0,
              "fix_x" => false,
              "fix_y" => false,
              "fix_z" => false,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => -1000.0
            }
          ],
          "elements" => [
            %{
              "id" => "e0",
              "node_i" => 0,
              "node_j" => 3,
              "area" => 0.01,
              "youngs_modulus" => 7.0e10
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["nodes"]) == 4
  end

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
  end

  test "registers, heartbeats, and removes remote agents through the API" do
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :registry, endpoints: [])
    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "solver-remote-a",
          "host" => "10.20.0.11",
          "port" => 6101,
          "region" => "ap-shanghai"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 201
    payload = Jason.decode!(conn.resp_body)
    assert payload["agent"]["id"] == "solver-remote-a"

    conn =
      :post
      |> conn(
        "/api/v1/agents/solver-remote-a/heartbeat",
        Jason.encode!(%{
          "host" => "10.20.0.11",
          "port" => 6101,
          "zone" => "rack-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["agent"]["zone"] == "rack-a"

    conn =
      :get
      |> conn("/api/v1/agents")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["summary"]["active_agents"] == 1
    assert Enum.map(payload["agents"], & &1["id"]) == ["solver-remote-a"]

    conn =
      :delete
      |> conn("/api/v1/agents/solver-remote-a")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["status"] == "removed"
  end

  test "surfaces solver failure messages through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => false,
          "error" => %{
            "code" => "solver_failed",
            "message" =>
              "truss response exceeds the small-deformation limit; check supports or connectivity"
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/truss-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            }
          ],
          "elements" => []
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "failed"
    assert result_payload["job"]["message"] =~ "small-deformation limit"
  end

  test "lists persisted jobs in reverse chronological order" do
    {:ok, _job_1} =
      Store.create(%{
        job_id: "job-old",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    {:ok, _job_1_completed} =
      Store.apply_progress(%{
        job_id: "job-old",
        stage: "completed",
        progress: 1.0,
        iteration: 5,
        residual: 1.0e-3
      })

    :ok =
      AnalysisResultStore.put("job-old", %{
        "kind" => "axial_bar_1d",
        "max_displacement" => 1.0e-6
      })

    Process.sleep(5)

    {:ok, _job_2} =
      Store.create(%{
        job_id: "job-new",
        project_id: "project-2",
        simulation_case_id: "case-2"
      })

    {:ok, _job_2_updated} =
      Store.apply_progress(%{
        job_id: "job-new",
        stage: "solving",
        progress: 0.5,
        iteration: 2,
        residual: 5.0e-1
      })

    conn =
      :get
      |> conn("/api/v1/jobs")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    jobs = payload["jobs"]

    assert Enum.map(jobs, & &1["job_id"]) == ["job-new", "job-old"]

    assert Enum.at(jobs, 0)["status"] == "solving"
    assert Enum.at(jobs, 0)["has_result"] == false
    assert Enum.at(jobs, 1)["status"] == "completed"
    assert Enum.at(jobs, 1)["has_result"] == true
    assert is_binary(Enum.at(jobs, 0)["updated_at"])
    assert is_binary(Enum.at(jobs, 1)["created_at"])
  end

  test "supports CRUD and export for persisted jobs and results" do
    {:ok, _job} =
      Store.create(%{
        job_id: "job-admin",
        project_id: "project-admin",
        simulation_case_id: "case-admin",
        message: "queued"
      })

    :ok =
      AnalysisResultStore.put("job-admin", %{
        "kind" => "truss_2d",
        "max_displacement" => 2.0e-6
      })

    {:ok, _event_payload} =
      SecurityEventStore.create(%{
        "event_id" => "event-admin",
        "event_type" => "security_high_risk_action",
        "source" => "assistant",
        "action" => "data/exportDatabase",
        "risk" => "sensitive",
        "status" => "completed",
        "note" => "database export finished",
        "context" => %{"study_kind" => "truss_2d"},
        "occurred_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
      })

    update_job_conn =
      :patch
      |> conn("/api/v1/jobs/job-admin", Jason.encode!(%{"message" => "reviewed"}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_job_conn.status == 200
    assert Jason.decode!(update_job_conn.resp_body)["job"]["message"] == "reviewed"

    list_results_conn =
      :get
      |> conn("/api/v1/results")
      |> Router.call(@opts)

    assert list_results_conn.status == 200
    assert [%{"job_id" => "job-admin"}] = Jason.decode!(list_results_conn.resp_body)["results"]

    update_result_conn =
      :patch
      |> conn(
        "/api/v1/results/job-admin",
        Jason.encode!(%{"result" => %{"kind" => "truss_2d", "max_displacement" => 4.0e-6}})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_result_conn.status == 200
    assert Jason.decode!(update_result_conn.resp_body)["result"]["max_displacement"] == 4.0e-6

    export_conn =
      :get
      |> conn("/api/v1/export/database")
      |> Router.call(@opts)

    assert export_conn.status == 200
    export_payload = Jason.decode!(export_conn.resp_body)
    assert [%{"job_id" => "job-admin"}] = export_payload["jobs"]
    assert [%{"job_id" => "job-admin"}] = export_payload["results"]
    assert [%{"event_id" => "event-admin"}] = export_payload["security_events"]
    assert is_list(export_payload["projects"])
    assert is_list(export_payload["models"])
    assert is_list(export_payload["model_versions"])

    security_export_conn =
      :get
      |> conn("/api/v1/export/security-events?status=completed")
      |> Router.call(@opts)

    assert security_export_conn.status == 200
    security_export_payload = Jason.decode!(security_export_conn.resp_body)
    assert security_export_payload["schema"]["name"] == "kyuubiki.security-events.export/v1"
    assert security_export_payload["filters"]["status"] == "completed"
    assert security_export_payload["summary"]["total"] == 1

    assert [%{"event_id" => "event-admin", "status" => "completed"}] =
             security_export_payload["events"]

    security_export_csv_conn =
      :get
      |> conn("/api/v1/export/security-events.csv?status=completed")
      |> Router.call(@opts)

    assert security_export_csv_conn.status == 200

    assert get_resp_header(security_export_csv_conn, "content-type") == [
             "text/csv; charset=utf-8"
           ]

    assert String.contains?(
             security_export_csv_conn.resp_body,
             "event_id,event_type,source,action"
           )

    assert String.contains?(security_export_csv_conn.resp_body, "event-admin")

    delete_result_conn =
      :delete
      |> conn("/api/v1/results/job-admin")
      |> Router.call(@opts)

    assert delete_result_conn.status == 200

    delete_job_conn =
      :delete
      |> conn("/api/v1/jobs/job-admin")
      |> Router.call(@opts)

    assert delete_job_conn.status == 200

    missing_job_conn =
      :get
      |> conn("/api/v1/jobs/job-admin")
      |> Router.call(@opts)

    assert missing_job_conn.status == 422
  end

  test "supports append-only security event ingestion and listing" do
    create_conn =
      :post
      |> conn(
        "/api/v1/security-events",
        Jason.encode!(%{
          "event_id" => "event-1",
          "event_type" => "security_high_risk_action",
          "source" => "script",
          "action" => "project/deleteSelected",
          "risk" => "destructive",
          "status" => "cancelled",
          "note" => "operator cancelled confirmation",
          "context" => %{"project_id" => "proj-1", "study_kind" => "truss_3d"},
          "occurred_at" => "2026-04-29T08:00:00Z"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_conn.status == 201
    assert Jason.decode!(create_conn.resp_body)["event"]["action"] == "project/deleteSelected"

    list_conn =
      :get
      |> conn("/api/v1/security-events")
      |> Router.call(@opts)

    assert list_conn.status == 200

    assert [
             %{
               "event_id" => "event-1",
               "source" => "script",
               "risk" => "destructive",
               "status" => "cancelled"
             }
           ] = Jason.decode!(list_conn.resp_body)["events"]

    {:ok, _second_event} =
      SecurityEventStore.create(%{
        "event_id" => "event-2",
        "event_type" => "security_high_risk_action",
        "source" => "assistant",
        "action" => "data/exportDatabase",
        "risk" => "sensitive",
        "status" => "completed",
        "note" => "assistant finished export",
        "context" => %{"study_kind" => "truss_2d", "project_id" => "proj-2"},
        "occurred_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
      })

    filtered_conn =
      :get
      |> conn("/api/v1/security-events?source=assistant&risk=sensitive&action=export")
      |> Router.call(@opts)

    assert filtered_conn.status == 200

    assert [%{"event_id" => "event-2", "source" => "assistant"}] =
             Jason.decode!(filtered_conn.resp_body)["events"]

    window_filtered_conn =
      :get
      |> conn("/api/v1/security-events?occurred_after=2026-04-29T09:00:00Z")
      |> Router.call(@opts)

    assert window_filtered_conn.status == 200
    assert [%{"event_id" => "event-2"}] = Jason.decode!(window_filtered_conn.resp_body)["events"]
  end

  test "serves a Hub-facing workload catalog with project download URLs" do
    previous_trust = Application.get_env(:kyuubiki_web, :trust_forwarded_headers, false)
    Application.put_env(:kyuubiki_web, :trust_forwarded_headers, true)
    on_exit(fn -> Application.put_env(:kyuubiki_web, :trust_forwarded_headers, previous_trust) end)

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

  defp await_fake_agent_port do
    receive do
      {:fake_agent_ready, port} -> port
    after
      1_000 -> flunk("timed out waiting for fake agent port")
    end
  end

  defp wait_for_job(job_id, attempts \\ 20)

  defp wait_for_job(job_id, attempts) when attempts > 0 do
    conn =
      :get
      |> conn("/api/v1/jobs/#{job_id}")
      |> Router.call(@opts)

    payload = Jason.decode!(conn.resp_body)

    if payload["job"]["status"] in ["completed", "failed"] do
      payload
    else
      Process.sleep(10)
      wait_for_job(job_id, attempts - 1)
    end
  end

  defp wait_for_job(_job_id, 0), do: flunk("timed out waiting for async job completion")
end
