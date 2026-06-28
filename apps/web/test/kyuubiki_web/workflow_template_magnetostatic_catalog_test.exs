defmodule KyuubikiWeb.WorkflowTemplateMagnetostaticCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "catalog exposes magnetostatic quad guard workflow" do
    workflow_ids = WorkflowTemplateCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(workflow_ids, "workflow.magnetostatic-plane-quad-guard-json")
    assert MapSet.member?(workflow_ids, "workflow.magnetostatic-plane-quad-benchmark-json")
  end

  test "can resolve magnetostatic quad guard workflow graph" do
    assert {:ok, %{"id" => "workflow.magnetostatic-plane-quad-guard-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.magnetostatic-plane-quad-guard-json")

    assert graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.magnetostatic_plane_quad_guard/v1"

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.magnetostatic_plane_quad_2d"))

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "extract.magnetostatic_result_diagnostics")
           )

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "transform.evaluate_magnetostatic_guard")
           )

    guard_node = Enum.find(graph["nodes"], &(&1["id"] == "evaluate_guard"))

    assert guard_node["outputs"] == [
             %{
               "id" => "result",
               "artifact_type" => "report/summary",
               "dataset_value" => "magnetostatic_guard"
             }
           ]
  end

  test "can resolve magnetostatic quad benchmark workflow graph" do
    assert {:ok, %{"id" => "workflow.magnetostatic-plane-quad-benchmark-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.magnetostatic-plane-quad-benchmark-json"
             )

    assert graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.magnetostatic_plane_quad_benchmark/v1"

    assert graph["entry_nodes"] == ["candidate_a_model", "candidate_b_model"]

    solve_count =
      Enum.count(graph["nodes"], &(&1["operator_id"] == "solve.magnetostatic_plane_quad_2d"))

    assert solve_count == 2

    assert Enum.count(
             graph["nodes"],
             &(&1["operator_id"] == "extract.magnetostatic_result_diagnostics")
           ) == 2

    benchmark_node = Enum.find(graph["nodes"], &(&1["id"] == "benchmark_candidates"))

    assert benchmark_node["operator_id"] == "transform.benchmark_magnetostatic_pair"

    assert Enum.map(benchmark_node["inputs"], & &1["dataset_value"]) == [
             "candidate_a_diagnostics",
             "candidate_b_diagnostics"
           ]

    assert benchmark_node["outputs"] == [
             %{
               "id" => "result",
               "artifact_type" => "report/summary",
               "dataset_value" => "benchmark_summary"
             }
           ]
  end
end
