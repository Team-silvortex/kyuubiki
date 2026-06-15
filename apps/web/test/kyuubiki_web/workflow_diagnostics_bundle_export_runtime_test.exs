defmodule KyuubikiWeb.WorkflowDiagnosticsBundleExportRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes diagnostics bundle markdown export operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "export.diagnostics_bundle_markdown")
  end

  test "exports diagnostics bundle markdown with guard details" do
    payload = %{
      "bundle_contract" => "kyuubiki.workflow_diagnostics_bundle/v1",
      "bundle_source_count" => 2,
      "bundle_domains" => ["electrostatic", "thermal"],
      "bundle_subjects" => ["electrostatic_result", "thermal_result"],
      "bundle_total_node_count" => 7,
      "bundle_total_element_count" => 5,
      "bundle_metric_groups" => ["field", "temperature"],
      "bundle_items" => [
        %{
          "source" => "electrostatic",
          "domain" => "electrostatic",
          "subject" => "electrostatic_result",
          "prefix" => "electrostatic",
          "node_count" => 3,
          "element_count" => 2,
          "metric_groups" => ["field"]
        }
      ]
    }

    guard_payload = %{
      "guard_status" => "block",
      "guard_passed" => false,
      "guard_recommendation" => "hold_and_review",
      "guard_summary" => "BLOCK: 1 trigger(s) (electrostatic.field ceiling=10.0).",
      "guard_triggers" => [
        %{
          "source" => "electrostatic",
          "label" => "field ceiling",
          "value" => 10.0,
          "comparison" => "gt",
          "threshold" => 9.0,
          "severity" => "block"
        }
      ]
    }

    assert {:ok, exported} =
             WorkflowOperatorRuntime.run_export_operator(
               "export.diagnostics_bundle_markdown",
               payload,
               %{"title" => "Bundle Report", "guard_payload" => guard_payload}
             )

    assert exported["format"] == "markdown"
    assert String.contains?(exported["content"], "# Bundle Report")
    assert String.contains?(exported["content"], "## Diagnostics Sources")
    assert String.contains?(exported["content"], "## Guard Decision")
    assert String.contains?(exported["content"], "hold_and_review")
    assert String.contains?(exported["content"], "electrostatic.field ceiling")
  end
end
