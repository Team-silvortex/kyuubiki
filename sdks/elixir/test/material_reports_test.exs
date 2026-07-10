defmodule KyuubikiSdk.MaterialReportsTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.MaterialReports

  test "catalog exposes material studies" do
    assert {:ok, catalog} = MaterialReports.material_study_catalog()
    assert length(catalog) == 5
    assert {:ok, dielectric} = MaterialReports.describe_material_study("dielectric-screening")
    assert dielectric["id"] == "material_dielectric_screening"
    assert dielectric["domain"] == "electromagnetic"
    assert dielectric["metric_specs"] != []

    assert {:ok, composite} =
             MaterialReports.describe_material_study("composite-thermo-electric-panel")

    assert composite["domain"] == "multiphysics_materials"
  end

  test "extracts successful headless result fetch payloads" do
    assert {:ok, payloads} =
             MaterialReports.extract_material_result_payloads(%{
               "schema_version" => "kyuubiki.headless-execution-run/v1",
               "steps" => [
                 %{
                   "action" => "result_fetch",
                   "status" => "executed",
                   "result_preview" => %{"result" => %{"max_electric_field" => 42.0e6}}
                 },
                 %{
                   "action" => "result_fetch",
                   "status" => "failed",
                   "result_preview" => %{"result" => %{"max_electric_field" => 1.0}}
                 }
               ]
             })

    assert payloads == [%{"max_electric_field" => 42.0e6}]
  end

  test "builds dielectric report from alias" do
    assert {:ok, report} =
             MaterialReports.build_material_report("dielectric-screening", [
               %{"result" => %{"max_electric_field" => 42.0e6, "max_flux_density" => 1.2e-3}},
               %{"result" => %{"max_electric_field" => 38.0e6, "max_flux_density" => 3.3e-3}},
               %{"max_electric_field" => 48.0e6, "max_flux_density" => 0.9e-3}
             ])

    assert report["schema_version"] == "kyuubiki.dielectric-material-report/v1"
    assert report["winner_candidate_id"] == "polyimide_film"
    assert hd(report["candidates"])["rank"] == 1
    assert report["reliability"]["summary"]["decision"] == "ready_for_next_round"
    assert report["reliability"]["summary"]["violation_count"] == 0
  end

  test "builds report directly from payload wrapper" do
    assert {:ok, report} =
             MaterialReports.build_material_report_from_payload("structural-panel", %{
               "result_payloads" => [
                 %{"result" => %{"max_stress" => 240.0e6, "max_displacement" => 0.001}},
                 %{"result" => %{"max_stress" => 180.0e6, "max_displacement" => 0.0008}},
                 %{"result" => %{"max_stress" => 210.0e6, "max_displacement" => 0.0012}}
               ]
             })

    assert report["study"] == "material.structural_panel_screening.v1"
    assert is_binary(report["winner_candidate_id"])
  end

  test "reliability summary blocks missing required metrics" do
    assert {:ok, report} =
             MaterialReports.build_material_report("dielectric-screening", [
               %{"result" => %{"max_flux_density" => 1.2e-3}},
               %{"result" => %{"max_electric_field" => 38.0e6, "max_flux_density" => 3.3e-3}},
               %{"max_electric_field" => 48.0e6, "max_flux_density" => 0.9e-3}
             ])

    assert report["reliability"]["summary"]["decision"] == "blocked_by_quality_gates"
    assert report["reliability"]["summary"]["blocking_gate_ids"] == ["gate.result_completeness"]
    assert hd(report["reliability"]["quality_gates"])["status"] == "violate"
  end

  test "builds composite thermo electric report" do
    assert {:ok, report} =
             MaterialReports.build_material_report("composite-thermo-electric-panel", [
               %{
                 "electrostatic" => %{"max_electric_field" => 45.0e6},
                 "heat" => %{"max_temperature" => 120.0},
                 "thermal" => %{"max_stress" => 180.0e6}
               },
               %{
                 "electrostatic" => %{"max_electric_field" => 52.0e6},
                 "heat" => %{"max_temperature" => 98.0},
                 "thermal" => %{"max_stress" => 140.0e6}
               },
               %{
                 "electrostatic" => %{"max_electric_field" => 58.0e6},
                 "heat" => %{"max_temperature" => 132.0},
                 "thermal" => %{"max_stress" => 210.0e6}
               }
             ])

    assert report["schema_version"] == "kyuubiki.composite-panel-report/v1"
    assert report["study"] == "material.composite_thermo_electric_panel.v1"
    assert length(report["candidates"]) == 3
    assert is_binary(report["reliability"]["summary"]["decision"])
  end
end
