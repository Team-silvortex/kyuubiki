defmodule KyuubikiWeb.WorkflowTemplateSolverContractGraphs do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def electrostatic_plane_quad_summary_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-plane-quad-2d",
      "name" => "Electrostatic plane quad",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.electrostatic_plane_quad_summary/v1",
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
              "electrostatic_summary",
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
          %{"workflow_family" => "electrostatic_plane_quad_summary"}
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
          "id" => "extract_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
          "config" => %{
            "fields" => ["max_potential", "max_electric_field", "max_flux_density"]
          },
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "electrostatic_summary"
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
              "dataset_value" => "electrostatic_summary"
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
          "extract_summary",
          "result",
          "result/electrostatic_plane_quad_2d",
          "electrostatic_result"
        ),
        edge(
          "e2",
          "extract_summary",
          "summary",
          "export_json",
          "summary",
          "report/summary",
          "electrostatic_summary"
        ),
        edge("e3", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  def electrostatic_plane_quad_hotspot_alert_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-plane-quad-hotspot-alert",
      "name" => "Electrostatic plane quad hotspot alert",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.electrostatic_plane_quad_hotspot_alert/v1",
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
              "hotspot_summary",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "alert_markdown",
              "export",
              "export/markdown",
              "utf8_text"
            )
          ],
          %{"workflow_family" => "electrostatic_plane_quad_hotspot_alert"}
        ),
      "entry_nodes" => ["electrostatic_model"],
      "output_nodes" => ["markdown_output"],
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
          "id" => "extract_hotspots",
          "kind" => "extract",
          "operator_id" => "extract.field_hotspots",
          "config" => %{
            "source" => "elements",
            "field" => "electric_field_magnitude",
            "percentile" => 90,
            "sample_limit" => 4
          },
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "hotspot_summary"
            }
          ]
        },
        %{
          "id" => "export_alert",
          "kind" => "export",
          "operator_id" => "export.alert_markdown",
          "config" => %{
            "title" => "Electrostatic Quad Hotspot Alert",
            "severity" => "warning",
            "summary" =>
              "The electrostatic quad solve produced high electric-field hotspot regions.",
            "sample_field" => "electric_field_magnitude_hotspot_samples",
            "sample_value_key" => "electric_field_magnitude"
          },
          "inputs" => [
            %{
              "id" => "summary",
              "artifact_type" => "report/summary",
              "dataset_value" => "hotspot_summary"
            }
          ],
          "outputs" => [
            %{
              "id" => "markdown",
              "artifact_type" => "export/markdown",
              "dataset_value" => "alert_markdown"
            }
          ]
        },
        %{
          "id" => "markdown_output",
          "kind" => "output",
          "inputs" => [
            %{
              "id" => "markdown",
              "artifact_type" => "export/markdown",
              "dataset_value" => "alert_markdown"
            }
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
          "extract_hotspots",
          "result",
          "result/electrostatic_plane_quad_2d",
          "electrostatic_result"
        ),
        edge(
          "e2",
          "extract_hotspots",
          "summary",
          "export_alert",
          "summary",
          "report/summary",
          "hotspot_summary"
        ),
        edge(
          "e3",
          "export_alert",
          "markdown",
          "markdown_output",
          "markdown",
          "export/markdown",
          "alert_markdown"
        )
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
end
