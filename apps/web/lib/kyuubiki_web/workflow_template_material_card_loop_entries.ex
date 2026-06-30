defmodule KyuubikiWeb.WorkflowTemplateMaterialCardLoopEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowTemplateMaterialAnalysisEntries, as: Analysis

  def list do
    [
      material_card_next_round_request_entry(),
      material_card_exploration_snapshot_entry()
    ]
  end

  defp material_card_next_round_request_entry do
    %{
      "id" => "workflow.material-card-next-round-request-json",
      "name" => "Material card next-round request JSON",
      "version" => "1.0.0",
      "summary" =>
        "Analyzes material-card screening experiment results, decides the loop state, and exports the next material-card candidate request.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "material_card",
        "next_round",
        "automation",
        "candidate_generation",
        "headless_safe"
      ],
      "graph" => graph(),
      "entry_inputs" => [
        %{"node_id" => "results_input", "artifact_type" => "report/summary_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end

  defp material_card_exploration_snapshot_entry do
    %{
      "id" => "workflow.material-card-exploration-snapshot-json",
      "name" => "Material card exploration snapshot JSON",
      "version" => "1.0.0",
      "summary" =>
        "Builds a durable material-card exploration snapshot from experiment results and next-round request state.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "material_card",
        "snapshot",
        "state",
        "automation",
        "headless_safe"
      ],
      "graph" => graph("snapshot"),
      "entry_inputs" => [
        %{"node_id" => "results_input", "artifact_type" => "report/summary_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end

  defp graph, do: graph("next_round")

  defp graph("next_round") do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-card-next-round-request-json",
      "name" => "Material card next-round request JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_card_next_round_request/v1",
          dataset_values(),
          %{"workflow_family" => "material_card_next_round_request"}
        ),
      "entry_nodes" => ["results_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        Analysis.result_input_node(),
        Analysis.analyze_node(),
        decision_node(),
        next_round_node(),
        export_node("material_card_next_round_request"),
        Analysis.output_node()
      ],
      "edges" => next_round_edges()
    }
  end

  defp graph("snapshot") do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-card-exploration-snapshot-json",
      "name" => "Material card exploration snapshot JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_card_exploration_snapshot/v1",
          dataset_values(),
          %{"workflow_family" => "material_card_exploration_snapshot"}
        ),
      "entry_nodes" => ["results_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        Analysis.result_input_node(),
        Analysis.analyze_node(),
        decision_node(),
        next_round_node(),
        snapshot_node(),
        export_node("material_card_exploration_snapshot"),
        Analysis.output_node()
      ],
      "edges" => snapshot_edges()
    }
  end

  defp next_round_edges do
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
        "material_card_next_round_request"
      ),
      Analysis.edge("e4", "export_json", "json", "json_output", "json", "summary_json")
    ]
  end

  defp snapshot_edges do
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
        "material_card_next_round_request"
      ),
      Analysis.edge(
        "e4",
        "build_snapshot",
        "snapshot",
        "export_json",
        "summary",
        "material_card_exploration_snapshot"
      ),
      Analysis.edge("e5", "export_json", "json", "json_output", "json", "summary_json")
    ]
  end

  defp decision_node do
    %{
      "id" => "decide_iteration",
      "kind" => "transform",
      "operator_id" => "transform.decide_material_iteration",
      "config" => %{"target_score" => 0.78, "min_validated" => 1, "max_rounds" => 6},
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
      "config" => %{
        "requested_candidate_count" => 4,
        "constraints" => %{
          "candidate_schema" => "kyuubiki.material-card/v1",
          "candidate_collection_dataset" => "material_cards",
          "screening_workflow" => "workflow.material-card-screening-experiment-plan-json"
        }
      },
      "inputs" => [Analysis.port("decision", "report/summary", "material_iteration_decision")],
      "outputs" => [
        Analysis.port("request", "report/summary", "material_card_next_round_request")
      ]
    }
  end

  defp snapshot_node do
    %{
      "id" => "build_snapshot",
      "kind" => "transform",
      "operator_id" => "transform.build_material_exploration_snapshot",
      "config" => %{
        "metadata" => %{
          "workflow_family" => "material_card_exploration",
          "candidate_schema" => "kyuubiki.material-card/v1"
        }
      },
      "inputs" => [Analysis.port("request", "report/summary", "material_card_next_round_request")],
      "outputs" => [
        Analysis.port("snapshot", "report/summary", "material_card_exploration_snapshot")
      ]
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

  defp dataset_values do
    Analysis.experiment_result_values() ++
      [
        Analysis.value("material_iteration_decision", "result", "report/summary"),
        Analysis.value("material_card_next_round_request", "result", "report/summary"),
        Analysis.value("material_card_exploration_snapshot", "result", "report/summary")
      ]
  end
end
