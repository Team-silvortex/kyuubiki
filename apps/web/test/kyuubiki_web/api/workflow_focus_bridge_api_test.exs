defmodule KyuubikiWeb.Api.WorkflowFocusBridgeApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "runs a focus-driven heat to thermo bridge workflow graph" do
    {:ok, graph} =
      WorkflowTemplateCatalog.graph_by_id("workflow.focus-heat-to-thermo-bridge-json")

    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => graph,
          "input_artifacts" => %{
            "report_input" => %{
              "report_contract" => "kyuubiki.workflow_report_payload/v1",
              "report_focus_payloads" => %{
                "thermo.stress_peak" => %{
                  "focus_contract" => "kyuubiki.workflow_focus_payload/v1",
                  "metric_id" => "thermo.stress_peak",
                  "source" => "thermo",
                  "value" => 220.0,
                  "value_field" => "thermo_peak_stress",
                  "context" => %{"peak_element_id" => "tq0"}
                }
              }
            },
            "heat_input" => %{
              "input" => %{
                "nodes" => [
                  %{
                    "id" => "h0",
                    "x" => 0.0,
                    "y" => 0.0,
                    "fix_temperature" => true,
                    "temperature" => 100.0,
                    "heat_load" => 6.0
                  },
                  %{
                    "id" => "h1",
                    "x" => 1.0,
                    "y" => 0.0,
                    "fix_temperature" => true,
                    "temperature" => 80.0,
                    "heat_load" => 12.0
                  },
                  %{
                    "id" => "h2",
                    "x" => 1.0,
                    "y" => 1.0,
                    "fix_temperature" => true,
                    "temperature" => 60.0,
                    "heat_load" => 18.0
                  },
                  %{
                    "id" => "h3",
                    "x" => 0.0,
                    "y" => 1.0,
                    "fix_temperature" => true,
                    "temperature" => 40.0,
                    "heat_load" => 24.0
                  }
                ],
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
              },
              "nodes" => [
                %{
                  "index" => 0,
                  "id" => "h0",
                  "x" => 0.0,
                  "y" => 0.0,
                  "temperature" => 100.0,
                  "heat_load" => 6.0
                },
                %{
                  "index" => 1,
                  "id" => "h1",
                  "x" => 1.0,
                  "y" => 0.0,
                  "temperature" => 80.0,
                  "heat_load" => 12.0
                },
                %{
                  "index" => 2,
                  "id" => "h2",
                  "x" => 1.0,
                  "y" => 1.0,
                  "temperature" => 60.0,
                  "heat_load" => 18.0
                },
                %{
                  "index" => 3,
                  "id" => "h3",
                  "x" => 0.0,
                  "y" => 1.0,
                  "temperature" => 40.0,
                  "heat_load" => 24.0
                }
              ],
              "elements" => [
                %{
                  "index" => 0,
                  "id" => "hq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "area" => 1.0,
                  "average_temperature" => 70.0,
                  "temperature_gradient_x" => -15.0,
                  "temperature_gradient_y" => -5.0,
                  "heat_flux_x" => 30.0,
                  "heat_flux_y" => 10.0,
                  "heat_flux_magnitude" => 31.6227766017
                }
              ],
              "max_temperature" => 100.0,
              "max_heat_flux" => 31.6227766017
            }
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["completed_nodes"] == [
             "report_input",
             "heat_input",
             "select_focus",
             "compose_chain",
             "compose_request",
             "resolve_execution",
             "execute_bridge",
             "bridge_output"
           ]

    bridge_result = get_in(payload, ["artifacts", "bridge_output.result"])
    assert bridge_result["result_contract"] == "kyuubiki.workflow_focus_bridge_result/v1"
    assert bridge_result["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d"
    assert bridge_result["metric_id"] == "thermo.stress_peak"
    assert bridge_result["bridge_result"]["nodes"] |> is_list()
    assert bridge_result["bridge_result"]["elements"] |> is_list()
  end
end
