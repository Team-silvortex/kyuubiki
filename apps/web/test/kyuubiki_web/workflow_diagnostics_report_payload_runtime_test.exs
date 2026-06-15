defmodule KyuubikiWeb.WorkflowDiagnosticsReportPayloadRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes diagnostics report payload transform operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "transform.compose_diagnostics_report_payload")
  end

  test "composes bundle and guard into export-ready payload" do
    payload = %{
      "bundle" => %{
        "bundle_contract" => "kyuubiki.workflow_diagnostics_bundle/v1",
        "bundle_sources" => ["electrostatic", "thermal"],
        "bundle_items" => [%{"source" => "electrostatic"}]
      },
      "guard" => %{
        "guard_status" => "block",
        "guard_recommendation" => "hold_and_review"
      }
    }

    assert {:ok, report_payload} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_diagnostics_report_payload",
               payload,
               %{}
             )

    assert report_payload["report_contract"] == "kyuubiki.workflow_report_payload/v1"
    assert report_payload["report_kind"] == "diagnostics_bundle_report_payload"
    assert report_payload["report_sources"] == ["electrostatic", "thermal"]
    assert report_payload["report_guard_status"] == "block"
    assert report_payload["guard_payload"]["guard_recommendation"] == "hold_and_review"
    assert report_payload["bundle_contract"] == "kyuubiki.workflow_diagnostics_bundle/v1"
  end
end
