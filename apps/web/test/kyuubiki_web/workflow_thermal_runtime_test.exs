defmodule KyuubikiWeb.WorkflowThermalRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowThermalRuntime

  test "extracts thermo diagnostics with payload-level max_stress fallback" do
    payload = %{
      "max_stress" => 220.0,
      "nodes" => [
        %{"id" => "n0", "ux" => 3.0, "uy" => 4.0, "displacement_magnitude" => 5.0, "temperature_delta" => 25.0},
        %{"id" => "n1", "ux" => 5.0, "uy" => 12.0, "displacement_magnitude" => 13.0, "temperature_delta" => 45.0}
      ],
      "elements" => [
        %{"id" => "e0", "stress_x" => 110.0},
        %{"id" => "e1", "stress_x" => 180.0}
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowThermalRuntime.extract_thermo_result_diagnostics(payload, %{})

    assert diagnostics["thermo_temperature_delta_min"] == 25.0
    assert diagnostics["thermo_temperature_delta_max"] == 45.0
    assert diagnostics["thermo_peak_displacement"] == 13.0
    assert diagnostics["thermo_displacement_peak_magnitude"] == 13.0
    assert diagnostics["thermo_peak_displacement_x"] == 5.0
    assert diagnostics["thermo_peak_displacement_y"] == 12.0
    assert diagnostics["thermo_peak_stress"] == 220.0
    assert diagnostics["thermo_stress_peak"] == 220.0
    assert diagnostics["thermo_peak_stress_id"] == "max_stress"
    assert diagnostics["thermo_stress_peak_element_id"] == "max_stress"
  end

  test "prefers explicit heat flux magnitude fields when available" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "n1", "temperature" => 30.0, "heat_load" => 4.0}
      ],
      "elements" => [
        %{"id" => "e0", "heat_flux_x" => 100.0, "heat_flux_y" => 0.0, "heat_flux_magnitude" => 5.0},
        %{"id" => "e1", "heat_flux_x" => 1.0, "heat_flux_y" => 1.0, "heat_flux_magnitude" => 9.0}
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowThermalRuntime.extract_thermal_result_diagnostics(payload, %{})

    assert diagnostics["thermal_peak_flux_magnitude"] == 9.0
    assert diagnostics["thermal_peak_flux_id"] == "e1"
    assert diagnostics["thermal_peak_flux_x"] == 1.0
    assert diagnostics["thermal_peak_flux_y"] == 1.0
  end

  test "falls back to signed stress component peaks when von mises is absent" do
    payload = %{
      "nodes" => [%{"id" => "n0", "temperature_delta" => 10.0}],
      "elements" => [%{"id" => "e0", "stress_x" => -80.0}, %{"id" => "e1", "stress_x" => -120.0}]
    }

    assert {:ok, diagnostics} =
             WorkflowThermalRuntime.extract_thermo_result_diagnostics(payload, %{})

    assert diagnostics["thermo_peak_stress"] == -120.0
    assert diagnostics["thermo_peak_stress_id"] == "e1"
  end

  test "extracts thermo strain peaks from component fields" do
    payload = %{
      "nodes" => [%{"id" => "n0", "temperature_delta" => 10.0}],
      "elements" => [
        %{"id" => "e0", "thermal_strain_x" => 2.0e-4, "mechanical_strain_x" => -1.0e-4, "total_strain_x" => 1.0e-4},
        %{"id" => "e1", "thermal_strain_x" => 4.5e-4, "mechanical_strain_x" => -3.0e-4, "total_strain_x" => 2.0e-4}
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowThermalRuntime.extract_thermo_result_diagnostics(payload, %{})

    assert diagnostics["thermo_peak_thermal_strain"] == 4.5e-4
    assert diagnostics["thermo_peak_thermal_strain_id"] == "e1"
    assert diagnostics["thermo_peak_mechanical_strain"] == -3.0e-4
    assert diagnostics["thermo_peak_total_strain"] == 2.0e-4
  end
end
