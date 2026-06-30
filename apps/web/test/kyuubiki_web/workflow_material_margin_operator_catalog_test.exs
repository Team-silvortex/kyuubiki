defmodule KyuubikiWeb.WorkflowMaterialMarginOperatorCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog

  test "catalog exposes material margin transform for optimization workflows" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.evaluate_material_margins")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_margin"
    assert operator["domain"] == "multi_domain"
    assert "material" in operator["capability_tags"]
    assert "failure_index" in operator["capability_tags"]
    assert operator["module"]["lane"] == "dataflow"
    assert operator["module"]["operator_scope"] == "dataflow"
  end

  test "catalog exposes material candidate ranking transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.rank_material_candidates")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_candidate_rank"
    assert "ranking" in operator["capability_tags"]
    assert "candidate_selection" in operator["capability_tags"]
  end

  test "catalog exposes material fatigue life transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.estimate_material_fatigue_life")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_fatigue_life"
    assert "fatigue" in operator["capability_tags"]
    assert "life_estimation" in operator["capability_tags"]
  end

  test "catalog exposes material thermal shock transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.evaluate_material_thermal_shock")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_thermal_shock"
    assert "thermal_shock" in operator["capability_tags"]
    assert "fracture_risk" in operator["capability_tags"]
  end

  test "catalog exposes material candidate scoring transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.score_material_candidates")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_candidate_score"
    assert "score" in operator["capability_tags"]
    assert "weighted_objective" in operator["capability_tags"]
  end

  test "catalog exposes material experiment planning transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.plan_material_experiments")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_experiment_plan"
    assert "experiment_plan" in operator["capability_tags"]
    assert "shortlist" in operator["capability_tags"]
  end

  test "catalog exposes material experiment result analysis transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.analyze_material_experiment_results")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_experiment_result_analysis"
    assert "experiment_result" in operator["capability_tags"]
    assert "validation" in operator["capability_tags"]
  end

  test "catalog exposes material iteration decision transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.decide_material_iteration")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_iteration_decision"
    assert "iteration_decision" in operator["capability_tags"]
    assert "automation" in operator["capability_tags"]
  end

  test "catalog exposes material next-round request transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.prepare_material_next_round_request")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_next_round_request"
    assert "next_round" in operator["capability_tags"]
    assert "orchestration" in operator["capability_tags"]
  end

  test "catalog exposes material exploration snapshot transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.build_material_exploration_snapshot")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_exploration_snapshot"
    assert "snapshot" in operator["capability_tags"]
    assert "state" in operator["capability_tags"]
  end

  test "catalog exposes material Pareto frontier transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.extract_material_pareto_frontier")

    assert operator["kind"] == "transform"
    assert operator["family"] == "material_pareto_frontier"
    assert "pareto" in operator["capability_tags"]
    assert "multi_objective" in operator["capability_tags"]
  end
end
