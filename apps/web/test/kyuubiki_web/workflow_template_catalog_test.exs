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
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-plane-quad-field-statistics-json")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-preheat-guard-markdown")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-preheat-guard-heat-json")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-preheat-guard-heat-thermo-json")
    assert MapSet.member?(workflow_ids, "workflow.diagnostics-bundle-guard-report-markdown")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-triangle-preheat-guard-markdown")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-triangle-preheat-guard-heat-json")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json")
    assert MapSet.member?(workflow_ids, "workflow.heat-plane-triangle-summary-json")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-to-heat-triangle-2d")
    assert MapSet.member?(workflow_ids, "workflow.electrostatic-quad-triangle-compare-json")

    assert MapSet.member?(
             workflow_ids,
             "workflow.electrostatic-heat-thermo-triangle-summary-json"
           )

    assert MapSet.member?(
             workflow_ids,
             "workflow.electrostatic-heat-thermo-summary-json"
           )

    assert MapSet.member?(workflow_ids, "workflow.electrostatic-plane-quad-hotspot-alert")
    assert MapSet.member?(workflow_ids, "workflow.heat-to-thermo-quad-comparison-json")
    assert MapSet.member?(workflow_ids, "workflow.heat-plane-quad-guard-json")
    assert MapSet.member?(workflow_ids, "workflow.heat-thermo-quad-benchmark-json")
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
    assert graph["dataset_contract"]["id"] == "kyuubiki.dataset.electrostatic_heat_thermo_triangle/v1"

    export_node =
      Enum.find(graph["nodes"], &(&1["id"] == "export_json"))

    assert export_node["inputs"] == [
             %{
               "id" => "summary",
               "artifact_type" => "report/summary",
               "dataset_value" => "thermo_summary"
             }
           ]
  end

  test "can resolve graphs for bridge workflows with dataset contracts" do
    assert {:ok, %{"id" => "workflow.electrostatic-to-heat-quad-2d"} = quad_bridge_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.electrostatic-to-heat-quad-2d")

    assert quad_bridge_graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.electrostatic_to_heat_quad/v1"

    quad_bridge_node =
      Enum.find(quad_bridge_graph["nodes"], &(&1["id"] == "bridge_field_to_heat"))

    assert quad_bridge_node["outputs"] == [
             %{
               "id" => "heat_model",
               "artifact_type" => "study_model/heat_plane_quad_2d",
               "dataset_value" => "heat_model"
             }
           ]

    assert {:ok, %{"id" => "workflow.electrostatic-to-heat-triangle-2d"} = triangle_bridge_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.electrostatic-to-heat-triangle-2d")

    assert triangle_bridge_graph["dataset_contract"]["metadata"] == %{
             "workflow_family" => "electrostatic_to_heat_triangle"
           }

    triangle_extract_node =
      Enum.find(triangle_bridge_graph["nodes"], &(&1["id"] == "extract_summary"))

    assert triangle_extract_node["inputs"] == [
             %{
               "id" => "result",
               "artifact_type" => "result/heat_plane_triangle_2d",
               "dataset_value" => "heat_result"
             }
           ]

    assert {:ok, %{"id" => "workflow.heat-to-thermo-quad-2d"} = heat_bridge_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.heat-to-thermo-quad-2d")

    assert heat_bridge_graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.heat_to_thermo_quad/v1"

    heat_export_node =
      Enum.find(heat_bridge_graph["nodes"], &(&1["id"] == "export_json"))

    assert heat_export_node["inputs"] == [
             %{
               "id" => "summary",
               "artifact_type" => "report/summary",
               "dataset_value" => "thermo_summary"
             }
           ]
  end

  test "can resolve graphs for quad coupled workflows" do
    assert {:ok, %{"id" => "workflow.electrostatic-heat-thermo-summary-json"} = graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-heat-thermo-summary-json"
             )

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.electrostatic_plane_quad_2d"))

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "bridge.electrostatic_field_to_heat_quad_2d")
           )

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.heat_plane_quad_2d"))

    assert Enum.any?(
             graph["nodes"],
             &(&1["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d")
           )

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "solve.thermal_plane_quad_2d"))
    assert graph["dataset_contract"]["id"] == "kyuubiki.dataset.electrostatic_heat_thermo_quad/v1"

    bridge_node =
      Enum.find(graph["nodes"], &(&1["id"] == "bridge_temperature"))

    assert bridge_node["inputs"] == [
             %{
               "id" => "heat_result",
               "artifact_type" => "result/heat_plane_quad_2d",
               "dataset_value" => "heat_result"
             }
           ]
  end

  test "can resolve graphs for hotspot alert and comparison workflows" do
    assert {:ok, %{"id" => "workflow.electrostatic-plane-quad-2d"} = electrostatic_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.electrostatic-plane-quad-2d")

    assert electrostatic_graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.electrostatic_plane_quad_summary/v1"

    summary_node =
      Enum.find(electrostatic_graph["nodes"], &(&1["id"] == "extract_summary"))

    assert summary_node["outputs"] == [
             %{
               "id" => "summary",
               "artifact_type" => "report/summary",
               "dataset_value" => "electrostatic_summary"
             }
           ]

    assert {:ok, %{"id" => "workflow.electrostatic-plane-quad-hotspot-alert"} = hotspot_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-plane-quad-hotspot-alert"
             )

    assert Enum.any?(hotspot_graph["nodes"], &(&1["operator_id"] == "extract.field_hotspots"))
    assert Enum.any?(hotspot_graph["nodes"], &(&1["operator_id"] == "export.alert_markdown"))
    assert hotspot_graph["dataset_contract"]["metadata"]["workflow_family"] ==
             "electrostatic_plane_quad_hotspot_alert"
    assert hotspot_graph["output_nodes"] == ["markdown_output"]

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

    assert compare_graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.heat_to_thermo_quad_comparison/v1"

    compare_node =
      Enum.find(compare_graph["nodes"], &(&1["id"] == "compare_summaries"))

    assert compare_node["inputs"] == [
             %{
               "id" => "left",
               "artifact_type" => "report/summary",
               "dataset_value" => "heat_summary"
             },
             %{
               "id" => "right",
               "artifact_type" => "report/summary",
               "dataset_value" => "thermo_summary"
             }
           ]
  end

  test "can resolve graphs for diagnostics bundle guard report workflows" do
    assert {:ok, %{"id" => "workflow.diagnostics-bundle-guard-report-markdown"} = graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.diagnostics-bundle-guard-report-markdown"
             )

    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "transform.compose_diagnostics_bundle"))
    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "transform.evaluate_diagnostics_bundle_guard"))
    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "transform.compose_diagnostics_report_payload"))
    assert Enum.any?(graph["nodes"], &(&1["operator_id"] == "export.diagnostics_bundle_markdown"))
    assert graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.diagnostics_bundle_guard_report/v1"
    assert graph["dataset_contract"]["metadata"] == %{
             "workflow_family" => "diagnostics_bundle_guard_report"
           }

    report_node = Enum.find(graph["nodes"], &(&1["id"] == "report"))

    assert report_node["inputs"] == [
             %{
               "id" => "bundle",
               "artifact_type" => "artifact/json",
               "dataset_value" => "diagnostics_bundle"
             },
             %{
               "id" => "guard",
               "artifact_type" => "artifact/json",
               "dataset_value" => "guard_result"
             }
           ]

    assert {:ok, %{"id" => "workflow.electrostatic-heat-thermo-diagnostics-markdown"} = coupled_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-heat-thermo-diagnostics-markdown"
             )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "solve.electrostatic_plane_quad_2d")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "bridge.electrostatic_field_to_heat_quad_2d")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "solve.heat_plane_quad_2d")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "solve.thermal_plane_quad_2d")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "extract.electrostatic_result_diagnostics")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "extract.thermal_result_diagnostics")
           )

    assert Enum.any?(
             coupled_graph["nodes"],
             &(&1["operator_id"] == "extract.thermo_result_diagnostics")
           )

    assert coupled_graph["dataset_contract"]["id"] ==
             "kyuubiki.dataset.electrostatic_heat_thermo_diagnostics_markdown/v1"

    assert coupled_graph["dataset_contract"]["metadata"] == %{
             "workflow_family" => "electrostatic_heat_thermo_diagnostics_markdown"
           }

    assert coupled_graph["output_nodes"] == ["markdown_output"]

    export_node = Enum.find(coupled_graph["nodes"], &(&1["id"] == "export"))

    assert export_node["inputs"] == [
             %{
               "id" => "bundle",
               "artifact_type" => "artifact/json",
               "dataset_value" => "report_payload"
             }
           ]
  end

  test "can resolve graphs for electromagnetic statistics and comparison workflows" do
    assert {:ok, %{"id" => "workflow.electrostatic-plane-quad-field-statistics-json"} = stats_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-plane-quad-field-statistics-json"
             )

    assert Enum.any?(stats_graph["nodes"], &(&1["operator_id"] == "solve.electrostatic_plane_quad_2d"))
    assert Enum.any?(stats_graph["nodes"], &(&1["operator_id"] == "extract.field_statistics"))

    assert {:ok, %{"id" => "workflow.electrostatic-quad-triangle-compare-json"} = compare_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-quad-triangle-compare-json"
             )

    assert Enum.any?(compare_graph["nodes"], &(&1["operator_id"] == "solve.electrostatic_plane_quad_2d"))
    assert Enum.any?(compare_graph["nodes"], &(&1["operator_id"] == "solve.electrostatic_plane_triangle_2d"))
    assert Enum.any?(compare_graph["nodes"], &(&1["operator_id"] == "transform.normalize_summary_fields"))
    assert Enum.any?(compare_graph["nodes"], &(&1["operator_id"] == "transform.compare_summary_pair"))
  end

  test "can resolve graphs for thermal guard and benchmark workflows" do
    assert {:ok, %{"id" => "workflow.heat-plane-quad-guard-json"} = guard_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.heat-plane-quad-guard-json")

    assert Enum.any?(guard_graph["nodes"], &(&1["operator_id"] == "solve.heat_plane_quad_2d"))
    assert Enum.any?(guard_graph["nodes"], &(&1["operator_id"] == "extract.thermal_result_diagnostics"))
    assert Enum.any?(guard_graph["nodes"], &(&1["operator_id"] == "transform.evaluate_thermal_guard"))
    assert guard_graph["dataset_contract"]["id"] == "kyuubiki.dataset.heat_plane_quad_guard/v1"

    guard_node = Enum.find(guard_graph["nodes"], &(&1["id"] == "evaluate_guard"))

    assert guard_node["outputs"] == [
             %{
               "id" => "result",
               "artifact_type" => "report/summary",
               "dataset_value" => "thermal_guard"
             }
           ]

    assert {:ok, %{"id" => "workflow.heat-thermo-quad-benchmark-json"} = benchmark_graph} =
             WorkflowTemplateCatalog.graph_by_id("workflow.heat-thermo-quad-benchmark-json")

    assert Enum.any?(benchmark_graph["nodes"], &(&1["operator_id"] == "solve.heat_plane_quad_2d"))
    assert Enum.any?(benchmark_graph["nodes"], &(&1["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d"))
    assert Enum.any?(benchmark_graph["nodes"], &(&1["operator_id"] == "solve.thermal_plane_quad_2d"))
    assert Enum.any?(benchmark_graph["nodes"], &(&1["operator_id"] == "extract.thermal_result_diagnostics"))
    assert Enum.any?(benchmark_graph["nodes"], &(&1["operator_id"] == "extract.thermo_result_diagnostics"))
    assert Enum.any?(benchmark_graph["nodes"], &(&1["operator_id"] == "transform.benchmark_coupled_heat_pair"))
    assert benchmark_graph["dataset_contract"]["metadata"]["workflow_family"] == "heat_thermo_quad_benchmark"

    benchmark_node = Enum.find(benchmark_graph["nodes"], &(&1["id"] == "benchmark_coupled"))

    assert Enum.map(benchmark_node["inputs"], & &1["dataset_value"]) ==
             ["thermal_diagnostics", "thermo_diagnostics"]
  end

  test "can resolve graphs for electromagnetic guard workflows" do
    assert {:ok, %{"id" => "workflow.electrostatic-preheat-guard-markdown"} = guard_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-preheat-guard-markdown"
             )

    assert Enum.any?(guard_graph["nodes"], &(&1["kind"] == "condition"))
    assert Enum.any?(guard_graph["nodes"], &(&1["operator_id"] == "extract.field_hotspots"))
    assert Enum.any?(guard_graph["nodes"], &(&1["operator_id"] == "transform.first_available"))

    assert {:ok, %{"id" => "workflow.electrostatic-triangle-preheat-guard-markdown"} = triangle_guard_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-triangle-preheat-guard-markdown"
             )

    assert Enum.any?(
             triangle_guard_graph["nodes"],
             &(&1["operator_id"] == "solve.electrostatic_plane_triangle_2d")
           )

    assert {:ok, %{"id" => "workflow.electrostatic-preheat-guard-heat-json"} = guard_heat_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-preheat-guard-heat-json"
             )

    assert Enum.any?(guard_heat_graph["nodes"], &(&1["kind"] == "condition"))
    assert Enum.any?(
             guard_heat_graph["nodes"],
             &(&1["operator_id"] == "bridge.electrostatic_field_to_heat_quad_2d")
           )
    assert Enum.any?(guard_heat_graph["nodes"], &(&1["operator_id"] == "solve.heat_plane_quad_2d"))

    assert {:ok, %{"id" => "workflow.electrostatic-preheat-guard-heat-thermo-json"} = guard_heat_thermo_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-preheat-guard-heat-thermo-json"
             )

    assert Enum.any?(guard_heat_thermo_graph["nodes"], &(&1["kind"] == "condition"))

    assert Enum.any?(
             guard_heat_thermo_graph["nodes"],
             &(&1["operator_id"] == "bridge.electrostatic_field_to_heat_quad_2d")
           )

    assert Enum.any?(
             guard_heat_thermo_graph["nodes"],
             &(&1["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d")
           )

    assert Enum.any?(
             guard_heat_thermo_graph["nodes"],
             &(&1["operator_id"] == "solve.thermal_plane_quad_2d")
           )

    assert guard_heat_thermo_graph["dataset_contract"]["metadata"]["workflow_family"] ==
             "electrostatic_guard_heat_thermo"

    assert {:ok, %{"id" => "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json"} = triangle_guard_heat_thermo_graph} =
             WorkflowTemplateCatalog.graph_by_id(
               "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json"
             )

    assert Enum.any?(triangle_guard_heat_thermo_graph["nodes"], &(&1["kind"] == "condition"))

    assert Enum.any?(
             triangle_guard_heat_thermo_graph["nodes"],
             &(&1["operator_id"] == "bridge.electrostatic_field_to_heat_triangle_2d")
           )

    assert Enum.any?(
             triangle_guard_heat_thermo_graph["nodes"],
             &(&1["operator_id"] == "bridge.temperature_field_to_thermo_triangle_2d")
           )

    assert Enum.any?(
             triangle_guard_heat_thermo_graph["nodes"],
             &(&1["operator_id"] == "solve.thermal_plane_triangle_2d")
           )

    merge_node = Enum.find(triangle_guard_heat_thermo_graph["nodes"], &(&1["id"] == "merge_summary"))

    assert Enum.map(merge_node["inputs"], & &1["dataset_value"]) ==
             ["hotspot_summary", "thermo_summary"]
  end
end
