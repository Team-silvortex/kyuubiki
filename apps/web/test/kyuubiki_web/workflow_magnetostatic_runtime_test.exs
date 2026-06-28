defmodule KyuubikiWeb.WorkflowMagnetostaticRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes magnetostatic diagnostics extract operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "extract.magnetostatic_result_diagnostics")
    assert MapSet.member?(operators, "extract.magnetostatic_peak_field")
    assert MapSet.member?(operators, "transform.evaluate_magnetostatic_guard")
    assert MapSet.member?(operators, "transform.benchmark_magnetostatic_pair")
  end

  test "extracts magnetostatic diagnostics from vector potential and field results" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "vector_potential" => 0.0, "current_density" => 1.0},
        %{"id" => "n1", "vector_potential" => 2.0, "current_density" => 3.0},
        %{"id" => "n2", "vector_potential" => 5.0, "current_density" => 2.0}
      ],
      "elements" => [
        %{
          "id" => "m0",
          "magnetic_field_strength_x" => 3.0,
          "magnetic_field_strength_y" => 4.0,
          "magnetic_flux_density_x" => 6.0,
          "magnetic_flux_density_y" => 8.0,
          "energy_area_density" => 2.5
        },
        %{
          "id" => "m1",
          "magnetic_field_strength_x" => 5.0,
          "magnetic_field_strength_y" => 12.0,
          "magnetic_flux_density_x" => 8.0,
          "magnetic_flux_density_y" => 15.0,
          "energy_area_density" => 7.0
        }
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.magnetostatic_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["diagnostic_contract"] == "kyuubiki.workflow_diagnostics/v1"
    assert diagnostics["diagnostic_domain"] == "magnetostatic"
    assert diagnostics["diagnostic_subject"] == "magnetostatic_result"
    assert diagnostics["diagnostic_prefix"] == "magnetostatic"
    assert diagnostics["diagnostic_node_count"] == 3
    assert diagnostics["diagnostic_element_count"] == 2

    assert diagnostics["magnetostatic_vector_potential_min"] == 0.0
    assert diagnostics["magnetostatic_vector_potential_max"] == 5.0
    assert diagnostics["magnetostatic_vector_potential_span"] == 5.0
    assert diagnostics["magnetostatic_current_density_sum"] == 6.0
    assert diagnostics["magnetostatic_energy_density_peak"] == 7.0
    assert diagnostics["magnetostatic_energy_density_peak_element_id"] == "m1"
    assert diagnostics["magnetostatic_field_peak_magnitude"] == 13.0
    assert diagnostics["magnetostatic_field_peak_element_id"] == "m1"
    assert diagnostics["magnetostatic_flux_peak_magnitude"] == 17.0
    assert diagnostics["magnetostatic_flux_peak_element_id"] == "m1"
  end

  test "extracts peak magnetostatic field summary from quad results" do
    assert {:ok, summary} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.magnetostatic_peak_field",
               magnetostatic_quad_result(),
               %{}
             )

    assert summary["peak_element_id"] == "m1"
    assert summary["peak_magnetic_field_strength"] == 13.0
    assert summary["peak_flux_density"] == 17.0
    assert summary["peak_average_vector_potential"] == 4.0
    assert summary["peak_vector_potential_gradient_magnitude"] == 13.0
    assert summary["magnetostatic_field_peak_element_id"] == "m1"
    assert summary["magnetostatic_flux_peak_element_id"] == "m1"
    assert summary["magnetostatic_peak_stored_energy"] == 7.0
    assert summary["max_magnetic_field_strength"] == 13.0
    assert summary["total_stored_energy"] == 9.5
  end

  test "evaluates magnetostatic guard thresholds" do
    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_magnetostatic_guard",
               %{"magnetostatic_field_peak_magnitude" => 13.0, "total_stored_energy" => 9.5},
               %{
                 "rules" => [
                   %{
                     "field" => "magnetostatic_field_peak_magnitude",
                     "threshold" => 12.0,
                     "severity" => "block",
                     "label" => "H peak"
                   },
                   %{"field" => "total_stored_energy", "threshold" => 20.0, "severity" => "warn"}
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert guard["guard_block_count"] == 1
    assert guard["guard_trigger_count"] == 1
    assert hd(guard["guard_triggers"])["label"] == "H peak"
  end

  test "benchmarks magnetostatic summaries" do
    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_magnetostatic_pair",
               %{
                 "left" => %{
                   "magnetostatic_field_peak_magnitude" => 11.0,
                   "total_stored_energy" => 7.0
                 },
                 "right" => %{
                   "magnetostatic_field_peak_magnitude" => 13.0,
                   "total_stored_energy" => 9.5
                 }
               },
               %{
                 "left_label" => "candidate_a",
                 "right_label" => "candidate_b",
                 "criteria" => [
                   %{
                     "field" => "magnetostatic_field_peak_magnitude",
                     "goal" => "min",
                     "weight" => 2.0
                   },
                   %{"field" => "total_stored_energy", "goal" => "min", "weight" => 1.0}
                 ]
               }
             )

    assert benchmark["benchmark_winner"] == "candidate_a"
    assert benchmark["candidate_a_score"] == 3.0
    assert benchmark["candidate_b_score"] == 0.0
    assert benchmark["benchmark_criteria_count"] == 2
  end

  defp magnetostatic_quad_result do
    %{
      "nodes" => [],
      "elements" => [
        %{
          "id" => "m0",
          "average_vector_potential" => 2.0,
          "vector_potential_gradient_x" => 3.0,
          "vector_potential_gradient_y" => 4.0,
          "magnetic_field_strength_x" => 3.0,
          "magnetic_field_strength_y" => 4.0,
          "magnetic_field_strength_magnitude" => 5.0,
          "magnetic_flux_density_x" => 6.0,
          "magnetic_flux_density_y" => 8.0,
          "magnetic_flux_density_magnitude" => 10.0,
          "stored_energy" => 2.5
        },
        %{
          "id" => "m1",
          "average_vector_potential" => 4.0,
          "vector_potential_gradient_x" => 5.0,
          "vector_potential_gradient_y" => 12.0,
          "magnetic_field_strength_x" => 5.0,
          "magnetic_field_strength_y" => 12.0,
          "magnetic_field_strength_magnitude" => 13.0,
          "magnetic_flux_density_x" => 8.0,
          "magnetic_flux_density_y" => 15.0,
          "magnetic_flux_density_magnitude" => 17.0,
          "stored_energy" => 7.0
        }
      ],
      "max_vector_potential" => 5.0,
      "max_magnetic_field_strength" => 13.0,
      "max_flux_density" => 17.0,
      "total_stored_energy" => 9.5
    }
  end
end
