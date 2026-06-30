defmodule KyuubikiWeb.WorkflowTemplateMaterialCardEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateMaterialCardGraphBuilder

  def list do
    [
      material_card_preflight_entry(),
      material_card_batch_preflight_entry(),
      material_card_screening_score_entry(),
      material_card_screening_experiment_plan_entry()
    ]
  end

  defp material_card_preflight_entry do
    %{
      "id" => "workflow.material-card-preflight-json",
      "name" => "Material card preflight JSON",
      "version" => "1.0.0",
      "summary" =>
        "Validates a versioned material card before material research workflows consume it.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "material_card",
        "preflight",
        "provenance",
        "unit_check",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateMaterialCardGraphBuilder.graph(),
      "entry_inputs" => [
        %{"node_id" => "material_card_input", "artifact_type" => "material/card"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end

  defp material_card_batch_preflight_entry do
    %{
      "id" => "workflow.material-card-batch-preflight-json",
      "name" => "Material card batch preflight JSON",
      "version" => "1.0.0",
      "summary" =>
        "Validates candidate material cards as a batch before material screening workflows consume them.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "material_card",
        "batch_preflight",
        "candidate_selection",
        "unit_check",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateMaterialCardGraphBuilder.graph("batch"),
      "entry_inputs" => [
        %{"node_id" => "material_cards_input", "artifact_type" => "material/card_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end

  defp material_card_screening_score_entry do
    %{
      "id" => "workflow.material-card-screening-score-json",
      "name" => "Material card screening score JSON",
      "version" => "1.0.0",
      "summary" =>
        "Validates candidate material cards, converts preflight reports into candidate summaries, scores them, and exports explainable rankings.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "material_card",
        "screening",
        "score",
        "candidate_selection",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateMaterialCardGraphBuilder.graph("screening_score"),
      "entry_inputs" => [
        %{"node_id" => "material_cards_input", "artifact_type" => "material/card_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end

  defp material_card_screening_experiment_plan_entry do
    %{
      "id" => "workflow.material-card-screening-experiment-plan-json",
      "name" => "Material card screening experiment plan JSON",
      "version" => "1.0.0",
      "summary" =>
        "Validates candidate material cards, explains screening rankings, turns them into a prioritized experiment plan, and exports JSON.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "material_card",
        "screening",
        "experiment_plan",
        "candidate_selection",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateMaterialCardGraphBuilder.graph("screening_experiment_plan"),
      "entry_inputs" => [
        %{"node_id" => "material_cards_input", "artifact_type" => "material/card_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end
end
