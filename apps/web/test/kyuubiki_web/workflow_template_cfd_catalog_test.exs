defmodule KyuubikiWeb.WorkflowTemplateCfdCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "catalog exposes stokes flow summary workflow" do
    workflow_ids = WorkflowTemplateCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(workflow_ids, "workflow.stokes-flow-quad-summary-json")
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
end
