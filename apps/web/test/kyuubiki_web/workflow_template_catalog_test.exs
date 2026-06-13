defmodule KyuubikiWeb.WorkflowTemplateCatalogTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "includes solver-backed summary workflows in the catalog" do
    workflow_ids =
      WorkflowTemplateCatalog.list()
      |> Enum.map(& &1["id"])
      |> MapSet.new()

    assert MapSet.member?(workflow_ids, "workflow.bar-1d-summary-json")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-plane-triangle-summary-json")
    assert MapSet.member?(workflow_ids, "workflow.heat-plane-triangle-summary-json")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-to-heat-triangle-2d")

    assert MapSet.member?(
             workflow_ids,
             "workflow.electrostatic-heat-thermo-triangle-summary-json"
           )

    assert MapSet.member?(workflow_ids, "workflow.electrostatic-plane-quad-hotspot-alert")
    assert MapSet.member?(workflow_ids, "workflow.heat-to-thermo-quad-comparison-json")
    assert MapSet.member?(workflow_ids, "workflow.plane-quad-2d-summary-json")
  end

  test "can resolve graphs for solver-backed summary workflows" do
    assert {:ok, %{"id" => "workflow.thermal-truss-2d-summary-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.thermal-truss-2d-summary-json")

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.thermal_truss_2d"))
    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "extract.result_summary"))
    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "export.summary_json"))
  end

  test "can resolve graphs for triangle coupled workflows" do
    assert {:ok, %{"id" => "workflow.electrostatic-heat-thermo-triangle-summary-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-heat-thermo-triangle-summary-json"
             )

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "solve.electrostatic_plane_triangle_2d")
           )

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "bridge.electrostatic_field_to_heat_triangle_2d")
           )

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.heat_plane_triangle_2d"))

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "bridge.temperature_field_to_thermo_triangle_2d")
           )

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.thermal_plane_triangle_2d"))
  end

  test "can resolve graphs for hotspot alert and comparison workflows" do
    assert {:ok, %{"id" => "workflow.electrostatic-plane-quad-hotspot-alert"} = hotspot_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-plane-quad-hotspot-alert"
             )

    assert Enum.any?(hotspot_graph["nodes"], &(&1["operator_id"] == "extract.field_hotspots"))
    assert Enum.any?(hotspot_graph["nodes"], &(&1["operator_id"] == "export.alert_markdown"))

    assert {:ok, %{"id" => "workflow.heat-to-thermo-quad-comparison-json"} = compare_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.heat-to-thermo-quad-comparison-json")

    assert Enum.any?(compare_graph["nodes"], &(&1["operator_id"] == "solve.heat_plane_quad_2d"))

    assert Enum.any?(
             compare_graph["nodes"],
             &(&1["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d")
           )

    assert Enum.any?(
             compare_graph["nodes"],
             &(&1["operator_id"] == "solve.thermal_plane_quad_2d")
           )

    assert Enum.any?(
             compare_graph["nodes"],
             &(&1["operator_id"] == "transform.compare_summary_pair")
           )
  end
end
