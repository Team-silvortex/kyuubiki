defmodule KyuubikiWeb.Api.WorkflowCatalogApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "lists the built-in workflow catalog and fetches a descriptor" do
    list_conn =
      :get
      |> conn("/api/v1/workflows/catalog")
      |> Router.call(@opts)

    assert list_conn.status == 200
    workflows = Jason.decode!(list_conn.resp_body)["workflows"]
    assert length(workflows) >= 3

    workflow_ids =
      MapSet.new(Enum.map(workflows, fn workflow -> workflow["id"] end))

    assert MapSet.subset?(
             MapSet.new([
               "workflow.electrostatic-to-heat-quad-2d",
               "workflow.electrostatic-plane-quad-2d",
               "workflow.electrostatic-plane-quad-field-statistics-json",
               "workflow.electrostatic-preheat-guard-markdown",
               "workflow.electrostatic-preheat-guard-heat-json",
               "workflow.electrostatic-preheat-guard-heat-thermo-json",
               "workflow.diagnostics-bundle-guard-report-markdown",
               "workflow.electrostatic-heat-thermo-diagnostics-markdown",
               "workflow.electrostatic-triangle-preheat-guard-markdown",
               "workflow.electrostatic-triangle-preheat-guard-heat-json",
               "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
               "workflow.electrostatic-quad-triangle-compare-json",
               "workflow.electrostatic-heat-thermo-summary-json",
               "workflow.heat-to-thermo-quad-2d"
             ]),
             workflow_ids
           )

    electrostatic_heat_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-to-heat-quad-2d"
      end)

    electrostatic_heat_thermo_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-heat-thermo-summary-json"
      end)

    electrostatic_field_statistics_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-plane-quad-field-statistics-json"
      end)

    electrostatic_compare_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-quad-triangle-compare-json"
      end)

    electrostatic_guard_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-preheat-guard-markdown"
      end)

    electrostatic_guard_heat_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-preheat-guard-heat-json"
      end)

    electrostatic_guard_heat_thermo_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-preheat-guard-heat-thermo-json"
      end)

    diagnostics_bundle_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.diagnostics-bundle-guard-report-markdown"
      end)

    coupled_diagnostics_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-heat-thermo-diagnostics-markdown"
      end)

    electrostatic_workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.electrostatic-plane-quad-2d"
      end)

    workflow =
      Enum.find(workflows, fn workflow ->
        workflow["id"] == "workflow.heat-to-thermo-quad-2d"
      end)

    assert electrostatic_heat_workflow["name"] == "Electrostatic to heat quad"
    assert electrostatic_heat_workflow["version"] == "1.0.0"

    assert electrostatic_field_statistics_workflow["name"] ==
             "Electrostatic plane quad field statistics JSON"

    assert electrostatic_compare_workflow["name"] ==
             "Electrostatic quad triangle compare JSON"

    assert electrostatic_guard_workflow["name"] ==
             "Electrostatic pre-heat guard markdown"

    assert electrostatic_guard_heat_workflow["name"] ==
             "Electrostatic pre-heat guard -> heat JSON"

    assert electrostatic_guard_heat_thermo_workflow["name"] ==
             "Electrostatic pre-heat guard -> heat -> thermo JSON"

    assert diagnostics_bundle_workflow["name"] ==
             "Diagnostics bundle guard report markdown"

    assert coupled_diagnostics_workflow["name"] ==
             "Electrostatic heat thermo diagnostics markdown"

    assert electrostatic_heat_thermo_workflow["name"] ==
             "Electrostatic heat thermo quad summary JSON"

    assert electrostatic_heat_thermo_workflow["version"] == "1.0.0"
    assert diagnostics_bundle_workflow["version"] == "1.0.0"
    assert coupled_diagnostics_workflow["version"] == "1.0.0"
    assert electrostatic_workflow["name"] == "Electrostatic plane quad"
    assert electrostatic_workflow["version"] == "1.0.0"
    assert workflow["name"] == "Heat to thermo quad"
    assert workflow["version"] == "1.0.0"
    assert length(workflow["entry_inputs"]) == 1
    assert length(workflow["output_artifacts"]) == 1
    assert is_map(workflow["graph"])

    assert workflow["runtime_manifest"]["dispatch_policy"]["authority_mode"] ==
             "central_operator_library"

    assert Enum.any?(workflow["runtime_manifest"]["operator_fetch_plan"], fn entry ->
             entry["node_id"] == "solve_heat" and
               entry["operator_id"] == "solve.heat_plane_quad_2d" and
               entry["package_ref"] ==
                 "orchestra://operator-package/solve.heat_plane_quad_2d"
           end)

    fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.heat-to-thermo-quad-2d")
      |> Router.call(@opts)

    assert fetch_conn.status == 200
    fetched = Jason.decode!(fetch_conn.resp_body)["workflow"]
    assert fetched["id"] == "workflow.heat-to-thermo-quad-2d"
    assert fetched["graph"]["output_nodes"] == ["json_output"]
    assert fetched["graph"]["dispatch_policy"] == "central_fetch"
    assert "workflow_bridge_runtime" in fetched["graph"]["required_capabilities"]

    bridge_node =
      Enum.find(fetched["graph"]["nodes"], fn node ->
        node["id"] == "bridge_temperature"
      end)

    assert "bridge" in bridge_node["placement_tags"]
    assert "workflow_bridge_runtime" in bridge_node["required_capabilities"]

    electrostatic_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-plane-quad-2d")
      |> Router.call(@opts)

    assert electrostatic_fetch_conn.status == 200
    electrostatic_fetched = Jason.decode!(electrostatic_fetch_conn.resp_body)["workflow"]
    assert electrostatic_fetched["id"] == "workflow.electrostatic-plane-quad-2d"
    assert electrostatic_fetched["graph"]["output_nodes"] == ["json_output"]

    assert electrostatic_fetched["graph"]["dataset_contract"]["metadata"] == %{
             "workflow_family" => "electrostatic_plane_quad_summary"
           }

    electrostatic_heat_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-to-heat-quad-2d")
      |> Router.call(@opts)

    assert electrostatic_heat_fetch_conn.status == 200

    electrostatic_heat_fetched =
      Jason.decode!(electrostatic_heat_fetch_conn.resp_body)["workflow"]

    assert electrostatic_heat_fetched["id"] == "workflow.electrostatic-to-heat-quad-2d"
    assert electrostatic_heat_fetched["graph"]["output_nodes"] == ["json_output"]

    assert electrostatic_heat_fetched["graph"]["dataset_contract"]["metadata"] == %{
             "workflow_family" => "electrostatic_to_heat_quad"
           }

    electrostatic_heat_thermo_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-heat-thermo-summary-json")
      |> Router.call(@opts)

    assert electrostatic_heat_thermo_fetch_conn.status == 200

    electrostatic_heat_thermo_fetched =
      Jason.decode!(electrostatic_heat_thermo_fetch_conn.resp_body)["workflow"]

    assert electrostatic_heat_thermo_fetched["id"] ==
             "workflow.electrostatic-heat-thermo-summary-json"

    assert electrostatic_heat_thermo_fetched["graph"]["output_nodes"] == ["json_output"]

    assert electrostatic_heat_thermo_fetched["graph"]["dataset_contract"]["values"] |> length() >=
             8

    assert hd(electrostatic_heat_thermo_fetched["graph"]["nodes"])["outputs"] == [
             %{
               "id" => "model",
               "artifact_type" => "study_model/electrostatic_plane_quad_2d",
               "dataset_value" => "electrostatic_model"
             }
           ]

    electrostatic_heat_thermo_triangle_fetch_conn =
      :get
      |> conn(
        "/api/v1/workflows/catalog/workflow.electrostatic-heat-thermo-triangle-summary-json"
      )
      |> Router.call(@opts)

    assert electrostatic_heat_thermo_triangle_fetch_conn.status == 200

    electrostatic_heat_thermo_triangle_fetched =
      Jason.decode!(electrostatic_heat_thermo_triangle_fetch_conn.resp_body)["workflow"]

    assert electrostatic_heat_thermo_triangle_fetched["id"] ==
             "workflow.electrostatic-heat-thermo-triangle-summary-json"

    assert electrostatic_heat_thermo_triangle_fetched["graph"]["dataset_contract"]["metadata"] ==
             %{"workflow_family" => "electrostatic_heat_thermo_triangle"}

    coupled_diagnostics_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-heat-thermo-diagnostics-markdown")
      |> Router.call(@opts)

    assert coupled_diagnostics_fetch_conn.status == 200

    coupled_diagnostics_fetched =
      Jason.decode!(coupled_diagnostics_fetch_conn.resp_body)["workflow"]

    assert coupled_diagnostics_fetched["id"] ==
             "workflow.electrostatic-heat-thermo-diagnostics-markdown"

    assert coupled_diagnostics_fetched["graph"]["output_nodes"] == ["markdown_output"]

    assert coupled_diagnostics_fetched["graph"]["dataset_contract"]["metadata"] == %{
             "workflow_family" => "electrostatic_heat_thermo_diagnostics_markdown"
           }

    assert Enum.any?(
             coupled_diagnostics_fetched["graph"]["nodes"],
             &(&1["operator_id"] == "extract.electrostatic_result_diagnostics")
           )

    assert Enum.any?(
             coupled_diagnostics_fetched["graph"]["nodes"],
             &(&1["operator_id"] == "extract.thermal_result_diagnostics")
           )

    assert Enum.any?(
             coupled_diagnostics_fetched["graph"]["nodes"],
             &(&1["operator_id"] == "extract.thermo_result_diagnostics")
           )

    electrostatic_heat_triangle_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-to-heat-triangle-2d")
      |> Router.call(@opts)

    assert electrostatic_heat_triangle_fetch_conn.status == 200

    electrostatic_heat_triangle_fetched =
      Jason.decode!(electrostatic_heat_triangle_fetch_conn.resp_body)["workflow"]

    assert electrostatic_heat_triangle_fetched["id"] ==
             "workflow.electrostatic-to-heat-triangle-2d"

    assert electrostatic_heat_triangle_fetched["graph"]["dataset_contract"]["values"] |> length() >=
             6

    heat_to_thermo_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.heat-to-thermo-quad-2d")
      |> Router.call(@opts)

    assert heat_to_thermo_fetch_conn.status == 200

    heat_to_thermo_fetched =
      Jason.decode!(heat_to_thermo_fetch_conn.resp_body)["workflow"]

    assert heat_to_thermo_fetched["graph"]["dataset_contract"]["metadata"] == %{
             "workflow_family" => "heat_to_thermo_quad"
           }

    bridge_node =
      Enum.find(heat_to_thermo_fetched["graph"]["nodes"], fn node ->
        node["id"] == "bridge_temperature"
      end)

    assert bridge_node["outputs"] == [
             %{
               "id" => "thermo_model",
               "artifact_type" => "study_model/thermal_plane_quad_2d",
               "dataset_value" => "thermo_model"
             }
           ]

    comparison_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.heat-to-thermo-quad-comparison-json")
      |> Router.call(@opts)

    assert comparison_fetch_conn.status == 200

    comparison_fetched =
      Jason.decode!(comparison_fetch_conn.resp_body)["workflow"]

    assert comparison_fetched["id"] == "workflow.heat-to-thermo-quad-comparison-json"
    assert comparison_fetched["graph"]["dataset_contract"]["values"] |> length() >= 8

    compare_node =
      Enum.find(comparison_fetched["graph"]["nodes"], fn node ->
        node["id"] == "compare_summaries"
      end)

    assert compare_node["outputs"] == [
             %{
               "id" => "result",
               "artifact_type" => "report/summary",
               "dataset_value" => "comparison_summary"
             }
           ]

    electrostatic_statistics_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-plane-quad-field-statistics-json")
      |> Router.call(@opts)

    assert electrostatic_statistics_fetch_conn.status == 200

    electrostatic_statistics_fetched =
      Jason.decode!(electrostatic_statistics_fetch_conn.resp_body)["workflow"]

    assert electrostatic_statistics_fetched["id"] ==
             "workflow.electrostatic-plane-quad-field-statistics-json"

    electrostatic_compare_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-quad-triangle-compare-json")
      |> Router.call(@opts)

    assert electrostatic_compare_fetch_conn.status == 200

    electrostatic_compare_fetched =
      Jason.decode!(electrostatic_compare_fetch_conn.resp_body)["workflow"]

    assert electrostatic_compare_fetched["id"] ==
             "workflow.electrostatic-quad-triangle-compare-json"

    electrostatic_hotspot_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-plane-quad-hotspot-alert")
      |> Router.call(@opts)

    assert electrostatic_hotspot_fetch_conn.status == 200

    electrostatic_hotspot_fetched =
      Jason.decode!(electrostatic_hotspot_fetch_conn.resp_body)["workflow"]

    assert electrostatic_hotspot_fetched["graph"]["output_nodes"] == ["markdown_output"]
    assert electrostatic_hotspot_fetched["graph"]["dataset_contract"]["values"] |> length() >= 4

    electrostatic_guard_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-preheat-guard-markdown")
      |> Router.call(@opts)

    assert electrostatic_guard_fetch_conn.status == 200

    electrostatic_guard_fetched =
      Jason.decode!(electrostatic_guard_fetch_conn.resp_body)["workflow"]

    assert electrostatic_guard_fetched["id"] ==
             "workflow.electrostatic-preheat-guard-markdown"

    electrostatic_guard_heat_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-preheat-guard-heat-json")
      |> Router.call(@opts)

    assert electrostatic_guard_heat_fetch_conn.status == 200

    electrostatic_guard_heat_fetched =
      Jason.decode!(electrostatic_guard_heat_fetch_conn.resp_body)["workflow"]

    assert electrostatic_guard_heat_fetched["id"] ==
             "workflow.electrostatic-preheat-guard-heat-json"

    electrostatic_guard_heat_thermo_fetch_conn =
      :get
      |> conn("/api/v1/workflows/catalog/workflow.electrostatic-preheat-guard-heat-thermo-json")
      |> Router.call(@opts)

    assert electrostatic_guard_heat_thermo_fetch_conn.status == 200

    electrostatic_guard_heat_thermo_fetched =
      Jason.decode!(electrostatic_guard_heat_thermo_fetch_conn.resp_body)["workflow"]

    assert electrostatic_guard_heat_thermo_fetched["id"] ==
             "workflow.electrostatic-preheat-guard-heat-thermo-json"

    assert electrostatic_guard_heat_thermo_fetched["graph"]["dataset_contract"]["metadata"] == %{
             "workflow_family" => "electrostatic_guard_heat_thermo"
           }

    electrostatic_triangle_guard_heat_thermo_fetch_conn =
      :get
      |> conn(
        "/api/v1/workflows/catalog/workflow.electrostatic-triangle-preheat-guard-heat-thermo-json"
      )
      |> Router.call(@opts)

    assert electrostatic_triangle_guard_heat_thermo_fetch_conn.status == 200

    electrostatic_triangle_guard_heat_thermo_fetched =
      Jason.decode!(electrostatic_triangle_guard_heat_thermo_fetch_conn.resp_body)["workflow"]

    assert electrostatic_triangle_guard_heat_thermo_fetched["id"] ==
             "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json"
  end

  test "filters workflow catalog by query contract fields" do
    conn =
      :get
      |> conn(
        "/api/v1/workflows/catalog?q=thermo&domain=thermo_mechanical&capability=workflow_bridge&entry_artifact=study_model/heat_plane_quad_2d&operator_id=bridge.temperature_field_to_thermo_quad_2d"
      )
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    workflows = payload["workflows"]
    assert length(workflows) == 1
    assert hd(workflows)["id"] == "workflow.heat-to-thermo-quad-2d"
  end
end
