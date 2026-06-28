defmodule KyuubikiWeb.WorkflowTemplateCfdCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "catalog exposes stokes flow summary workflow" do
    workflow_ids = WorkflowTemplateCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(workflow_ids, "workflow.stokes-flow-quad-summary-json")
    assert MapSet.member?(workflow_ids, "workflow.stokes-flow-quad-guard-json")
    assert MapSet.member?(workflow_ids, "workflow.stokes-flow-quad-benchmark-json")
  end

  test "can resolve stokes flow summary workflow graph" do
    assert {:ok, %{"id" => "workflow.stokes-flow-quad-summary-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.stokes-flow-quad-summary-json")

    assert graph["dataset_contract"]["id"] == "kyuubiki.dataset.stokes_flow_quad_summary/v1"
    assert graph["entry_nodes"] == ["stokes_flow_model"]

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.stokes_flow_quad_2d"))

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "extract.stokes_flow_result_diagnostics")
           )

    diagnostics_node = Enum.find(graph["nodes"], &(&1["id"] == "extract_cfd_diagnostics"))

    assert diagnostics_node["outputs"] == [
             %{
               "id" => "summary",
               "artifact_type" => "report/summary",
               "dataset_value" => "cfd_diagnostics"
             }
           ]
  end

  test "can resolve stokes flow guard workflow graph" do
    assert {:ok, %{"id" => "workflow.stokes-flow-quad-guard-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.stokes-flow-quad-guard-json")

    assert graph["dataset_contract"]["id"] == "kyuubiki.dataset.stokes_flow_quad_guard/v1"

    guard_node = Enum.find(graph["nodes"], &(&1["id"] == "evaluate_cfd_guard"))

    assert guard_node["operator_id"] == "transform.evaluate_cfd_guard"

    assert guard_node["outputs"] == [
             %{
               "id" => "result",
               "artifact_type" => "report/summary",
               "dataset_value" => "cfd_guard"
             }
           ]
  end

  test "can resolve stokes flow benchmark workflow graph" do
    assert {:ok, %{"id" => "workflow.stokes-flow-quad-benchmark-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.stokes-flow-quad-benchmark-json")

    assert graph["dataset_contract"]["id"] == "kyuubiki.dataset.stokes_flow_quad_benchmark/v1"
    assert graph["entry_nodes"] == ["candidate_a_model", "candidate_b_model"]

    assert Enum.count(graph["nodes"], &(&1["operator_id"] == "solve.stokes_flow_quad_2d")) == 2

    benchmark_node = Enum.find(graph["nodes"], &(&1["id"] == "benchmark_candidates"))

    assert benchmark_node["operator_id"] == "transform.benchmark_cfd_pair"

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
