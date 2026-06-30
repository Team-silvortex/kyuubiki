defmodule KyuubikiWeb.WorkflowTemplateAdvancedMechanicsCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowTemplateCatalog

  @advanced_mechanics_workflows %{
    "workflow.truss-2d-summary-json" => "solve.truss_2d",
    "workflow.truss-3d-summary-json" => "solve.truss_3d",
    "workflow.beam-1d-summary-json" => "solve.beam_1d",
    "workflow.thermal-beam-1d-summary-json" => "solve.thermal_beam_1d",
    "workflow.spring-1d-summary-json" => "solve.spring_1d",
    "workflow.spring-2d-summary-json" => "solve.spring_2d",
    "workflow.spring-3d-summary-json" => "solve.spring_3d",
    "workflow.frame-2d-summary-json" => "solve.frame_2d",
    "workflow.frame-3d-summary-json" => "solve.frame_3d",
    "workflow.thermal-frame-2d-summary-json" => "solve.thermal_frame_2d",
    "workflow.thermal-frame-3d-summary-json" => "solve.thermal_frame_3d",
    "workflow.thermal-truss-3d-summary-json" => "solve.thermal_truss_3d",
    "workflow.modal-frame-2d-summary-json" => "solve.modal_frame_2d",
    "workflow.modal-frame-3d-summary-json" => "solve.modal_frame_3d",
    "workflow.nonlinear-spring-1d-summary-json" => "solve.nonlinear_spring_1d",
    "workflow.contact-gap-1d-summary-json" => "solve.contact_gap_1d"
  }

  test "catalog exposes advanced mechanics solver summary workflows" do
    workflow_ids =
      WorkflowTemplateCatalog.list()
      |> Enum.map(& &1["id"])
      |> MapSet.new()

    for workflow_id <- Map.keys(@advanced_mechanics_workflows) do
      assert MapSet.member?(workflow_ids, workflow_id)
    end
  end

  test "advanced mechanics templates resolve to runnable solve extract export chains" do
    for {workflow_id, operator_id} <- @advanced_mechanics_workflows do
      assert {:ok, graph} = WorkflowTemplateCatalog.graph_by_id(workflow_id)

      assert Enum.any?(graph["nodes"], &(&1["operator_id"] == operator_id))
      assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "extract.result_summary"))
      assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "export.summary_json"))
      assert graph["output_nodes"] == ["json_output"]
      assert graph["dispatch_policy"] == "central_fetch"
    end
  end
end
