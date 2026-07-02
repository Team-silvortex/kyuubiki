defmodule KyuubikiWeb.WorkflowOperatorSummaryRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "compares summary pairs through the transform runtime" do
    payload = %{
      "left" => %{"max_stress" => 10.0, "max_temperature" => 40.0},
      "right" => %{"max_stress" => 13.0, "max_temperature" => 44.0}
    }

    assert {:ok, compared} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compare_summary_pair",
               payload,
               %{"left_prefix" => "mechanical", "right_prefix" => "thermal"}
             )

    assert compared["mechanical_max_stress"] == 10.0
    assert compared["thermal_max_stress"] == 13.0
    assert compared["delta_max_stress"] == 3.0
    assert compared["ratio_max_stress"] == 1.3
    assert compared["summary_shared_numeric_field_count"] == 2
  end

  test "extracts field statistics and hotspots from numeric result collections" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "temperature" => 20.0},
        %{"id" => "n1", "temperature" => 50.0},
        %{"id" => "n2", "temperature" => 80.0}
      ],
      "elements" => [
        %{"id" => "e0", "electric_field_magnitude" => 2.0},
        %{"id" => "e1", "electric_field_magnitude" => 5.0},
        %{"id" => "e2", "electric_field_magnitude" => 9.0}
      ]
    }

    assert {:ok, stats} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_statistics",
               payload,
               %{"source" => "nodes", "field" => "temperature", "percentiles" => [50]}
             )

    assert stats["temperature_min"] == 20.0
    assert stats["temperature_max"] == 80.0
    assert stats["temperature_p50"] == 50.0

    assert {:ok, hotspots} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_hotspots",
               payload,
               %{"field" => "electric_field_magnitude", "threshold" => 5.0}
             )

    assert hotspots["electric_field_magnitude_hotspot_count"] == 2
    assert hotspots["electric_field_magnitude_hotspot_ids"] == ["e2", "e1"]
  end

  test "extracts dedicated thermal diagnostics from a heat result" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "n1", "temperature" => 50.0, "heat_load" => 10.0},
        %{"id" => "n2", "temperature" => 80.0, "heat_load" => -5.0}
      ],
      "elements" => [
        %{
          "id" => "e0",
          "temperature_gradient_x" => 3.0,
          "temperature_gradient_y" => 4.0,
          "heat_flux_x" => -6.0,
          "heat_flux_y" => 8.0
        },
        %{
          "id" => "e1",
          "temperature_gradient_x" => 0.0,
          "temperature_gradient_y" => 12.0,
          "heat_flux_x" => 5.0,
          "heat_flux_y" => 12.0
        }
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.thermal_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["thermal_temperature_min"] == 20.0
    assert diagnostics["diagnostic_contract"] == "kyuubiki.workflow_diagnostics/v1"
    assert diagnostics["diagnostic_domain"] == "thermal"
    assert diagnostics["diagnostic_subject"] == "thermal_result"
    assert diagnostics["diagnostic_prefix"] == "thermal"
    assert diagnostics["diagnostic_node_count"] == 3
    assert diagnostics["diagnostic_element_count"] == 2

    assert diagnostics["diagnostic_metric_groups"] == [
             "temperature",
             "heat_load",
             "gradient",
             "flux"
           ]

    assert diagnostics["thermal_temperature_max"] == 80.0
    assert diagnostics["thermal_temperature_span"] == 60.0
    assert diagnostics["thermal_heat_load_count"] == 3
    assert diagnostics["thermal_total_heat_load"] == 5.0
    assert diagnostics["thermal_heat_load_mean"] == 5.0 / 3.0
    assert diagnostics["thermal_loaded_node_count"] == 2
    assert diagnostics["thermal_peak_gradient_magnitude"] == 12.0
    assert diagnostics["thermal_peak_gradient_id"] == "e1"
    assert diagnostics["thermal_peak_gradient_element_id"] == "e1"
    assert diagnostics["thermal_peak_gradient_x"] == 0.0
    assert diagnostics["thermal_peak_gradient_y"] == 12.0
    assert diagnostics["thermal_peak_flux_magnitude"] == 13.0
    assert diagnostics["thermal_peak_flux_id"] == "e1"
    assert diagnostics["thermal_peak_flux_element_id"] == "e1"
    assert diagnostics["thermal_peak_flux_x"] == 5.0
    assert diagnostics["thermal_peak_flux_y"] == 12.0
  end

  test "extracts dedicated thermo-mechanical diagnostics from a structural thermal result" do
    payload = %{
      "nodes" => [
        %{
          "id" => "n0",
          "temperature_delta" => 0.0,
          "displacement_x" => 0.0,
          "displacement_y" => 0.0
        },
        %{
          "id" => "n1",
          "temperature_delta" => 20.0,
          "displacement_x" => 3.0,
          "displacement_y" => 4.0
        },
        %{
          "id" => "n2",
          "temperature_delta" => 35.0,
          "displacement_x" => 6.0,
          "displacement_y" => 8.0
        }
      ],
      "elements" => [
        %{
          "id" => "e0",
          "von_mises_stress" => 120.0,
          "thermal_strain_x" => 2.5e-4,
          "mechanical_strain_x" => -1.5e-4,
          "total_strain_x" => 1.0e-4
        },
        %{
          "id" => "e1",
          "von_mises_stress" => 180.0,
          "thermal_strain_x" => 4.8e-4,
          "mechanical_strain_x" => -3.3e-4,
          "total_strain_x" => 1.5e-4
        }
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.thermo_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["thermo_temperature_delta_max"] == 35.0
    assert diagnostics["diagnostic_contract"] == "kyuubiki.workflow_diagnostics/v1"
    assert diagnostics["diagnostic_domain"] == "thermo_mechanical"
    assert diagnostics["diagnostic_subject"] == "thermo_result"
    assert diagnostics["diagnostic_prefix"] == "thermo"
    assert diagnostics["diagnostic_node_count"] == 3
    assert diagnostics["diagnostic_element_count"] == 2

    assert diagnostics["diagnostic_metric_groups"] == [
             "temperature_delta",
             "displacement",
             "stress"
           ]

    assert diagnostics["thermo_temperature_delta_span"] == 35.0
    assert diagnostics["thermo_heated_node_count"] == 2
    assert diagnostics["thermo_peak_displacement"] == 10.0
    assert diagnostics["thermo_peak_displacement_id"] == "n2"
    assert diagnostics["thermo_peak_displacement_element_id"] == "n2"
    assert diagnostics["thermo_peak_displacement_x"] == 6.0
    assert diagnostics["thermo_peak_displacement_y"] == 8.0
    assert diagnostics["thermo_peak_stress"] == 180.0
    assert diagnostics["thermo_peak_stress_id"] == "e1"
    assert diagnostics["thermo_peak_stress_element_id"] == "e1"
    assert diagnostics["thermo_peak_thermal_strain"] == 4.8e-4
    assert diagnostics["thermo_peak_thermal_strain_id"] == "e1"
    assert diagnostics["thermo_peak_mechanical_strain"] == -3.3e-4
    assert diagnostics["thermo_peak_mechanical_strain_id"] == "e1"
    assert diagnostics["thermo_peak_total_strain"] == 1.5e-4
    assert diagnostics["thermo_peak_total_strain_id"] == "e1"
  end

  test "evaluates thermal guard thresholds into pass warn block states" do
    payload = %{
      "thermal_temperature_max" => 120.0,
      "thermal_peak_flux_magnitude" => 14.0,
      "thermo_peak_stress" => 210.0
    }

    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_thermal_guard",
               payload,
               %{
                 "rules" => [
                   %{
                     "field" => "thermal_temperature_max",
                     "threshold" => 100.0,
                     "severity" => "warn",
                     "label" => "temperature ceiling"
                   },
                   %{
                     "field" => "thermo_peak_stress",
                     "threshold" => 180.0,
                     "comparison" => "gt",
                     "severity" => "block",
                     "label" => "stress ceiling"
                   }
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert guard["guard_passed"] == false
    assert guard["guard_trigger_count"] == 2
    assert guard["guard_warn_count"] == 1
    assert guard["guard_block_count"] == 1
    assert guard["guard_recommendation"] == "hold_and_review"
    assert String.starts_with?(guard["guard_summary"], "BLOCK:")
    assert Enum.any?(guard["guard_triggers"], &(&1["field"] == "thermal_temperature_max"))
    assert Enum.any?(guard["guard_triggers"], &(&1["severity"] == "block"))
  end

  test "benchmarks coupled heat pairs with weighted criteria" do
    payload = %{
      "left" => %{
        "thermal_temperature_max" => 80.0,
        "thermal_peak_flux_magnitude" => 10.0,
        "thermal_loaded_node_count" => 3.0
      },
      "right" => %{
        "thermo_temperature_delta_max" => 75.0,
        "thermo_peak_stress" => 140.0,
        "thermo_heated_node_count" => 2.0
      }
    }

    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_coupled_heat_pair",
               payload,
               %{
                 "left_label" => "baseline",
                 "right_label" => "candidate",
                 "criteria" => [
                   %{
                     "field" => "temperature_vs_delta",
                     "left_field" => "thermal_temperature_max",
                     "right_field" => "thermo_temperature_delta_max",
                     "goal" => "min",
                     "weight" => 2.0
                   },
                   %{
                     "field" => "loaded_vs_heated_nodes",
                     "left_field" => "thermal_loaded_node_count",
                     "right_field" => "thermo_heated_node_count",
                     "goal" => "min",
                     "weight" => 1.0
                   },
                   %{
                     "field" => "flux_vs_stress",
                     "left_field" => "thermal_peak_flux_magnitude",
                     "right_field" => "thermo_peak_stress",
                     "goal" => "min",
                     "weight" => 3.0
                   }
                 ]
               }
             )

    assert benchmark["baseline_score"] == 3.0
    assert benchmark["candidate_score"] == 3.0
    assert benchmark["benchmark_winner"] == "tie"
    assert benchmark["benchmark_margin"] == 0.0
    assert benchmark["benchmark_criteria_count"] == 3
    assert benchmark["benchmark_left_win_count"] == 1
    assert benchmark["benchmark_right_win_count"] == 2
    assert benchmark["benchmark_tie_count"] == 0
    assert benchmark["benchmark_recommendation"] == "keep_both_under_review"
    assert String.contains?(benchmark["benchmark_summary"], "tie across 3 criteria")
    assert length(benchmark["benchmark_breakdown"]) == 3
  end

  test "exports hotspot summaries as markdown alerts" do
    payload = %{
      "field_hotspot_count" => 2,
      "field_hotspot_samples" => [
        %{"id" => "e2", "electric_field_magnitude" => 9.0},
        %{"id" => "e1", "electric_field_magnitude" => 5.0}
      ]
    }

    assert {:ok, exported} =
             WorkflowOperatorRuntime.run_export_operator(
               "export.alert_markdown",
               payload,
               %{"title" => "Electrostatic Hotspots", "severity" => "critical"}
             )

    assert exported["format"] == "markdown"
    assert String.contains?(exported["content"], "# Electrostatic Hotspots")
    assert String.contains?(exported["content"], "- Severity: critical")
    assert String.contains?(exported["content"], "## Sample Context")
  end
end
