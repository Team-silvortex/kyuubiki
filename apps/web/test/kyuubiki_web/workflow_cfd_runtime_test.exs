defmodule KyuubikiWeb.WorkflowCfdRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes stokes flow CFD operators" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "solve.stokes_flow_quad_2d")
    assert MapSet.member?(operators, "extract.stokes_flow_result_diagnostics")
    assert MapSet.member?(operators, "transform.evaluate_cfd_guard")
    assert MapSet.member?(operators, "transform.benchmark_cfd_pair")
  end

  test "extracts stokes flow diagnostics" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "velocity_magnitude" => 0.0, "pressure" => 1.0},
        %{"id" => "n1", "velocity_magnitude" => 2.0, "pressure" => -3.0},
        %{"id" => "n2", "velocity_magnitude" => 4.0, "pressure" => -1.0}
      ],
      "elements" => [
        %{
          "id" => "f0",
          "divergence_error" => 0.02,
          "reynolds_number" => 5.0,
          "viscous_dissipation" => 0.3
        },
        %{
          "id" => "f1",
          "divergence_error" => 0.08,
          "reynolds_number" => 12.0,
          "viscous_dissipation" => 0.9
        }
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.stokes_flow_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["diagnostic_contract"] == "kyuubiki.workflow_diagnostics/v1"
    assert diagnostics["diagnostic_domain"] == "fluid"
    assert diagnostics["diagnostic_subject"] == "stokes_flow_result"
    assert diagnostics["diagnostic_node_count"] == 3
    assert diagnostics["diagnostic_element_count"] == 2
    assert diagnostics["cfd_velocity_min"] == 0.0
    assert diagnostics["cfd_velocity_max"] == 4.0
    assert diagnostics["cfd_pressure_min"] == -3.0
    assert diagnostics["cfd_pressure_max"] == 1.0
    assert diagnostics["cfd_divergence_error_peak"] == 0.08
    assert diagnostics["cfd_divergence_error_peak_element_id"] == "f1"
    assert diagnostics["cfd_reynolds_number_peak"] == 12.0
    assert diagnostics["cfd_viscous_dissipation_total"] == 1.2
  end

  test "evaluates CFD guard thresholds" do
    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_cfd_guard",
               %{"cfd_divergence_error_peak" => 0.08, "cfd_reynolds_number_peak" => 12.0},
               %{
                 "rules" => [
                   %{
                     "field" => "cfd_divergence_error_peak",
                     "threshold" => 0.05,
                     "severity" => "block",
                     "label" => "divergence ceiling"
                   }
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert guard["guard_block_count"] == 1
    assert guard["guard_summary"] == "BLOCK: 1 CFD guard trigger(s)."
  end

  test "benchmarks CFD summaries" do
    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_cfd_pair",
               %{
                 "left" => %{
                   "cfd_divergence_error_peak" => 0.03,
                   "cfd_reynolds_number_peak" => 8.0
                 },
                 "right" => %{
                   "cfd_divergence_error_peak" => 0.08,
                   "cfd_reynolds_number_peak" => 12.0
                 }
               },
               %{
                 "left_label" => "candidate_a",
                 "right_label" => "candidate_b",
                 "criteria" => [
                   %{"field" => "cfd_divergence_error_peak", "goal" => "min", "weight" => 2.0},
                   %{"field" => "cfd_reynolds_number_peak", "goal" => "min", "weight" => 1.0}
                 ]
               }
             )

    assert benchmark["benchmark_winner"] == "candidate_a"
    assert benchmark["candidate_a_score"] == 3.0
    assert benchmark["candidate_b_score"] == 0.0
    assert benchmark["benchmark_criteria_count"] == 2
  end
end
