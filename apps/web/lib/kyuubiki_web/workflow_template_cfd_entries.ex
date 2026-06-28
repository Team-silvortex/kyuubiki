defmodule KyuubikiWeb.WorkflowTemplateCfdEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [stokes_flow_quad_summary_entry()]
  end

  defp stokes_flow_quad_summary_entry do
    custom_entry(
      "workflow.stokes-flow-quad-summary-json",
      "Stokes flow quad summary JSON",
      "Solves a low-Reynolds 2D Stokes flow quad model, extracts CFD diagnostics, and exports a JSON summary artifact.",
      ["fluid"],
      ["fluid", "cfd", "stokes", "flow", "diagnostics", "quad", "2d", "json"],
      stokes_flow_quad_summary_graph(),
      [
        %{
          "node_id" => "stokes_flow_model",
          "artifact_type" => "study_model/stokes_flow_quad_2d",
          "description" =>
            "Steady Stokes flow quad model used as the CFD workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded CFD diagnostics for the Stokes flow solve."
        }
      ]
    )
  end

  defp stokes_flow_quad_summary_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.stokes-flow-quad-summary-json",
      "name" => "Stokes flow quad summary JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.stokes_flow_quad_summary/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "stokes_flow_model",
              "model",
              "study_model/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "stokes_flow_result",
              "result",
              "result/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "cfd_diagnostics",
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
          %{"workflow_family" => "stokes_flow_quad_summary"}
        ),
      "entry_nodes" => ["stokes_flow_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("stokes_flow_model", "study_model/stokes_flow_quad_2d", "stokes_flow_model"),
        solve_node(),
        diagnostics_node(),
        export_node(),
        output_node()
      ],
      "edges" => [
        edge(
          "e1",
          "stokes_flow_model",
          "model",
          "solve_stokes_flow",
          "model",
          "study_model/stokes_flow_quad_2d",
          "stokes_flow_model"
        ),
        edge(
          "e2",
          "solve_stokes_flow",
          "result",
          "extract_cfd_diagnostics",
          "result",
          "result/stokes_flow_quad_2d",
          "stokes_flow_result"
        ),
        edge(
          "e3",
          "extract_cfd_diagnostics",
          "summary",
          "export_json",
          "summary",
          "report/summary",
          "cfd_diagnostics"
        ),
        edge("e4", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp input_node(id, artifact_type, dataset_value) do
    %{"id" => id, "kind" => "input", "outputs" => [port("model", artifact_type, dataset_value)]}
  end

  defp solve_node do
    %{
      "id" => "solve_stokes_flow",
      "kind" => "solve",
      "operator_id" => "solve.stokes_flow_quad_2d",
      "inputs" => [port("model", "study_model/stokes_flow_quad_2d", "stokes_flow_model")],
      "outputs" => [port("result", "result/stokes_flow_quad_2d", "stokes_flow_result")]
    }
  end

  defp diagnostics_node do
    %{
      "id" => "extract_cfd_diagnostics",
      "kind" => "extract",
      "operator_id" => "extract.stokes_flow_result_diagnostics",
      "inputs" => [port("result", "result/stokes_flow_quad_2d", "stokes_flow_result")],
      "outputs" => [port("summary", "report/summary", "cfd_diagnostics")]
    }
  end

  defp export_node do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", "cfd_diagnostics")],
      "outputs" => [port("json", "export/json", "summary_json")]
    }
  end

  defp output_node do
    %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", "summary_json")],
      "outputs" => []
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

  defp port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  defp custom_entry(id, name, summary, domains, capability_tags, graph, entry_inputs, outputs) do
    %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => domains,
      "capability_tags" => capability_tags,
      "graph" => graph,
      "entry_inputs" => entry_inputs,
      "output_artifacts" => outputs
    }
  end
end
