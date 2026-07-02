defmodule KyuubikiWeb.WorkflowElectrostaticRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowElectrostaticRuntime
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes electrostatic diagnostics extract operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "extract.electrostatic_result_diagnostics")
  end

  test "extracts electrostatic diagnostics from field and potential results" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "potential" => 0.0, "charge_density" => 1.0},
        %{"id" => "n1", "potential" => 3.0, "charge_density" => 2.0},
        %{"id" => "n2", "potential" => 5.0, "charge_density" => 1.5}
      ],
      "elements" => [
        %{
          "id" => "e0",
          "electric_field_x" => 3.0,
          "electric_field_y" => 4.0,
          "energy_density" => 2.0
        },
        %{
          "id" => "e1",
          "electric_field_x" => 6.0,
          "electric_field_y" => 8.0,
          "energy_density" => 7.0
        }
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.electrostatic_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["electrostatic_node_count"] == 3
    assert diagnostics["electrostatic_element_count"] == 2
    assert diagnostics["diagnostic_contract"] == "kyuubiki.workflow_diagnostics/v1"
    assert diagnostics["diagnostic_domain"] == "electrostatic"
    assert diagnostics["diagnostic_subject"] == "electrostatic_result"
    assert diagnostics["diagnostic_prefix"] == "electrostatic"
    assert diagnostics["diagnostic_node_count"] == 3
    assert diagnostics["diagnostic_element_count"] == 2

    assert diagnostics["diagnostic_metric_groups"] == [
             "potential",
             "charge_density",
             "energy_density",
             "field"
           ]

    assert diagnostics["electrostatic_potential_min"] == 0.0
    assert diagnostics["electrostatic_potential_max"] == 5.0
    assert diagnostics["electrostatic_potential_mean"] == 8.0 / 3.0
    assert diagnostics["electrostatic_potential_span"] == 5.0
    assert diagnostics["electrostatic_charge_density_count"] == 3
    assert diagnostics["electrostatic_charge_density_sum"] == 4.5
    assert diagnostics["electrostatic_charge_density_mean"] == 1.5
    assert diagnostics["electrostatic_energy_density_peak"] == 7.0
    assert diagnostics["electrostatic_energy_density_peak_element_id"] == "e1"
    assert diagnostics["electrostatic_field_peak_magnitude"] == 10.0
    assert diagnostics["electrostatic_field_peak_element_id"] == "e1"
    assert diagnostics["electrostatic_field_peak_x"] == 6.0
    assert diagnostics["electrostatic_field_peak_y"] == 8.0
  end

  test "falls back to scalar field magnitude and custom prefixes" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "voltage" => 1.0},
        %{"id" => "n1", "voltage" => 4.0}
      ],
      "cells" => [
        %{"id" => "c0", "rho" => 0.25, "field_abs" => 12.0},
        %{"id" => "c1", "rho" => 0.75, "field_abs" => 18.0}
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowElectrostaticRuntime.extract_electrostatic_result_diagnostics(
               payload,
               %{
                 "node_source" => "nodes",
                 "element_source" => "cells",
                 "potential_field" => "voltage",
                 "charge_density_field" => "rho",
                 "charge_density_source" => "elements",
                 "field_magnitude_field" => "field_abs",
                 "field_x_field" => "missing_x",
                 "field_y_field" => "missing_y",
                 "output_prefix" => "es_diag"
               }
             )

    assert diagnostics["es_diag_potential_span"] == 3.0
    assert diagnostics["diagnostic_prefix"] == "es_diag"
    assert diagnostics["es_diag_charge_density_sum"] == 1.0
    assert diagnostics["es_diag_field_peak_magnitude"] == 18.0
    assert diagnostics["es_diag_field_peak_element_id"] == "c1"
    refute Map.has_key?(diagnostics, "es_diag_field_peak_x")
  end
end
