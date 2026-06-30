defmodule KyuubikiWeb.WorkflowTemplateMaterialCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowTemplateCatalog

  @material_workflows %{
    "workflow.material-card-preflight-json" => "transform.validate_material_card",
    "workflow.material-card-batch-preflight-json" => "transform.validate_material_card_batch",
    "workflow.material-card-screening-score-json" =>
      "transform.build_material_card_candidate_summaries",
    "workflow.material-card-screening-experiment-plan-json" =>
      "transform.plan_material_experiments",
    "workflow.material-card-next-round-request-json" =>
      "transform.prepare_material_next_round_request",
    "workflow.material-card-exploration-snapshot-json" =>
      "transform.build_material_exploration_snapshot",
    "workflow.material-margin-summary-json" => "transform.evaluate_material_margins",
    "workflow.material-candidate-ranking-json" => "transform.rank_material_candidates",
    "workflow.material-fatigue-life-json" => "transform.estimate_material_fatigue_life",
    "workflow.material-candidate-score-json" => "transform.score_material_candidates",
    "workflow.material-experiment-plan-json" => "transform.plan_material_experiments",
    "workflow.material-experiment-result-analysis-json" =>
      "transform.analyze_material_experiment_results",
    "workflow.material-iteration-decision-json" => "transform.decide_material_iteration",
    "workflow.material-next-round-request-json" =>
      "transform.prepare_material_next_round_request",
    "workflow.material-exploration-snapshot-json" =>
      "transform.build_material_exploration_snapshot",
    "workflow.material-pareto-frontier-json" => "transform.extract_material_pareto_frontier"
  }

  test "catalog exposes material optimization workflow templates" do
    workflow_ids =
      WorkflowTemplateCatalog.list()
      |> Enum.map(& &1["id"])
      |> MapSet.new()

    for workflow_id <- Map.keys(@material_workflows) do
      assert MapSet.member?(workflow_ids, workflow_id)
    end
  end

  test "material workflow templates resolve to transform export chains" do
    for {workflow_id, operator_id} <- @material_workflows do
      assert {:ok, graph} = WorkflowTemplateCatalog.graph_by_id(workflow_id)

      assert graph["dataset_contract"]["metadata"]["workflow_family"] =~ "material"
      assert Enum.any?(graph["nodes"], &(&1["operator_id"] == operator_id))
      assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "export.summary_json"))
      assert graph["output_nodes"] == ["json_output"]
      assert graph["dispatch_policy"] == "central_fetch"
    end
  end
end
