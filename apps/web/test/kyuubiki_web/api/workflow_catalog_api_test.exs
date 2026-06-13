defmodule KyuubikiWeb.Api.WorkflowCatalogApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "lists the built-in workflow catalog and fetches a descriptor" do
    list_conn =
      :get
      |> conn("/api/v1/workflows/catalog")
      |> Router.call(@opts)

    assert list_conn.status == 200
    payload = Jason.decode!(list_conn.resp_body)
    workflows = payload["workflows"]
    assert length(workflows) >= 3

    workflow_ids =
      MapSet.new(Enum.map(workflows, fn workflow -> workflow["id"] end))

    assert MapSet.subset?(
             MapSet.new([
               "workflow.electrostatic-to-heat-quad-2d",
               "workflow.electrostatic-plane-quad-2d",
               "workflow.heat-to-thermo-quad-2d"
             ]),
             workflow_ids
           )

    electrostatic_heat_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-to-heat-quad-2d"
      end)

    electrostatic_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-plane-quad-2d"
      end)

    workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.heat-to-thermo-quad-2d"
      end)

    assert electrostatic_heat_workflow["name"] == "Electrostatic to heat quad"
    assert electrostatic_heat_workflow["version"] == "1.0.0"
    assert electrostatic_workflow["name"] == "Electrostatic plane quad"
    assert electrostatic_workflow["version"] == "1.0.0"
    assert workflow["name"] == "Heat to thermo quad"
    assert workflow["version"] == "1.0.0"
    assert length(workflow["entry_inputs"]) == 1
    assert length(workflow["output_artifacts"]) == 1
    assert is_map(workflow["graph"])

    fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.heat-to-thermo-quad-2d")
      |> Router.call(@opts)

    assert fetch_conn.status == 200
    fetched = Jason.decode!(fetch_conn.resp_body)["workflow"]
    assert fetched["id"] == "workflow.heat-to-thermo-quad-2d"
    assert fetched["graph"]["output_nodes"] == ["json_output"]

    electrostatic_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-plane-quad-2d")
      |> Router.call(@opts)

    assert electrostatic_fetch_conn.status == 200
    electrostatic_fetched = Jason.decode!(electrostatic_fetch_conn.resp_body)["workflow"]
    assert electrostatic_fetched["id"] == "workflow.electrostatic-plane-quad-2d"
    assert electrostatic_fetched["graph"]["output_nodes"] == ["json_output"]

    electrostatic_heat_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-to-heat-quad-2d")
      |> Router.call(@opts)

    assert electrostatic_heat_fetch_conn.status == 200

    electrostatic_heat_fetched =
      Jason.decode!(electrostatic_heat_fetch_conn.resp_body)["workflow"]

    assert electrostatic_heat_fetched["id"] == "workflow.electrostatic-to-heat-quad-2d"
    assert electrostatic_heat_fetched["graph"]["output_nodes"] == ["json_output"]
  end

  test "filters workflow catalog by query contract fields" do
    conn =
      :get
      |> conn(
        "/api/v1/workflows/catalog?q=thermo&domain=thermo_mechanical&capability=workflow_bridge&entry_artifact=study_model/heat_plane_quad_2d&operator_id=bridge.temperature_field_to_thermo_quad_2d"
      )
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    workflows = payload["workflows"]
    assert length(workflows) == 1
    assert hd(workflows)["id"] == "workflow.heat-to-thermo-quad-2d"
  end

  test "submits a catalog workflow as an asynchronous job" do
    {:ok, _pid} =
      WorkflowApi.start_fake_agent_sessions([
        [
          %{
            "ok" => true,
            "result" => %{
              "max_temperature" => 100.0,
              "max_heat_flux" => 2846.0498941515416,
              "nodes" => [
                %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 100.0},
                %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 60.0},
                %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 20.0},
                %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0}
              ],
              "elements" => [
                %{
                  "id" => "hq0",
                  "temperature_gradient_x" => -20.0,
                  "temperature_gradient_y" => -60.0
                }
              ],
              "input" => %{
                "elements" => [
                  %{
                    "id" => "hq0",
                    "node_i" => 0,
                    "node_j" => 1,
                    "node_k" => 2,
                    "node_l" => 3,
                    "thickness" => 0.02,
                    "conductivity" => 45.0
                  }
                ]
              }
            }
          }
        ],
        [
          %{
            "ok" => true,
            "result" => %{
              "max_displacement" => 0.0,
              "max_stress" => 34_477_611.940298505,
              "max_temperature_delta" => 30,
              "nodes" => [
                %{"id" => "h0", "temperature_delta" => 100.0},
                %{"id" => "h1", "temperature_delta" => 60.0},
                %{"id" => "h2", "temperature_delta" => 20.0},
                %{"id" => "h3", "temperature_delta" => 20.0}
              ],
              "elements" => [
                %{
                  "id" => "tq0",
                  "stress_x" => -34_477_611.940298505,
                  "stress_y" => -34_477_611.940298505,
                  "mechanical_strain_x" => -3.3e-4,
                  "mechanical_strain_y" => -3.3e-4
                }
              ]
            }
          }
        ]
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/workflows/catalog/workflow.heat-to-thermo-quad-2d/jobs",
        Jason.encode!(%{"input_artifacts" => WorkflowApi.heat_to_thermo_quad_input_artifacts()})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    job_id = payload["job"]["job_id"]
    assert payload["job"]["status"] == "queued"

    result_payload = WorkflowApi.wait_for_job(job_id, @opts)
    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.heat-to-thermo-quad-2d"
    assert length(result_payload["result"]["completed_nodes"]) == 7
    assert length(result_payload["result"]["progress_events"]) == 7
    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"
    summary = Jason.decode!(exported["content"])
    assert summary["max_temperature_delta"] == 30
    assert_in_delta summary["max_stress"], 34_477_611.940298505, 1.0e-6
  end

  test "submits an electrostatic quad catalog workflow as an asynchronous job" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving electrostatic plane quad"
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
                "potential" => 10.0,
                "charge_density" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "epq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "average_potential" => 5.0,
                "potential_gradient_x" => -10.0,
                "potential_gradient_y" => 0.0,
                "electric_field_x" => 10.0,
                "electric_field_y" => 0.0,
                "electric_field_magnitude" => 10.0,
                "electric_flux_density_x" => 20.0,
                "electric_flux_density_y" => 0.0,
                "electric_flux_density_magnitude" => 20.0
              }
            ],
            "max_potential" => 10.0,
            "max_electric_field" => 10.0,
            "max_flux_density" => 20.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/workflows/catalog/workflow.electrostatic-plane-quad-2d/jobs",
        Jason.encode!(%{"input_artifacts" => WorkflowApi.electrostatic_plane_quad_input_artifacts()})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.electrostatic-plane-quad-2d"
    assert length(result_payload["result"]["completed_nodes"]) == 5
    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"
    summary = Jason.decode!(exported["content"])
    assert summary["max_potential"] == 10.0
    assert summary["max_electric_field"] == 10.0
    assert summary["max_flux_density"] == 20.0
  end

  test "submits an electrostatic to heat catalog workflow as an asynchronous job" do
    {:ok, _pid} =
      WorkflowApi.start_fake_agent_sessions([
        [
          %{
            "ok" => true,
            "result" => %{
              "nodes" => [
                %{
                  "index" => 0,
                  "id" => "n0",
                  "x" => 0.0,
                  "y" => 0.0,
                  "potential" => 10.0,
                  "charge_density" => 0.0
                },
                %{
                  "index" => 1,
                  "id" => "n1",
                  "x" => 1.0,
                  "y" => 0.0,
                  "potential" => 0.0,
                  "charge_density" => 0.0
                },
                %{
                  "index" => 2,
                  "id" => "n2",
                  "x" => 1.0,
                  "y" => 1.0,
                  "potential" => 0.0,
                  "charge_density" => 0.0
                },
                %{
                  "index" => 3,
                  "id" => "n3",
                  "x" => 0.0,
                  "y" => 1.0,
                  "potential" => 10.0,
                  "charge_density" => 0.0
                }
              ],
              "elements" => [
                %{
                  "index" => 0,
                  "id" => "epq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "area" => 1.0,
                  "average_potential" => 5.0,
                  "potential_gradient_x" => -10.0,
                  "potential_gradient_y" => 0.0,
                  "electric_field_x" => 10.0,
                  "electric_field_y" => 0.0,
                  "electric_field_magnitude" => 10.0,
                  "electric_flux_density_x" => 20.0,
                  "electric_flux_density_y" => 0.0,
                  "electric_flux_density_magnitude" => 20.0
                }
              ],
              "max_potential" => 10.0,
              "max_electric_field" => 10.0,
              "max_flux_density" => 20.0,
              "input" => %{"nodes" => [], "elements" => []}
            }
          }
        ],
        [
          %{
            "ok" => true,
            "result" => %{
              "max_temperature" => 70.0,
              "max_heat_flux" => 1500.0,
              "nodes" => [
                %{
                  "id" => "n0",
                  "x" => 0.0,
                  "y" => 0.0,
                  "temperature" => 20.0,
                  "heat_load" => 500.0
                },
                %{
                  "id" => "n1",
                  "x" => 1.0,
                  "y" => 0.0,
                  "temperature" => 70.0,
                  "heat_load" => 500.0
                },
                %{
                  "id" => "n2",
                  "x" => 1.0,
                  "y" => 1.0,
                  "temperature" => 70.0,
                  "heat_load" => 500.0
                },
                %{
                  "id" => "n3",
                  "x" => 0.0,
                  "y" => 1.0,
                  "temperature" => 20.0,
                  "heat_load" => 500.0
                }
              ],
              "elements" => [
                %{
                  "id" => "hq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "temperature_gradient_x" => 50.0,
                  "temperature_gradient_y" => 0.0,
                  "heat_flux_x" => -1500.0,
                  "heat_flux_y" => 0.0
                }
              ],
              "input" => %{"nodes" => [], "elements" => []}
            }
          }
        ]
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/workflows/catalog/workflow.electrostatic-to-heat-quad-2d/jobs",
        Jason.encode!(%{"input_artifacts" => WorkflowApi.electrostatic_plane_quad_input_artifacts()})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.electrostatic-to-heat-quad-2d"
    assert length(result_payload["result"]["completed_nodes"]) == 7
    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"
    summary = Jason.decode!(exported["content"])
    assert summary["max_temperature"] == 70.0
    assert summary["max_heat_flux"] == 1500.0

    bridged_model = result_payload["result"]["artifacts"]["bridge_field_to_heat.heat_model"]
    bridged_nodes = bridged_model["nodes"]
    assert Enum.all?(bridged_nodes, fn node -> node["heat_load"] == 500.0 end)
  end
end
