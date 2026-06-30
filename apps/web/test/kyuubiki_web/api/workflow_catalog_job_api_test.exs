defmodule KyuubikiWeb.Api.WorkflowCatalogJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

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

    assert result_payload["result"]["dataset_contract"]["id"] ==
             "kyuubiki.dataset.heat_to_thermo_quad/v1"

    completed_nodes = result_payload["result"]["completed_nodes"]
    assert length(completed_nodes) == 8
    assert "validate_bridge_temperature" in completed_nodes
    assert length(result_payload["result"]["progress_events"]) == 8

    assert Enum.any?(result_payload["result"]["dataset_lineage"], fn entry ->
             entry["dataset_value"] == "thermo_summary" and
               entry["source_datasets"] == ["thermo_result"]
           end)

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"
    summary = Jason.decode!(exported["content"])
    assert summary["max_temperature_delta"] == 30
    assert_in_delta summary["max_stress"], 34_477_611.940298505, 1.0e-6
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
        Jason.encode!(%{
          "input_artifacts" => WorkflowApi.electrostatic_plane_quad_input_artifacts()
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    result_payload =
      WorkflowApi.wait_for_job(Jason.decode!(conn.resp_body)["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.electrostatic-to-heat-quad-2d"

    assert result_payload["result"]["dataset_contract"]["id"] ==
             "kyuubiki.dataset.electrostatic_to_heat_quad/v1"

    completed_nodes = result_payload["result"]["completed_nodes"]
    assert length(completed_nodes) == 8
    assert "validate_bridge_field_to_heat" in completed_nodes

    assert Enum.any?(result_payload["result"]["dataset_lineage"], fn entry ->
             entry["dataset_value"] == "heat_model" and
               entry["source_datasets"] == ["electrostatic_result"]
           end)

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
