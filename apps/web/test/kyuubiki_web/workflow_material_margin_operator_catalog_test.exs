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
end
