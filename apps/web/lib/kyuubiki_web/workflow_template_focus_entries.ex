defmodule KyuubikiWeb.WorkflowTemplateFocusEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [focus_heat_to_thermo_bridge_entry()]
  end

  defp focus_heat_to_thermo_bridge_entry do
    %{
      "id" => "workflow.focus-heat-to-thermo-bridge-json",
      "name" => "Focus heat to thermo bridge JSON",
      "version" => "1.0.0",
      "summary" =>
        "Use a diagnostics report focus payload to drive a heat-to-thermo bridge execution and emit a runnable bridge result artifact.",
      "domains" => ["thermal", "thermo_mechanical"],
      "capability_tags" => [
        "focus_chain",
        "diagnostics",
        "workflow_bridge",
        "thermal",
        "thermo_mechanical",
        "headless_safe"
      ],
      "graph" => build_graph(),
      "entry_inputs" => [
        %{
          "node_id" => "report_input",
          "artifact_type" => "artifact/json",
          "description" => "Diagnostics report payload carrying focus payload candidates."
        },
        %{
          "node_id" => "heat_input",
          "artifact_type" => "result/heat_plane_quad_2d",
          "description" => "Heat solve result used as the upstream bridge payload."
        }
      ],
      "output_artifacts" => [
        %{
          "node_id" => "bridge_output",
          "artifact_type" => "artifact/json",
          "description" => "JSON focus bridge result wrapping the generated thermo model."
        }
      ]
    }
  end

  defp build_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.focus-heat-to-thermo-bridge-json",
      "name" => "Focus heat to thermo bridge JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.focus_heat_to_thermo_bridge/v1",
          [
            dataset_value("report_payload", "result", "artifact/json"),
            dataset_value("heat_result", "result", "result/heat_plane_quad_2d"),
            dataset_value("focus_payload", "result", "artifact/json"),
            dataset_value("focus_chain_input", "result", "artifact/json"),
            dataset_value("bridge_request", "result", "artifact/json"),
            dataset_value("bridge_execution", "result", "artifact/json"),
            dataset_value("bridge_result", "result", "artifact/json")
          ],
          %{"workflow_family" => "focus_heat_to_thermo_bridge"}
        ),
      "entry_nodes" => ["report_input", "heat_input"],
      "output_nodes" => ["bridge_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("report_input", "report", "report_payload", "artifact/json"),
        input_node("heat_input", "heat_result", "heat_result", "result/heat_plane_quad_2d"),
        %{
          "id" => "select_focus",
          "kind" => "transform",
          "operator_id" => "transform.select_focus_payload",
          "config" => %{"metric_id" => "thermo.stress_peak"},
          "inputs" => [
            %{
              "id" => "report",
              "artifact_type" => "artifact/json",
              "dataset_value" => "report_payload"
            }
          ],
          "outputs" => [
            %{
              "id" => "focus",
              "artifact_type" => "artifact/json",
              "dataset_value" => "focus_payload"
            }
          ]
        },
        %{
          "id" => "compose_chain",
          "kind" => "transform",
          "operator_id" => "transform.compose_focus_chain_input",
          "config" => %{
            "target_operator" => "bridge.temperature_field_to_thermo_quad_2d",
            "stage" => "thermo_bridge"
          },
          "inputs" => [
            %{
              "id" => "focus",
              "artifact_type" => "artifact/json",
              "dataset_value" => "focus_payload"
            }
          ],
          "outputs" => [
            %{
              "id" => "chain",
              "artifact_type" => "artifact/json",
              "dataset_value" => "focus_chain_input"
            }
          ]
        },
        %{
          "id" => "compose_request",
          "kind" => "transform",
          "operator_id" => "transform.compose_focus_bridge_request",
          "config" => %{
            "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example(),
            "bridge_payload_source" => "heat_input.heat_result"
          },
          "inputs" => [
            %{
              "id" => "chain",
              "artifact_type" => "artifact/json",
              "dataset_value" => "focus_chain_input"
            }
          ],
          "outputs" => [
            %{
              "id" => "request",
              "artifact_type" => "artifact/json",
              "dataset_value" => "bridge_request"
            }
          ]
        },
        %{
          "id" => "resolve_execution",
          "kind" => "transform",
          "operator_id" => "transform.resolve_focus_bridge_execution",
          "config" => %{},
          "inputs" => [
            %{
              "id" => "request",
              "artifact_type" => "artifact/json",
              "dataset_value" => "bridge_request"
            },
            %{
              "id" => "bridge_payload",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "execution",
              "artifact_type" => "artifact/json",
              "dataset_value" => "bridge_execution"
            }
          ]
        },
        %{
          "id" => "execute_bridge",
          "kind" => "transform",
          "operator_id" => "transform.execute_focus_bridge_execution",
          "config" => %{},
          "inputs" => [
            %{
              "id" => "execution",
              "artifact_type" => "artifact/json",
              "dataset_value" => "bridge_execution"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "artifact/json",
              "dataset_value" => "bridge_result"
            }
          ]
        },
        %{
          "id" => "bridge_output",
          "kind" => "output",
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => "artifact/json",
              "dataset_value" => "bridge_result"
            }
          ],
          "outputs" => []
        }
      ],
      "edges" => [
        edge(
          "e0",
          "report_input",
          "report",
          "select_focus",
          "report",
          "artifact/json",
          "report_payload"
        ),
        edge(
          "e1",
          "select_focus",
          "focus",
          "compose_chain",
          "focus",
          "artifact/json",
          "focus_payload"
        ),
        edge(
          "e2",
          "compose_chain",
          "chain",
          "compose_request",
          "chain",
          "artifact/json",
          "focus_chain_input"
        ),
        edge(
          "e3",
          "compose_request",
          "request",
          "resolve_execution",
          "request",
          "artifact/json",
          "bridge_request"
        ),
        edge(
          "e4",
          "heat_input",
          "heat_result",
          "resolve_execution",
          "bridge_payload",
          "result/heat_plane_quad_2d",
          "heat_result"
        ),
        edge(
          "e5",
          "resolve_execution",
          "execution",
          "execute_bridge",
          "execution",
          "artifact/json",
          "bridge_execution"
        ),
        edge(
          "e6",
          "execute_bridge",
          "result",
          "bridge_output",
          "result",
          "artifact/json",
          "bridge_result"
        )
      ]
    }
  end

  defp input_node(id, port_id, dataset_value, artifact_type) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [
        %{"id" => port_id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}
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
end
