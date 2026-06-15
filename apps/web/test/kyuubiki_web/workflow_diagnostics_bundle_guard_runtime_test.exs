defmodule KyuubikiWeb.WorkflowDiagnosticsBundleGuardRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes diagnostics bundle guard transform operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "transform.evaluate_diagnostics_bundle_guard")
  end

  test "evaluates source-specific and bundle-level rules" do
    payload = %{
      "bundle_contract" => "kyuubiki.workflow_diagnostics_bundle/v1",
      "bundle_source_count" => 2,
      "bundle_total_node_count" => 7,
      "bundle_payloads" => %{
        "electrostatic" => %{"electrostatic_field_peak_magnitude" => 10.0},
        "thermal" => %{"thermal_temperature_max" => 80.0}
      }
    }

    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_diagnostics_bundle_guard",
               payload,
               %{
                 "rules" => [
                   %{
                     "source" => "thermal",
                     "field" => "thermal_temperature_max",
                     "threshold" => 75.0,
                     "severity" => "warn",
                     "label" => "thermal temperature"
                   },
                   %{
                     "source" => "electrostatic",
                     "field" => "electrostatic_field_peak_magnitude",
                     "comparison" => "gt",
                     "threshold" => 9.0,
                     "severity" => "block",
                     "label" => "field ceiling"
                   },
                   %{
                     "field" => "bundle_total_node_count",
                     "comparison" => "gte",
                     "threshold" => 7.0,
                     "severity" => "warn",
                     "label" => "bundle nodes"
                   }
                 ]
               }
             )

    assert guard["guard_contract"] == "kyuubiki.workflow_guard_result/v1"
    assert guard["guard_scope"] == "workflow_diagnostics_bundle"
    assert guard["guard_status"] == "block"
    assert guard["guard_passed"] == false
    assert guard["guard_trigger_count"] == 3
    assert guard["guard_warn_count"] == 2
    assert guard["guard_block_count"] == 1
    assert guard["guard_recommendation"] == "hold_and_review"
    assert String.starts_with?(guard["guard_summary"], "BLOCK:")
    assert Enum.any?(guard["guard_triggers"], &(&1["source"] == "thermal"))
    assert Enum.any?(guard["guard_triggers"], &(&1["source"] == "bundle"))
    assert Enum.any?(guard["guard_triggers"], &(&1["severity"] == "block"))
  end
end
