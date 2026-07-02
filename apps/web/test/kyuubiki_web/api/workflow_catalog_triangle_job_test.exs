defmodule KyuubikiWeb.WorkflowCatalogTriangleJobTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Playground.AgentRegistry
  alias KyuubikiWeb.Router
  alias KyuubikiWeb.SecurityEvents.Store, as: SecurityEventStore
  alias KyuubikiWeb.TestSupport.WorkflowApi

  @opts Router.init([])

  setup do
    Store.reset()
    AnalysisResultStore.reset()
    Library.reset()
    SecurityEventStore.reset()
    Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)

    original_config = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)
      Application.put_env(:kyuubiki_web, AgentPool, original_config)
      AgentPool.reload()
    end)

    :ok
  end

  test "submits an electrostatic to heat to thermo triangle catalog workflow as an asynchronous job" do
    {:ok, _pid} =
      WorkflowApi.start_fake_agent_sessions([
        [
          %{
            "ok" => true,
            "result" => %{
              "nodes" => [
                %{"index" => 0, "id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 12.0},
                %{"index" => 1, "id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 0.0},
                %{"index" => 2, "id" => "e2", "x" => 0.0, "y" => 1.0, "potential" => 6.0}
              ],
              "elements" => [
                %{
                  "index" => 0,
                  "id" => "et0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "area" => 0.5,
                  "electric_field_x" => 6.0,
                  "electric_field_y" => 8.0,
                  "electric_field_magnitude" => 10.0,
                  "electric_flux_density_x" => 12.0,
                  "electric_flux_density_y" => 16.0,
                  "electric_flux_density_magnitude" => 20.0
                }
              ],
              "max_potential" => 12.0,
              "max_electric_field" => 10.0,
              "max_flux_density" => 20.0,
              "input" => %{
                "elements" => [
                  %{
                    "id" => "et0",
                    "node_i" => 0,
                    "node_j" => 1,
                    "node_k" => 2,
                    "thickness" => 0.05,
                    "permittivity" => 2.0
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
              "max_temperature" => 75.0,
              "max_heat_flux" => 900.0,
              "nodes" => [
                %{
                  "id" => "t0",
                  "x" => 0.0,
                  "y" => 0.0,
                  "temperature" => 75.0,
                  "heat_load" => 500.0
                },
                %{
                  "id" => "t1",
                  "x" => 1.0,
                  "y" => 0.0,
                  "temperature" => 55.0,
                  "heat_load" => 500.0
                },
                %{
                  "id" => "t2",
                  "x" => 0.0,
                  "y" => 1.0,
                  "temperature" => 35.0,
                  "heat_load" => 500.0
                }
              ],
              "elements" => [
                %{
                  "id" => "ht0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "temperature_gradient_x" => 20.0,
                  "temperature_gradient_y" => -40.0,
                  "heat_flux_x" => -450.0,
                  "heat_flux_y" => 900.0
                }
              ],
              "input" => %{
                "elements" => [
                  %{
                    "id" => "ht0",
                    "node_i" => 0,
                    "node_j" => 1,
                    "node_k" => 2,
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
              "max_displacement" => 0.0025,
              "max_stress" => 22_500_000.0,
              "max_temperature_delta" => 75.0,
              "nodes" => [
                %{"id" => "t0", "temperature_delta" => 75.0},
                %{"id" => "t1", "temperature_delta" => 55.0},
                %{"id" => "t2", "temperature_delta" => 35.0}
              ],
              "elements" => [
                %{
                  "id" => "tt0",
                  "stress_x" => -22_500_000.0,
                  "stress_y" => -18_000_000.0,
                  "mechanical_strain_x" => -2.2e-4,
                  "mechanical_strain_y" => -1.8e-4
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
        "/api/v1/workflows/catalog/workflow.electrostatic-heat-thermo-triangle-summary-json/jobs",
        Jason.encode!(%{
          "input_artifacts" => WorkflowApi.electrostatic_plane_triangle_input_artifacts()
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"

    assert result_payload["result"]["workflow_id"] ==
             "workflow.electrostatic-heat-thermo-triangle-summary-json"

    assert length(result_payload["result"]["completed_nodes"]) == 9
    assert length(result_payload["result"]["progress_events"]) == 9

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"

    summary = Jason.decode!(exported["content"])
    assert summary["max_displacement"] == 0.0025
    assert summary["max_stress"] == 22_500_000.0
    assert summary["max_temperature_delta"] == 75.0

    bridged_heat_model = result_payload["result"]["artifacts"]["bridge_field_to_heat.heat_model"]
    assert Enum.all?(bridged_heat_model["nodes"], fn node -> node["heat_load"] == 500.0 end)

    bridged_thermo_model =
      result_payload["result"]["artifacts"]["bridge_temperature.thermo_model"]

    assert Enum.map(bridged_thermo_model["nodes"], & &1["temperature_delta"]) == [
             75.0,
             55.0,
             35.0
           ]
  end
end
