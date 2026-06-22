defmodule KyuubikiWeb.WorkflowElectrostaticTransformRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes electrostatic guard and benchmark transform operators" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "transform.evaluate_electrostatic_guard")
    assert MapSet.member?(operators, "transform.benchmark_electrostatic_pair")
  end

  test "evaluates electrostatic guard thresholds into pass warn block states" do
    payload = %{
      "electrostatic_potential_max" => 220.0,
      "electrostatic_field_peak_magnitude" => 14.0,
      "electrostatic_energy_density_peak" => 3.5
    }

    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_electrostatic_guard",
               payload,
               %{
                 "rules" => [
                   %{
                     "field" => "electrostatic_field_peak_magnitude",
                     "threshold" => 10.0,
                     "severity" => "warn",
                     "label" => "field ceiling"
                   },
                   %{
                     "field" => "electrostatic_potential_max",
                     "threshold" => 200.0,
                     "comparison" => "gt",
                     "severity" => "block",
                     "label" => "potential ceiling"
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

    assert Enum.any?(
             guard["guard_triggers"],
             &(&1["field"] == "electrostatic_field_peak_magnitude")
           )

    assert Enum.any?(guard["guard_triggers"], &(&1["severity"] == "block"))
  end

  test "benchmarks electrostatic pairs with weighted criteria" do
    payload = %{
      "left" => %{
        "electrostatic_potential_span" => 55.0,
        "electrostatic_field_peak_magnitude" => 9.0,
        "electrostatic_charge_density_sum" => 6.0
      },
      "right" => %{
        "electrostatic_potential_span" => 52.0,
        "electrostatic_field_peak_magnitude" => 12.0,
        "electrostatic_charge_density_sum" => 6.0
      }
    }

    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_electrostatic_pair",
               payload,
               %{
                 "left_label" => "quad",
                 "right_label" => "triangle",
                 "criteria" => [
                   %{
                     "field" => "potential_span",
                     "left_field" => "electrostatic_potential_span",
                     "right_field" => "electrostatic_potential_span",
                     "goal" => "min",
                     "weight" => 2.0
                   },
                   %{
                     "field" => "field_peak",
                     "left_field" => "electrostatic_field_peak_magnitude",
                     "right_field" => "electrostatic_field_peak_magnitude",
                     "goal" => "min",
                     "weight" => 1.0
                   },
                   %{
                     "field" => "charge_density_sum",
                     "left_field" => "electrostatic_charge_density_sum",
                     "right_field" => "electrostatic_charge_density_sum",
                     "goal" => "min",
                     "weight" => 3.0
                   }
                 ]
               }
             )

    assert benchmark["quad_score"] == 1.0
    assert benchmark["triangle_score"] == 2.0
    assert benchmark["benchmark_winner"] == "triangle"
    assert benchmark["benchmark_margin"] == 1.0
    assert benchmark["benchmark_criteria_count"] == 3
    assert benchmark["benchmark_left_win_count"] == 1
    assert benchmark["benchmark_right_win_count"] == 1
    assert benchmark["benchmark_tie_count"] == 1
    assert benchmark["benchmark_recommendation"] == "prefer_triangle"
    assert String.contains?(benchmark["benchmark_summary"], "triangle leads across 3 criteria")
    assert length(benchmark["benchmark_breakdown"]) == 3
  end
end
