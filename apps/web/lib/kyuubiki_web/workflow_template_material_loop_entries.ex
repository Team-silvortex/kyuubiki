defmodule KyuubikiWeb.WorkflowTemplateMaterialLoopEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowTemplateMaterialAnalysisEntries, as: Analysis

  def list do
    [
      material_iteration_decision_entry(),
      material_next_round_request_entry(),
      material_exploration_snapshot_entry()
    ]
  end

  defp material_iteration_decision_entry do
    entry(
      "workflow.material-iteration-decision-json",
      "Material iteration decision JSON",
      "Analyzes observed material experiment results, decides whether the exploration loop should stop, continue, or replan, and exports the decision.",
      ["material", "optimization", "iteration_decision", "automation", "validation"],
      decision_graph()
    )
  end

  defp material_next_round_request_entry do
    entry(
      "workflow.material-next-round-request-json",
      "Material next-round request JSON",
      "Analyzes experiment results, decides the material exploration iteration state, and exports a standard next-round orchestration request.",
      ["material", "optimization", "next_round", "orchestration", "automation"],
      next_round_graph()
    )
  end

  defp material_exploration_snapshot_entry do
    entry(
      "workflow.material-exploration-snapshot-json",
      "Material exploration snapshot JSON",
      "Builds a durable material exploration state snapshot from experiment results and next-round orchestration state.",
      ["material", "optimization", "snapshot", "state", "orchestration"],
      snapshot_graph()
    )
  end

  defp decision_graph do
    graph_with_nodes(
      "workflow.material-iteration-decision-json",
      "Material iteration decision JSON",
      "kyuubiki.dataset.material_iteration_decision/v1",
      "material_iteration_decision",
      [
        Analysis.result_input_node(),
        Analysis.analyze_node(),
        decision_node(),
        export_node("material_iteration_decision"),
        Analysis.output_node()
      ],
      [
        Analysis.edge(
          "e0",
          "results_input",
          "results",
          "analyze_results",
          "results",
          "material_experiment_results"
        ),
        Analysis.edge(
          "e1",
          "analyze_results",
          "analysis",
          "decide_iteration",
          "analysis",
          "material_experiment_result_analysis"
        ),
        Analysis.edge(
          "e2",
          "decide_iteration",
          "decision",
          "export_json",
          "summary",
          "material_iteration_decision"
        ),
        Analysis.edge("e3", "export_json", "json", "json_output", "json", "summary_json")
      ]
    )
  end

  defp next_round_graph do
    graph_with_nodes(
      "workflow.material-next-round-request-json",
      "Material next-round request JSON",
      "kyuubiki.dataset.material_next_round_request/v1",
      "material_next_round_request",
      [
        Analysis.result_input_node(),
        Analysis.analyze_node(),
        decision_node(),
        next_round_node(),
        export_node("material_next_round_request"),
        Analysis.output_node()
      ],
      [
        Analysis.edge(
          "e0",
          "results_input",
          "results",
          "analyze_results",
          "results",
          "material_experiment_results"
        ),
        Analysis.edge(
          "e1",
          "analyze_results",
          "analysis",
          "decide_iteration",
          "analysis",
          "material_experiment_result_analysis"
        ),
        Analysis.edge(
          "e2",
          "decide_iteration",
          "decision",
          "prepare_next_round",
          "decision",
          "material_iteration_decision"
        ),
        Analysis.edge(
          "e3",
          "prepare_next_round",
          "request",
          "export_json",
          "summary",
          "material_next_round_request"
        ),
        Analysis.edge("e4", "export_json", "json", "json_output", "json", "summary_json")
      ]
    )
  end

  defp snapshot_graph do
    graph_with_nodes(
      "workflow.material-exploration-snapshot-json",
      "Material exploration snapshot JSON",
      "kyuubiki.dataset.material_exploration_snapshot/v1",
      "material_exploration_snapshot",
      [
        Analysis.result_input_node(),
        Analysis.analyze_node(),
        decision_node(),
        next_round_node(),
        snapshot_node(),
        export_node("material_exploration_snapshot"),
        Analysis.output_node()
      ],
      [
        Analysis.edge(
          "e0",
          "results_input",
          "results",
          "analyze_results",
          "results",
          "material_experiment_results"
        ),
        Analysis.edge(
          "e1",
          "analyze_results",
          "analysis",
          "decide_iteration",
          "analysis",
          "material_experiment_result_analysis"
        ),
        Analysis.edge(
          "e2",
          "decide_iteration",
          "decision",
          "prepare_next_round",
          "decision",
          "material_iteration_decision"
        ),
        Analysis.edge(
          "e3",
          "prepare_next_round",
          "request",
          "build_snapshot",
          "request",
          "material_next_round_request"
        ),
        Analysis.edge(
          "e4",
          "build_snapshot",
          "snapshot",
          "export_json",
          "summary",
          "material_exploration_snapshot"
        ),
        Analysis.edge("e5", "export_json", "json", "json_output", "json", "summary_json")
      ]
    )
  end

  defp decision_node do
    %{
      "id" => "decide_iteration",
      "kind" => "transform",
      "operator_id" => "transform.decide_material_iteration",
      "config" => %{"target_score" => 0.7, "min_validated" => 1, "max_rounds" => 5},
      "inputs" => [
        Analysis.port("analysis", "report/summary", "material_experiment_result_analysis")
      ],
      "outputs" => [Analysis.port("decision", "report/summary", "material_iteration_decision")]
    }
  end

  defp next_round_node do
    %{
      "id" => "prepare_next_round",
      "kind" => "transform",
      "operator_id" => "transform.prepare_material_next_round_request",
      "config" => %{"requested_candidate_count" => 3},
      "inputs" => [Analysis.port("decision", "report/summary", "material_iteration_decision")],
      "outputs" => [Analysis.port("request", "report/summary", "material_next_round_request")]
    }
  end

  defp snapshot_node do
    %{
      "id" => "build_snapshot",
      "kind" => "transform",
      "operator_id" => "transform.build_material_exploration_snapshot",
      "config" => %{},
      "inputs" => [Analysis.port("request", "report/summary", "material_next_round_request")],
      "outputs" => [Analysis.port("snapshot", "report/summary", "material_exploration_snapshot")]
    }
  end

  defp export_node(dataset_value) do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [Analysis.port("summary", "report/summary", dataset_value)],
      "outputs" => [Analysis.port("json", "export/json", "summary_json")]
    }
  end

  defp graph_with_nodes(id, name, dataset_contract_id, family, nodes, edges) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          dataset_contract_id,
          dataset_values(),
          %{"workflow_family" => family}
        ),
      "entry_nodes" => ["results_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => nodes,
      "edges" => edges
    }
  end

  defp dataset_values do
    Analysis.experiment_result_values() ++
      [
        Analysis.value("material_iteration_decision", "result", "report/summary"),
        Analysis.value("material_next_round_request", "result", "report/summary"),
        Analysis.value("material_exploration_snapshot", "result", "report/summary")
      ]
  end

  defp entry(id, name, summary, tags, graph) do
    %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => ["material"],
      "capability_tags" => tags ++ ["headless_safe"],
      "graph" => graph,
      "entry_inputs" => [
        %{"node_id" => "results_input", "artifact_type" => "report/summary_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end
end
