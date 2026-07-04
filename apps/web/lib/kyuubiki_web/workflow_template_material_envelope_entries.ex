defmodule KyuubikiWeb.WorkflowTemplateMaterialEnvelopeEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      entry(
        "workflow.material-study-envelope-ranking-json",
        "Material study envelope ranking JSON",
        "Composes multi-domain material envelopes, ranks candidates, extracts a Pareto frontier, and exports the combined decision bundle.",
        ["material", "envelope", "ranking", "pareto", "multi_physics", "optimization"],
        graph(),
        [%{"node_id" => "material_rows", "artifact_type" => "report/summary_collection"}],
        [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
      )
    ]
  end

  defp graph do
    graph(
      [
        input_node("material_rows", "rows", "report/summary_collection", "material_rows"),
        transform_node(
          "compose_envelopes",
          "transform.compose_material_study_envelope",
          %{},
          "rows",
          "report/summary_collection",
          "material_rows",
          "envelopes",
          "report/summary_collection",
          "material_envelopes"
        ),
        transform_node(
          "rank_envelopes",
          "transform.rank_material_candidates",
          %{"margin_prefix" => "material_envelope"},
          "envelopes",
          "report/summary_collection",
          "material_envelopes",
          "ranking",
          "report/summary",
          "material_envelope_ranking"
        ),
        transform_node(
          "pareto_envelopes",
          "transform.extract_material_pareto_frontier",
          %{
            "feasible_field" => "material_envelope_status",
            "objectives" => [
              %{"field" => "material_envelope_score", "goal" => "min"},
              %{"field" => "material_envelope_safety_factor", "goal" => "max"}
            ]
          },
          "envelopes",
          "report/summary_collection",
          "material_envelopes",
          "pareto",
          "report/summary",
          "material_envelope_pareto"
        ),
        bundle_node(),
        export_node(),
        output_node()
      ],
      edges()
    )
  end

  defp graph(nodes, edges) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-study-envelope-ranking-json",
      "name" => "Material study envelope ranking JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_study_envelope_ranking/v1",
          dataset_values(),
          %{"workflow_family" => "material_study_envelope_ranking"}
        ),
      "entry_nodes" => ["material_rows"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => nodes,
      "edges" => edges
    }
  end

  defp bundle_node do
    %{
      "id" => "bundle_decision",
      "kind" => "transform",
      "operator_id" => "transform.compose_diagnostics_bundle",
      "config" => %{"include_non_diagnostics" => true},
      "inputs" => [
        port("ranking", "report/summary", "material_envelope_ranking"),
        port("pareto", "report/summary", "material_envelope_pareto")
      ],
      "outputs" => [port("bundle", "report/summary", "material_envelope_decision_bundle")]
    }
  end

  defp edges do
    [
      edge(
        "e0",
        "material_rows",
        "rows",
        "compose_envelopes",
        "rows",
        "report/summary_collection",
        "material_rows"
      ),
      edge(
        "e1",
        "compose_envelopes",
        "envelopes",
        "rank_envelopes",
        "envelopes",
        "report/summary_collection",
        "material_envelopes"
      ),
      edge(
        "e2",
        "compose_envelopes",
        "envelopes",
        "pareto_envelopes",
        "envelopes",
        "report/summary_collection",
        "material_envelopes"
      ),
      edge(
        "e3",
        "rank_envelopes",
        "ranking",
        "bundle_decision",
        "ranking",
        "report/summary",
        "material_envelope_ranking"
      ),
      edge(
        "e4",
        "pareto_envelopes",
        "pareto",
        "bundle_decision",
        "pareto",
        "report/summary",
        "material_envelope_pareto"
      ),
      edge(
        "e5",
        "bundle_decision",
        "bundle",
        "export_json",
        "summary",
        "report/summary",
        "material_envelope_decision_bundle"
      ),
      edge("e6", "export_json", "json", "json_output", "json", "export/json", "summary_json")
    ]
  end

  defp dataset_values do
    [
      value("material_rows", "input", "report/summary_collection"),
      value("material_envelopes", "result", "report/summary_collection"),
      value("material_envelope_ranking", "result", "report/summary"),
      value("material_envelope_pareto", "result", "report/summary"),
      value("material_envelope_decision_bundle", "result", "report/summary"),
      value("summary_json", "export", "export/json", "utf8_text")
    ]
  end

  defp input_node(id, port_id, artifact_type, dataset_value),
    do: %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port(port_id, artifact_type, dataset_value)]
    }

  defp transform_node(
         id,
         operator_id,
         config,
         in_id,
         in_type,
         in_value,
         out_id,
         out_type,
         out_value
       ),
       do: %{
         "id" => id,
         "kind" => "transform",
         "operator_id" => operator_id,
         "config" => config,
         "inputs" => [port(in_id, in_type, in_value)],
         "outputs" => [port(out_id, out_type, out_value)]
       }

  defp export_node,
    do: %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", "material_envelope_decision_bundle")],
      "outputs" => [port("json", "export/json", "summary_json")]
    }

  defp output_node,
    do: %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", "summary_json")],
      "outputs" => []
    }

  defp entry(id, name, summary, tags, graph, inputs, outputs),
    do: %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => ["material"],
      "capability_tags" => tags ++ ["headless_safe"],
      "graph" => graph,
      "entry_inputs" => inputs,
      "output_artifacts" => outputs
    }

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value),
    do: %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
    }

  defp port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  defp value(id, data_class, semantic_type, element_type \\ "json_object"),
    do:
      WorkflowCatalogSupport.workflow_dataset_value_info(
        id,
        data_class,
        semantic_type,
        element_type
      )
end
