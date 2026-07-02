defmodule KyuubikiWeb.WorkflowTemplateThermalContractGraphs do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def electrostatic_heat_thermo_triangle_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-heat-thermo-triangle-summary-json",
      "name" => "Electrostatic heat thermo triangle summary JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.electrostatic_heat_thermo_triangle/v1",
          [
            dataset_value(
              "electrostatic_model",
              "model",
              "study_model/electrostatic_plane_triangle_2d"
            ),
            dataset_value(
              "electrostatic_result",
              "result",
              "result/electrostatic_plane_triangle_2d"
            ),
            dataset_value("heat_model", "model", "study_model/heat_plane_triangle_2d"),
            dataset_value("heat_result", "result", "result/heat_plane_triangle_2d"),
            dataset_value("thermo_model", "model", "study_model/thermal_plane_triangle_2d"),
            dataset_value("thermo_result", "result", "result/thermal_plane_triangle_2d"),
            dataset_value("thermo_summary", "result", "report/summary"),
            dataset_value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "electrostatic_heat_thermo_triangle"}
        ),
      "entry_nodes" => ["electrostatic_plane_triangle_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        %{
          "id" => "electrostatic_plane_triangle_model",
          "kind" => "input",
          "outputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
              "dataset_value" => "electrostatic_model"
            }
          ]
        },
        %{
          "id" => "solve_electrostatic",
          "kind" => "solve",
          "operator_id" => "solve.electrostatic_plane_triangle_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
              "dataset_value" => "electrostatic_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/electrostatic_plane_triangle_2d",
              "dataset_value" => "electrostatic_result"
            }
          ]
        },
        %{
          "id" => "bridge_field_to_heat",
          "kind" => "transform",
          "operator_id" => "bridge.electrostatic_field_to_heat_triangle_2d",
          "config" => %{
            "seed_model" => heat_triangle_seed_model_example(),
            "contract" =>
              WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(
                50.0,
                ["node_i", "node_j", "node_k"]
              )
          },
          "inputs" => [
            %{
              "id" => "electrostatic_result",
              "artifact_type" => "result/electrostatic_plane_triangle_2d",
              "dataset_value" => "electrostatic_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "heat_model",
              "artifact_type" => "study_model/heat_plane_triangle_2d",
              "dataset_value" => "heat_model"
            }
          ]
        },
        %{
          "id" => "solve_heat",
          "kind" => "solve",
          "operator_id" => "solve.heat_plane_triangle_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/heat_plane_triangle_2d",
              "dataset_value" => "heat_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/heat_plane_triangle_2d",
              "dataset_value" => "heat_result"
            }
          ]
        },
        %{
          "id" => "bridge_temperature",
          "kind" => "transform",
          "operator_id" => "bridge.temperature_field_to_thermo_triangle_2d",
          "config" => %{
            "seed_model" => WorkflowCatalogSupport.thermo_triangle_seed_model_example(),
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          },
          "inputs" => [
            %{
              "id" => "heat_result",
              "artifact_type" => "result/heat_plane_triangle_2d",
              "dataset_value" => "heat_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "thermo_model",
              "artifact_type" => "study_model/thermal_plane_triangle_2d",
              "dataset_value" => "thermo_model"
            }
          ]
        },
        %{
          "id" => "solve_thermo",
          "kind" => "solve",
          "operator_id" => "solve.thermal_plane_triangle_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/thermal_plane_triangle_2d",
              "dataset_value" => "thermo_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/thermal_plane_triangle_2d",
              "dataset_value" => "thermo_result"
            }
          ]
        },
        %{
          "id" => "extract_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
          "config" => %{"fields" => ["max_displacement", "max_stress", "max_temperature_delta"]},
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/thermal_plane_triangle_2d",
              "dataset_value" => "thermo_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "thermo_summary"
            }
          ]
        },
        %{
          "id" => "export_json",
          "kind" => "export",
          "operator_id" => "export.summary_json",
          "inputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "thermo_summary"
            }
          ],
          "outputs" => [
            %{"id" => "json", "artifact_type" => "export/json", "dataset_value" => "summary_json"}
          ]
        },
        %{
          "id" => "json_output",
          "kind" => "output",
          "inputs" => [
            %{"id" => "json", "artifact_type" => "export/json", "dataset_value" => "summary_json"}
          ],
          "outputs" => []
        }
      ],
      "edges" => [
        edge(
          "e0",
          "electrostatic_plane_triangle_model",
          "model",
          "solve_electrostatic",
          "model",
          "study_model/electrostatic_plane_triangle_2d",
          "electrostatic_model"
        ),
        edge(
          "e1",
          "solve_electrostatic",
          "result",
          "bridge_field_to_heat",
          "electrostatic_result",
          "result/electrostatic_plane_triangle_2d",
          "electrostatic_result"
        ),
        edge(
          "e2",
          "bridge_field_to_heat",
          "heat_model",
          "solve_heat",
          "model",
          "study_model/heat_plane_triangle_2d",
          "heat_model"
        ),
        edge(
          "e3",
          "solve_heat",
          "result",
          "bridge_temperature",
          "heat_result",
          "result/heat_plane_triangle_2d",
          "heat_result"
        ),
        edge(
          "e4",
          "bridge_temperature",
          "thermo_model",
          "solve_thermo",
          "model",
          "study_model/thermal_plane_triangle_2d",
          "thermo_model"
        ),
        edge(
          "e5",
          "solve_thermo",
          "result",
          "extract_summary",
          "result",
          "result/thermal_plane_triangle_2d",
          "thermo_result"
        ),
        edge(
          "e6",
          "extract_summary",
          "summary",
          "export_json",
          "summary",
          "report/summary",
          "thermo_summary"
        ),
        edge("e7", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  def heat_to_thermo_quad_comparison_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.heat-to-thermo-quad-comparison-json",
      "name" => "Heat to thermo quad comparison JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.heat_to_thermo_quad_comparison/v1",
          [
            dataset_value("heat_model", "model", "study_model/heat_plane_quad_2d"),
            dataset_value("heat_result", "result", "result/heat_plane_quad_2d"),
            dataset_value("heat_summary", "result", "report/summary"),
            dataset_value("thermo_model", "model", "study_model/thermal_plane_quad_2d"),
            dataset_value("thermo_result", "result", "result/thermal_plane_quad_2d"),
            dataset_value("thermo_summary", "result", "report/summary"),
            dataset_value("comparison_summary", "result", "report/summary"),
            dataset_value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "heat_to_thermo_quad_comparison"}
        ),
      "entry_nodes" => ["heat_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        %{
          "id" => "heat_model",
          "kind" => "input",
          "outputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/heat_plane_quad_2d",
              "dataset_value" => "heat_model"
            }
          ]
        },
        %{
          "id" => "solve_heat",
          "kind" => "solve",
          "operator_id" => "solve.heat_plane_quad_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/heat_plane_quad_2d",
              "dataset_value" => "heat_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            }
          ]
        },
        %{
          "id" => "extract_heat_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
          "config" => %{"fields" => ["max_temperature", "max_heat_flux"]},
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "heat_summary"
            }
          ]
        },
        %{
          "id" => "bridge_temperature",
          "kind" => "transform",
          "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
          "config" => %{
            "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          },
          "inputs" => [
            %{
              "id" => "heat_result",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "thermo_model",
              "artifact_type" => "study_model/thermal_plane_quad_2d",
              "dataset_value" => "thermo_model"
            }
          ]
        },
        %{
          "id" => "solve_thermo",
          "kind" => "solve",
          "operator_id" => "solve.thermal_plane_quad_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/thermal_plane_quad_2d",
              "dataset_value" => "thermo_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/thermal_plane_quad_2d",
              "dataset_value" => "thermo_result"
            }
          ]
        },
        %{
          "id" => "extract_thermo_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
          "config" => %{"fields" => ["max_displacement", "max_stress", "max_temperature_delta"]},
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/thermal_plane_quad_2d",
              "dataset_value" => "thermo_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "thermo_summary"
            }
          ]
        },
        %{
          "id" => "compare_summaries",
          "kind" => "transform",
          "operator_id" => "transform.compare_summary_pair",
          "config" => %{
            "left_prefix" => "heat",
            "right_prefix" => "thermo",
            "include_ratio" => false,
            "include_percent_change" => false
          },
          "inputs" => [
            %{
              "id" => "left",
              "artifact_type" => "report/summary",
              "dataset_value" => "heat_summary"
            },
            %{
              "id" => "right",
              "artifact_type" => "report/summary",
              "dataset_value" => "thermo_summary"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "report/summary",
              "dataset_value" => "comparison_summary"
            }
          ]
        },
        %{
          "id" => "export_json",
          "kind" => "export",
          "operator_id" => "export.summary_json",
          "inputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "comparison_summary"
            }
          ],
          "outputs" => [
            %{"id" => "json", "artifact_type" => "export/json", "dataset_value" => "summary_json"}
          ]
        },
        %{
          "id" => "json_output",
          "kind" => "output",
          "inputs" => [
            %{"id" => "json", "artifact_type" => "export/json", "dataset_value" => "summary_json"}
          ],
          "outputs" => []
        }
      ],
      "edges" => [
        edge(
          "e0",
          "heat_model",
          "model",
          "solve_heat",
          "model",
          "study_model/heat_plane_quad_2d",
          "heat_model"
        ),
        edge(
          "e1",
          "solve_heat",
          "result",
          "extract_heat_summary",
          "result",
          "result/heat_plane_quad_2d",
          "heat_result"
        ),
        edge(
          "e2",
          "solve_heat",
          "result",
          "bridge_temperature",
          "heat_result",
          "result/heat_plane_quad_2d",
          "heat_result"
        ),
        edge(
          "e3",
          "bridge_temperature",
          "thermo_model",
          "solve_thermo",
          "model",
          "study_model/thermal_plane_quad_2d",
          "thermo_model"
        ),
        edge(
          "e4",
          "solve_thermo",
          "result",
          "extract_thermo_summary",
          "result",
          "result/thermal_plane_quad_2d",
          "thermo_result"
        ),
        edge(
          "e5",
          "extract_heat_summary",
          "summary",
          "compare_summaries",
          "left",
          "report/summary",
          "heat_summary"
        ),
        edge(
          "e6",
          "extract_thermo_summary",
          "summary",
          "compare_summaries",
          "right",
          "report/summary",
          "thermo_summary"
        ),
        edge(
          "e7",
          "compare_summaries",
          "result",
          "export_json",
          "summary",
          "report/summary",
          "comparison_summary"
        ),
        edge("e8", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
    }
  end

  defp dataset_value(id, data_class, semantic_type, element_type \\ "json_object"),
    do:
      WorkflowCatalogSupport.workflow_dataset_value_info(
        id,
        data_class,
        semantic_type,
        element_type
      )

  defp heat_triangle_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "t0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "t1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "t2",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        }
      ],
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
  end
end
