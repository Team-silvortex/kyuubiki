defmodule KyuubikiWeb.WorkflowDiagnosticsBundleRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes diagnostics bundle transform operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "transform.compose_diagnostics_bundle")
  end

  test "composes multiple diagnostics payloads into a bundle" do
    payload = %{
      "electrostatic" => %{
        "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
        "diagnostic_domain" => "electrostatic",
        "diagnostic_subject" => "electrostatic_result",
        "diagnostic_prefix" => "electrostatic",
        "diagnostic_node_count" => 3,
        "diagnostic_element_count" => 2,
        "diagnostic_metric_groups" => ["potential", "field"],
        "electrostatic_potential_max" => 5.0
      },
      "thermal" => %{
        "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
        "diagnostic_domain" => "thermal",
        "diagnostic_subject" => "thermal_result",
        "diagnostic_prefix" => "thermal",
        "diagnostic_node_count" => 4,
        "diagnostic_element_count" => 3,
        "diagnostic_metric_groups" => ["temperature", "flux"],
        "thermal_temperature_max" => 80.0
      }
    }

    assert {:ok, bundle} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_diagnostics_bundle",
               payload,
               %{}
             )

    assert bundle["bundle_contract"] == "kyuubiki.workflow_diagnostics_bundle/v1"
    assert bundle["bundle_kind"] == "workflow_diagnostics_bundle"
    assert bundle["bundle_source_count"] == 2
    assert bundle["bundle_sources"] == ["electrostatic", "thermal"]
    assert bundle["bundle_domains"] == ["electrostatic", "thermal"]
    assert bundle["bundle_subjects"] == ["electrostatic_result", "thermal_result"]
    assert bundle["bundle_domain_counts"] == %{"electrostatic" => 1, "thermal" => 1}
    assert bundle["bundle_metric_groups"] == ["field", "flux", "potential", "temperature"]
    assert bundle["bundle_total_node_count"] == 7
    assert bundle["bundle_total_element_count"] == 5
    assert bundle["bundle_numeric_field_count"] == 4
    assert Map.has_key?(bundle["bundle_payloads"], "electrostatic")
    assert bundle["bundle_numeric_fields"] == [
             "diagnostic_element_count",
             "diagnostic_node_count",
             "electrostatic_potential_max",
             "thermal_temperature_max"
           ]
    assert Enum.any?(bundle["bundle_items"], &(&1["source"] == "thermal"))
  end

  test "can omit payload copies and numeric field index" do
    payload = %{
      "thermal" => %{
        "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
        "diagnostic_domain" => "thermal",
        "diagnostic_subject" => "thermal_result",
        "diagnostic_prefix" => "thermal",
        "diagnostic_node_count" => 2,
        "diagnostic_element_count" => 1,
        "diagnostic_metric_groups" => ["temperature"]
      }
    }

    assert {:ok, bundle} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_diagnostics_bundle",
               payload,
               %{"include_payloads" => false, "include_numeric_fields" => false}
             )

    refute Map.has_key?(bundle, "bundle_payloads")
    refute Map.has_key?(bundle, "bundle_numeric_fields")
  end
end
