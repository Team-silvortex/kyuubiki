defmodule KyuubikiWeb.WorkflowTemplateMaterialCardGraphBuilder do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def graph, do: graph("single")

  def graph("single") do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-card-preflight-json",
      "name" => "Material card preflight JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_card_preflight/v1",
          [
            value("material_card", "model", "material/card"),
            value("material_card_preflight", "result", "report/summary"),
            value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "material_card_preflight"}
        ),
      "entry_nodes" => ["material_card_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(),
        validate_node(),
        export_node(),
        output_node()
      ],
      "edges" => [
        edge(
          "e0",
          "material_card_input",
          "card",
          "validate_card",
          "card",
          "material/card",
          "material_card"
        ),
        edge(
          "e1",
          "validate_card",
          "report",
          "export_json",
          "summary",
          "report/summary",
          "material_card_preflight"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  def graph("batch") do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-card-batch-preflight-json",
      "name" => "Material card batch preflight JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_card_batch_preflight/v1",
          [
            value("material_cards", "model", "material/card_collection"),
            value("material_card_batch_preflight", "result", "report/summary"),
            value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "material_card_batch_preflight"}
        ),
      "entry_nodes" => ["material_cards_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("batch"),
        validate_node("batch"),
        export_node("material_card_batch_preflight"),
        output_node()
      ],
      "edges" => [
        edge(
          "e0",
          "material_cards_input",
          "cards",
          "validate_cards",
          "cards",
          "material/card_collection",
          "material_cards"
        ),
        edge(
          "e1",
          "validate_cards",
          "report",
          "export_json",
          "summary",
          "report/summary",
          "material_card_batch_preflight"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  def graph("screening_score") do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-card-screening-score-json",
      "name" => "Material card screening score JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_card_screening_score/v1",
          [
            value("material_cards", "model", "material/card_collection"),
            value("material_card_batch_preflight", "result", "report/summary"),
            value("material_card_candidate_summaries", "result", "report/summary_collection"),
            value("material_score", "result", "report/summary"),
            value("material_card_screening_report", "result", "report/summary"),
            value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "material_card_screening_score"}
        ),
      "entry_nodes" => ["material_cards_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("batch"),
        validate_node("batch"),
        candidate_summary_node(),
        score_node(),
        screening_report_node(),
        export_node("material_card_screening_report"),
        output_node()
      ],
      "edges" => screening_score_edges()
    }
  end

  def graph("screening_experiment_plan") do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-card-screening-experiment-plan-json",
      "name" => "Material card screening experiment plan JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_card_screening_experiment_plan/v1",
          [
            value("material_cards", "model", "material/card_collection"),
            value("material_card_batch_preflight", "result", "report/summary"),
            value("material_card_candidate_summaries", "result", "report/summary_collection"),
            value("material_score", "result", "report/summary"),
            value("material_card_screening_report", "result", "report/summary"),
            value("material_experiment_plan", "result", "report/summary"),
            value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "material_card_screening_experiment_plan"}
        ),
      "entry_nodes" => ["material_cards_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("batch"),
        validate_node("batch"),
        candidate_summary_node(),
        score_node(),
        screening_report_node(),
        experiment_plan_node(),
        export_node("material_experiment_plan"),
        output_node()
      ],
      "edges" => screening_experiment_plan_edges()
    }
  end

  defp screening_score_edges do
    [
      edge(
        "e0",
        "material_cards_input",
        "cards",
        "validate_cards",
        "cards",
        "material/card_collection",
        "material_cards"
      ),
      edge(
        "e1",
        "validate_cards",
        "report",
        "build_candidates",
        "batch",
        "report/summary",
        "material_card_batch_preflight"
      ),
      edge(
        "e2",
        "build_candidates",
        "candidates",
        "score_candidates",
        "candidates",
        "report/summary_collection",
        "material_card_candidate_summaries"
      ),
      edge(
        "e3",
        "score_candidates",
        "score",
        "explain_screening",
        "score",
        "report/summary",
        "material_score"
      ),
      edge(
        "e4",
        "explain_screening",
        "report",
        "export_json",
        "summary",
        "report/summary",
        "material_card_screening_report"
      ),
      edge("e5", "export_json", "json", "json_output", "json", "export/json", "summary_json")
    ]
  end

  defp screening_experiment_plan_edges do
    [
      edge(
        "e0",
        "material_cards_input",
        "cards",
        "validate_cards",
        "cards",
        "material/card_collection",
        "material_cards"
      ),
      edge(
        "e1",
        "validate_cards",
        "report",
        "build_candidates",
        "batch",
        "report/summary",
        "material_card_batch_preflight"
      ),
      edge(
        "e2",
        "build_candidates",
        "candidates",
        "score_candidates",
        "candidates",
        "report/summary_collection",
        "material_card_candidate_summaries"
      ),
      edge(
        "e3",
        "score_candidates",
        "score",
        "explain_screening",
        "score",
        "report/summary",
        "material_score"
      ),
      edge(
        "e4",
        "explain_screening",
        "report",
        "plan_experiments",
        "screening",
        "report/summary",
        "material_card_screening_report"
      ),
      edge(
        "e5",
        "plan_experiments",
        "plan",
        "export_json",
        "summary",
        "report/summary",
        "material_experiment_plan"
      ),
      edge("e6", "export_json", "json", "json_output", "json", "export/json", "summary_json")
    ]
  end

  defp input_node, do: input_node("single")

  defp input_node("single") do
    %{
      "id" => "material_card_input",
      "kind" => "input",
      "outputs" => [port("card", "material/card", "material_card")]
    }
  end

  defp input_node("batch") do
    %{
      "id" => "material_cards_input",
      "kind" => "input",
      "outputs" => [port("cards", "material/card_collection", "material_cards")]
    }
  end

  defp validate_node, do: validate_node("single")

  defp validate_node("single") do
    %{
      "id" => "validate_card",
      "kind" => "transform",
      "operator_id" => "transform.validate_material_card",
      "config" => validation_config(),
      "inputs" => [port("card", "material/card", "material_card")],
      "outputs" => [port("report", "report/summary", "material_card_preflight")]
    }
  end

  defp validate_node("batch") do
    %{
      "id" => "validate_cards",
      "kind" => "transform",
      "operator_id" => "transform.validate_material_card_batch",
      "config" => validation_config(),
      "inputs" => [port("cards", "material/card_collection", "material_cards")],
      "outputs" => [port("report", "report/summary", "material_card_batch_preflight")]
    }
  end

  defp candidate_summary_node do
    %{
      "id" => "build_candidates",
      "kind" => "transform",
      "operator_id" => "transform.build_material_card_candidate_summaries",
      "config" => %{},
      "inputs" => [port("batch", "report/summary", "material_card_batch_preflight")],
      "outputs" => [
        port("candidates", "report/summary_collection", "material_card_candidate_summaries")
      ]
    }
  end

  defp score_node do
    %{
      "id" => "score_candidates",
      "kind" => "transform",
      "operator_id" => "transform.score_material_candidates",
      "config" => %{
        "criteria" => [
          %{"field" => "material_trust_score", "goal" => "max", "weight" => 0.35},
          %{"field" => "material_param_thermal_conductivity", "goal" => "max", "weight" => 0.3},
          %{"field" => "material_param_youngs_modulus", "goal" => "max", "weight" => 0.2},
          %{"field" => "material_issue_count", "goal" => "min", "weight" => 0.15}
        ]
      },
      "inputs" => [
        port("candidates", "report/summary_collection", "material_card_candidate_summaries")
      ],
      "outputs" => [port("score", "report/summary", "material_score")]
    }
  end

  defp screening_report_node do
    %{
      "id" => "explain_screening",
      "kind" => "transform",
      "operator_id" => "transform.explain_material_card_screening",
      "config" => %{"label" => "material-card-screening"},
      "inputs" => [port("score", "report/summary", "material_score")],
      "outputs" => [port("report", "report/summary", "material_card_screening_report")]
    }
  end

  defp experiment_plan_node do
    %{
      "id" => "plan_experiments",
      "kind" => "transform",
      "operator_id" => "transform.plan_material_experiments",
      "config" => %{"top_k" => 2, "label" => "material-card-screening"},
      "inputs" => [port("screening", "report/summary", "material_card_screening_report")],
      "outputs" => [port("plan", "report/summary", "material_experiment_plan")]
    }
  end

  defp export_node, do: export_node("material_card_preflight")

  defp export_node(dataset_value) do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", dataset_value)],
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

  defp validation_config do
    %{
      "required_parameters" => ["youngs_modulus", "thermal_conductivity"],
      "expected_units" => %{
        "youngs_modulus" => "Pa",
        "thermal_conductivity" => "W/(m*K)"
      }
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

  defp value(id, data_class, semantic_type, element_type \\ "json_object") do
    WorkflowCatalogSupport.workflow_dataset_value_info(
      id,
      data_class,
      semantic_type,
      element_type
    )
  end
end
