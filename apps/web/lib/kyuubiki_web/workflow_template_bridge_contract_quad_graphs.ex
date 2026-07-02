defmodule KyuubikiWeb.WorkflowTemplateBridgeContractQuadGraphs do
  @moduledoc false

  import KyuubikiWeb.WorkflowTemplateBridgeContractGraphSupport,
    only: [edge: 7, heat_quad_seed_model_example: 0]

  alias KyuubikiWeb.WorkflowCatalogSupport

  def electrostatic_to_heat_quad_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-to-heat-quad-2d",
      "name" => "Electrostatic to heat quad",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.electrostatic_to_heat_quad/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "electrostatic_model",
              "model",
              "study_model/electrostatic_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "electrostatic_result",
              "result",
              "result/electrostatic_plane_quad_2d"
            ),
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
              "heat_summary",
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
          %{"workflow_family" => "electrostatic_to_heat_quad"}
        ),
      "entry_nodes" => ["electrostatic_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        %{
          "id" => "electrostatic_model",
          "kind" => "input",
          "outputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_model"
            }
          ]
        },
        %{
          "id" => "solve_electrostatic",
          "kind" => "solve",
          "operator_id" => "solve.electrostatic_plane_quad_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            }
          ]
        },
        %{
          "id" => "bridge_field_to_heat",
          "kind" => "transform",
          "operator_id" => "bridge.electrostatic_field_to_heat_quad_2d",
          "config" => %{
            "seed_model" => heat_quad_seed_model_example(),
            "contract" =>
              WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(50.0)
          },
          "inputs" => [
            %{
              "id" => "electrostatic_result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "heat_model",
              "artifact_type" => "study_model/heat_plane_quad_2d",
              "dataset_value" => "heat_model"
            }
          ]
        },
        %{
          "id" => "validate_bridge_field_to_heat",
          "kind" => "transform",
          "operator_id" => "transform.validate_electrostatic_heat_bridge",
          "config" => %{
            "contract" =>
              WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(50.0)
          },
          "inputs" => [
            %{
              "id" => "electrostatic_result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            },
            %{
              "id" => "heat_model",
              "artifact_type" => "study_model/heat_plane_quad_2d",
              "dataset_value" => "heat_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "heat_model",
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
          "id" => "extract_summary",
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
          "id" => "export_json",
          "kind" => "export",
          "operator_id" => "export.summary_json",
          "inputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "heat_summary"
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
          "electrostatic_model",
          "model",
          "solve_electrostatic",
          "model",
          "study_model/electrostatic_plane_quad_2d",
          "electrostatic_model"
        ),
        edge(
          "e1",
          "solve_electrostatic",
          "result",
          "bridge_field_to_heat",
          "electrostatic_result",
          "result/electrostatic_plane_quad_2d",
          "electrostatic_result"
        ),
        edge(
          "e2",
          "solve_electrostatic",
          "result",
          "validate_bridge_field_to_heat",
          "electrostatic_result",
          "result/electrostatic_plane_quad_2d",
          "electrostatic_result"
        ),
        edge(
          "e3",
          "bridge_field_to_heat",
          "heat_model",
          "validate_bridge_field_to_heat",
          "heat_model",
          "study_model/heat_plane_quad_2d",
          "heat_model"
        ),
        edge(
          "e4",
          "validate_bridge_field_to_heat",
          "heat_model",
          "solve_heat",
          "model",
          "study_model/heat_plane_quad_2d",
          "heat_model"
        ),
        edge(
          "e5",
          "solve_heat",
          "result",
          "extract_summary",
          "result",
          "result/heat_plane_quad_2d",
          "heat_result"
        ),
        edge(
          "e6",
          "extract_summary",
          "summary",
          "export_json",
          "summary",
          "report/summary",
          "heat_summary"
        ),
        edge("e7", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end
end
