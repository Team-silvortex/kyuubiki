defmodule KyuubikiWeb.Api.RouterAccessAndBarTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

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

  test "allows loopback read routes when read protection is disabled" do
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

  test "rejects non-loopback read routes without a configured API token even when read protection is disabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :get
      |> conn("/api/v1/export/database")
      |> Map.put(:remote_ip, {203, 0, 113, 9})
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
  end

  test "allows loopback mutating routes without a configured API token" do
    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Local Project", "description" => "loopback allowed"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["project"]["name"] == "Local Project"
  end

  test "exposes active execution leases through the agents API" do
    assert {:ok, _agent} =
             AgentRegistry.register(%{
               "id" => "solver-agent-lease-view",
               "host" => "10.20.2.10",
               "port" => 6401,
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a"
             })

    assert {:ok, _lease} =
             AgentRegistry.claim_execution("solver-agent-lease-view", %{
               "lease_id" => "lease-view-a",
               "control_mode" => "orch_managed",
               "orch_id" => "orch-alpha",
               "orch_session_id" => "session-a",
               "job_id" => "job-a",
               "method" => "solve_bar_1d"
             })

    conn =
      :get
      |> conn("/api/v1/agents")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    [agent] = payload["agents"]

    assert agent["id"] == "solver-agent-lease-view"
    assert agent["execution_state"] == "leased"
    assert agent["active_lease"]["lease_id"] == "lease-view-a"
    assert agent["active_lease"]["job_id"] == "job-a"
    assert is_integer(agent["active_lease"]["age_ms"])
    assert agent["active_lease"]["is_stale"] == false

    assert payload["summary"]["active_execution_lease_count"] == 1
    assert payload["summary"]["stale_execution_lease_count"] == 0

    assert hd(payload["summary"]["active_execution_leases"])["agent_id"] ==
             "solver-agent-lease-view"
  end

  test "rejects non-loopback mutating routes without a configured API token" do
    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Remote Project", "description" => "should fail"})
      )
      |> put_req_header("content-type", "application/json")
      |> Map.put(:remote_ip, {203, 0, 113, 9})
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
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

    result_payload = WorkflowApi.wait_for_job(job_id, @opts)

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
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

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
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "temperature" => 100.0,
                "heat_load" => 0.0
              },
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
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_temperature"] == 100.0
    assert result_payload["result"]["max_heat_flux"] > 0
    assert length(result_payload["result"]["elements"]) == 1
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
end
