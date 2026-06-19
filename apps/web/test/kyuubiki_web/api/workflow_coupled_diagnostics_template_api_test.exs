defmodule KyuubikiWeb.Api.WorkflowCoupledDiagnosticsTemplateApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.TestSupport.WorkflowApi

  test "runs the coupled electrostatic heat thermo diagnostics template through the catalog job api" do
    {:ok, _pid} =
      WorkflowApi.start_fake_agent_sessions([
        [
          %{
            "ok" => true,
            "result" => %{
              "nodes" => [
                %{"id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 10.0, "charge_density" => 0.0},
                %{"id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 0.0, "charge_density" => 0.0},
                %{"id" => "e2", "x" => 1.0, "y" => 1.0, "potential" => 0.0, "charge_density" => 0.0},
                %{"id" => "e3", "x" => 0.0, "y" => 1.0, "potential" => 0.0, "charge_density" => 0.0}
              ],
              "elements" => [
                %{
                  "id" => "eq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "electric_field_x" => 10.0,
                  "electric_field_y" => 0.0,
                  "electric_field_magnitude" => 10.0,
                  "energy_density" => 4.0
                }
              ],
              "max_potential" => 10.0,
              "max_electric_field" => 10.0
            }
          }
        ],
        [
          %{
            "ok" => true,
            "result" => %{
              "max_temperature" => 125.0,
              "max_heat_flux" => 900.0,
              "nodes" => [
                %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 125.0, "heat_load" => 500.0},
                %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 80.0, "heat_load" => 500.0},
                %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 35.0, "heat_load" => 500.0},
                %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 500.0}
              ],
              "elements" => [
                %{
                  "id" => "hq0",
                  "temperature_gradient_x" => 20.0,
                  "temperature_gradient_y" => -40.0,
                  "heat_flux_x" => -450.0,
                  "heat_flux_y" => 900.0
                }
              ],
              "input" => %{
                "elements" => [
                  %{
                    "id" => "hq0",
                    "node_i" => 0,
                    "node_j" => 1,
                    "node_k" => 2,
                    "node_l" => 3,
                    "thickness" => 0.02,
                    "conductivity" => 45.0
                  }
                ]
              }
            }
          }
        ],
        [
          %{
            "ok" => true,
            "result" => %{
              "max_displacement" => 0.0025,
              "max_stress" => 220.0,
              "max_temperature_delta" => 125.0,
              "nodes" => [
                %{"id" => "t0", "ux" => 0.0, "uy" => 1.0, "displacement_magnitude" => 1.0, "temperature_delta" => 125.0},
                %{"id" => "t1", "ux" => 0.0, "uy" => 2.5, "displacement_magnitude" => 2.5, "temperature_delta" => 80.0},
                %{"id" => "t2", "ux" => 0.0, "uy" => 1.5, "displacement_magnitude" => 1.5, "temperature_delta" => 35.0},
                %{"id" => "t3", "ux" => 0.0, "uy" => 1.0, "displacement_magnitude" => 1.0, "temperature_delta" => 20.0}
              ],
              "elements" => [
                %{
                  "id" => "tq0",
                  "stress_x" => -220.0,
                  "thermal_strain_x" => 4.8e-4,
                  "mechanical_strain_x" => -3.3e-4,
                  "total_strain_x" => 1.5e-4
                }
              ]
            }
          }
        ]
      ])

    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.electrostatic-heat-thermo-diagnostics-markdown",
        WorkflowApi.electrostatic_plane_quad_input_artifacts()
      )

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] ==
             "workflow.electrostatic-heat-thermo-diagnostics-markdown"

    assert result_payload["result"]["completed_nodes"] == [
             "electrostatic_model",
             "solve_electrostatic",
             "bridge_field_to_heat",
             "solve_heat",
             "bridge_temperature",
             "solve_thermo",
             "extract_electrostatic_diagnostics",
             "extract_thermal_diagnostics",
             "extract_thermo_diagnostics",
             "bundle",
             "guard",
             "report",
             "export",
             "markdown_output"
           ]

    bundle = result_payload["result"]["artifacts"]["bundle.result"]
    guard = result_payload["result"]["artifacts"]["guard.result"]
    heat_model = result_payload["result"]["artifacts"]["bridge_field_to_heat.heat_model"]
    thermo_model = result_payload["result"]["artifacts"]["bridge_temperature.thermo_model"]
    report = result_payload["result"]["artifacts"]["report.result"]
    exported = result_payload["result"]["artifacts"]["markdown_output.markdown"]
    artifact_lineage = result_payload["result"]["artifact_lineage"]
    dataset_lineage = result_payload["result"]["dataset_lineage"]

    assert bundle["bundle_contract"] == "kyuubiki.workflow_diagnostics_bundle/v1"
    assert bundle["bundle_source_count"] == 3
    assert bundle["bundle_sources"] == ["electrostatic", "thermal", "thermo"]
    assert bundle["bundle_domains"] == ["electrostatic", "thermal", "thermo_mechanical"]
    assert guard["guard_status"] == "block"
    assert guard["guard_passed"] == false
    assert guard["guard_recommendation"] == "hold_and_review"
    assert Enum.map(heat_model["nodes"], & &1["heat_load"]) == [500.0, 500.0, 500.0, 500.0]
    assert Enum.map(thermo_model["nodes"], & &1["temperature_delta"]) == [125.0, 80.0, 35.0, 20.0]
    assert report["report_contract"] == "kyuubiki.workflow_report_payload/v1"
    assert report["report_kind"] == "diagnostics_bundle_report_payload"
    assert report["report_sources"] == ["electrostatic", "thermal", "thermo"]
    assert report["report_guard_status"] == "block"
    assert report["guard_payload"]["guard_recommendation"] == "hold_and_review"
    assert report["report_focus_metrics"]["thermal.temperature_max"] == 125.0
    assert report["report_focus_metrics"]["thermo.temperature_delta_max"] == 125.0
    assert report["report_focus_metrics"]["thermo.stress_peak"] == 220.0
    assert report["report_focus_metrics"]["thermo.thermal_strain_peak"] == 4.8e-4
    assert Enum.any?(report["report_highlights"], fn item ->
             item["id"] == "thermal.temperature_max" and item["attention"] == true
           end)
    assert Enum.any?(report["report_highlights"], fn item ->
             item["id"] == "thermo.stress_peak" and item["value"] == 220.0
           end)
    assert exported["format"] == "markdown"
    assert String.contains?(exported["content"], "# Electrostatic Heat Thermo Diagnostics")
    assert String.contains?(exported["content"], "## Diagnostics Sources")
    assert String.contains?(exported["content"], "## Guard Decision")
    assert String.contains?(exported["content"], "stress ceiling")

    assert Enum.any?(artifact_lineage, fn entry ->
             entry["artifact_key"] == "report.result" and
               entry["source_artifacts"] == ["bundle.result", "guard.result"]
           end)

    assert Enum.any?(artifact_lineage, fn entry ->
             entry["artifact_key"] == "bridge_temperature.thermo_model" and
               entry["source_artifacts"] == ["solve_heat.result"]
           end)

    assert Enum.any?(dataset_lineage, fn entry ->
             entry["artifact_key"] == "bridge_temperature.thermo_model" and
               entry["dataset_value"] == "thermo_model" and
               entry["source_datasets"] == ["heat_result"]
           end)

    assert Enum.any?(dataset_lineage, fn entry ->
             entry["artifact_key"] == "report.result" and
               entry["dataset_value"] == "report_payload" and
               Enum.sort(entry["source_datasets"]) == ["diagnostics_bundle", "guard_result"]
           end)
  end
end
