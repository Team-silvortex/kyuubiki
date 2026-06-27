defmodule KyuubikiWeb.WorkflowMagnetostaticRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes magnetostatic diagnostics extract operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "extract.magnetostatic_result_diagnostics")
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
end
