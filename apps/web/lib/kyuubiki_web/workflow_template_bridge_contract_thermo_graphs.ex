defmodule KyuubikiWeb.WorkflowTemplateBridgeContractThermoGraphs do
  @moduledoc false

  import KyuubikiWeb.WorkflowTemplateBridgeContractGraphSupport, only: [edge: 7]

  alias KyuubikiWeb.WorkflowCatalogSupport

  def heat_to_thermo_quad_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.heat-to-thermo-quad-2d",
      "name" => "Heat to thermo quad",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.heat_to_thermo_quad/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "heat_model",
              "model",
              "study_model/heat_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "heat_result",
              "result",
              "result/heat_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "thermo_model",
              "model",
              "study_model/thermal_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "thermo_result",
              "result",
              "result/thermal_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "thermo_summary",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "summary_json",
              "export",
              "export/json",
              "utf8_text"
            )
          ],
          %{"workflow_family" => "heat_to_thermo_quad"}
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
          "id" => "validate_bridge_temperature",
          "kind" => "transform",
          "operator_id" => "transform.validate_heat_thermo_bridge",
          "config" => %{
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          },
          "inputs" => [
            %{
              "id" => "heat_result",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            },
            %{
              "id" => "thermo_model",
              "artifact_type" => "study_model/thermal_plane_quad_2d",
              "dataset_value" => "thermo_model"
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
          "id" => "extract_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
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
          "bridge_temperature",
          "heat_result",
          "result/heat_plane_quad_2d",
          "heat_result"
        ),
        edge(
          "e2",
          "solve_heat",
          "result",
          "validate_bridge_temperature",
          "heat_result",
          "result/heat_plane_quad_2d",
          "heat_result"
        ),
        edge(
          "e3",
          "bridge_temperature",
          "thermo_model",
          "validate_bridge_temperature",
          "thermo_model",
          "study_model/thermal_plane_quad_2d",
          "thermo_model"
        ),
        edge(
          "e4",
          "validate_bridge_temperature",
          "thermo_model",
          "solve_thermo",
          "model",
          "study_model/thermal_plane_quad_2d",
          "thermo_model"
        ),
        edge(
          "e5",
          "solve_thermo",
          "result",
          "extract_summary",
          "result",
          "result/thermal_plane_quad_2d",
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
end
