defmodule KyuubikiWeb.Api.WorkflowRuntimeApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs a workflow graph through the orchestration API" do
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
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => %{
            "schema_version" => "kyuubiki.workflow-graph/v1",
            "id" => "workflow.heat-to-thermo-quad-2d",
            "name" => "Heat to thermo quad",
            "version" => "1.0.0",
            "entry_nodes" => ["heat_model"],
            "output_nodes" => ["json_output"],
            "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
            "nodes" => [
              %{
                "id" => "heat_model",
                "kind" => "input",
                "outputs" => [
                  %{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}
                ]
              },
              %{
                "id" => "solve_heat",
                "kind" => "solve",
                "operator_id" => "solve.heat_plane_quad_2d",
                "inputs" => [
                  %{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}
                ],
                "outputs" => [%{"id" => "result", "artifact_type" => "result/heat_plane_quad_2d"}]
              },
              %{
                "id" => "bridge_temperature",
                "kind" => "transform",
                "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
                "config" => %{
                  "nodes" => [
                    %{
                      "id" => "h0",
                      "x" => 0.0,
                      "y" => 0.0,
                      "fix_x" => true,
                      "fix_y" => true,
                      "temperature_delta" => 0.0
                    },
                    %{
                      "id" => "h1",
                      "x" => 1.0,
                      "y" => 0.0,
                      "fix_x" => false,
                      "fix_y" => false,
                      "temperature_delta" => 0.0
                    },
                    %{
                      "id" => "h2",
                      "x" => 1.0,
                      "y" => 1.0,
                      "fix_x" => false,
                      "fix_y" => false,
                      "temperature_delta" => 0.0
                    },
                    %{
                      "id" => "h3",
                      "x" => 0.0,
                      "y" => 1.0,
                      "fix_x" => true,
                      "fix_y" => true,
                      "temperature_delta" => 0.0
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
                      "youngs_modulus" => 210.0e9,
                      "poisson_ratio" => 0.3,
                      "thermal_expansion" => 11.0e-6
                    }
                  ]
                },
                "inputs" => [
                  %{"id" => "heat_result", "artifact_type" => "result/heat_plane_quad_2d"}
                ],
                "outputs" => [
                  %{
                    "id" => "thermo_model",
                    "artifact_type" => "study_model/thermal_plane_quad_2d"
                  }
                ]
              },
              %{
                "id" => "solve_thermo",
                "kind" => "solve",
                "operator_id" => "solve.thermal_plane_quad_2d",
                "inputs" => [
                  %{"id" => "model", "artifact_type" => "study_model/thermal_plane_quad_2d"}
                ],
                "outputs" => [
                  %{"id" => "result", "artifact_type" => "result/thermal_plane_quad_2d"}
                ]
              },
              %{
                "id" => "extract_summary",
                "kind" => "extract",
                "operator_id" => "extract.result_summary",
                "inputs" => [
                  %{"id" => "result", "artifact_type" => "result/thermal_plane_quad_2d"}
                ],
                "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
              },
              %{
                "id" => "export_json",
                "kind" => "export",
                "operator_id" => "export.summary_json",
                "inputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}],
                "outputs" => [%{"id" => "json", "artifact_type" => "export/json"}]
              },
              %{
                "id" => "json_output",
                "kind" => "output",
                "inputs" => [%{"id" => "json", "artifact_type" => "export/json"}],
                "outputs" => []
              }
            ],
            "edges" => [
              %{
                "id" => "e0",
                "from" => %{"node" => "heat_model", "port" => "model"},
                "to" => %{"node" => "solve_heat", "port" => "model"},
                "artifact_type" => "study_model/heat_plane_quad_2d"
              },
              %{
                "id" => "e1",
                "from" => %{"node" => "solve_heat", "port" => "result"},
                "to" => %{"node" => "bridge_temperature", "port" => "heat_result"},
                "artifact_type" => "result/heat_plane_quad_2d"
              },
              %{
                "id" => "e2",
                "from" => %{"node" => "bridge_temperature", "port" => "thermo_model"},
                "to" => %{"node" => "solve_thermo", "port" => "model"},
                "artifact_type" => "study_model/thermal_plane_quad_2d"
              },
              %{
                "id" => "e3",
                "from" => %{"node" => "solve_thermo", "port" => "result"},
                "to" => %{"node" => "extract_summary", "port" => "result"},
                "artifact_type" => "result/thermal_plane_quad_2d"
              },
              %{
                "id" => "e4",
                "from" => %{"node" => "extract_summary", "port" => "summary"},
                "to" => %{"node" => "export_json", "port" => "summary"},
                "artifact_type" => "report/summary"
              },
              %{
                "id" => "e5",
                "from" => %{"node" => "export_json", "port" => "json"},
                "to" => %{"node" => "json_output", "port" => "json"},
                "artifact_type" => "export/json"
              }
            ]
          },
          "input_artifacts" => WorkflowApi.heat_to_thermo_quad_input_artifacts()
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["workflow_id"] == "workflow.heat-to-thermo-quad-2d"
    assert length(payload["completed_nodes"]) == 7
    exported = payload["artifacts"]["json_output.json"]
    assert exported["format"] == "json"
    summary = Jason.decode!(exported["content"])
    assert summary["max_temperature_delta"] == 30
    assert_in_delta summary["max_stress"], 34_477_611.940298505, 1.0e-6
  end
end
