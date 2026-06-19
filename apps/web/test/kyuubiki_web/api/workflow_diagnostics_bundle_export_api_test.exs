defmodule KyuubikiWeb.Api.WorkflowDiagnosticsBundleExportApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs diagnostics bundle guard export workflow through the graph api" do
    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => %{
            "schema_version" => "kyuubiki.workflow-graph/v1",
            "id" => "workflow.diagnostics-bundle-export",
            "name" => "Diagnostics bundle export",
            "version" => "1.0.0",
            "entry_nodes" => ["electrostatic_input", "thermal_input"],
            "output_nodes" => ["markdown_output"],
            "nodes" => [
              %{"id" => "electrostatic_input", "kind" => "input", "outputs" => [%{"id" => "summary", "artifact_type" => "artifact/json"}]},
              %{"id" => "thermal_input", "kind" => "input", "outputs" => [%{"id" => "summary", "artifact_type" => "artifact/json"}]},
              %{
                "id" => "bundle",
                "kind" => "transform",
                "operator_id" => "transform.compose_diagnostics_bundle",
                "config" => %{},
                "inputs" => [%{"id" => "electrostatic", "artifact_type" => "artifact/json"}, %{"id" => "thermal", "artifact_type" => "artifact/json"}],
                "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "guard",
                "kind" => "transform",
                "operator_id" => "transform.evaluate_diagnostics_bundle_guard",
                "config" => %{
                  "rules" => [
                    %{"source" => "thermal", "field" => "thermal_temperature_max", "threshold" => 75.0, "severity" => "warn", "label" => "thermal temperature"},
                    %{"source" => "electrostatic", "field" => "electrostatic_field_peak_magnitude", "comparison" => "gt", "threshold" => 9.0, "severity" => "block", "label" => "field ceiling"}
                  ]
                },
                "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json"}],
                "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "report",
                "kind" => "transform",
                "operator_id" => "transform.compose_diagnostics_report_payload",
                "config" => %{},
                "inputs" => [
                  %{"id" => "bundle", "artifact_type" => "artifact/json"},
                  %{"id" => "guard", "artifact_type" => "artifact/json"}
                ],
                "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "export",
                "kind" => "export",
                "operator_id" => "export.diagnostics_bundle_markdown",
                "config" => %{"title" => "Diagnostics Bundle Report"},
                "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json"}],
                "outputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown"}]
              },
              %{"id" => "markdown_output", "kind" => "output", "inputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown"}], "outputs" => []}
            ],
            "edges" => [
              %{"id" => "e0", "from" => %{"node" => "electrostatic_input", "port" => "summary"}, "to" => %{"node" => "bundle", "port" => "electrostatic"}, "artifact_type" => "artifact/json"},
              %{"id" => "e1", "from" => %{"node" => "thermal_input", "port" => "summary"}, "to" => %{"node" => "bundle", "port" => "thermal"}, "artifact_type" => "artifact/json"},
              %{"id" => "e2", "from" => %{"node" => "bundle", "port" => "result"}, "to" => %{"node" => "guard", "port" => "bundle"}, "artifact_type" => "artifact/json"},
              %{"id" => "e3", "from" => %{"node" => "bundle", "port" => "result"}, "to" => %{"node" => "report", "port" => "bundle"}, "artifact_type" => "artifact/json"},
              %{"id" => "e4", "from" => %{"node" => "guard", "port" => "result"}, "to" => %{"node" => "report", "port" => "guard"}, "artifact_type" => "artifact/json"},
              %{"id" => "e5", "from" => %{"node" => "report", "port" => "result"}, "to" => %{"node" => "export", "port" => "bundle"}, "artifact_type" => "artifact/json"},
              %{"id" => "e6", "from" => %{"node" => "export", "port" => "markdown"}, "to" => %{"node" => "markdown_output", "port" => "markdown"}, "artifact_type" => "export/markdown"}
            ]
          },
          "input_artifacts" => %{
            "electrostatic_input" => %{"summary" => %{"diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1", "diagnostic_domain" => "electrostatic", "diagnostic_subject" => "electrostatic_result", "diagnostic_prefix" => "electrostatic", "diagnostic_node_count" => 3, "diagnostic_element_count" => 2, "diagnostic_metric_groups" => ["field"], "electrostatic_field_peak_magnitude" => 10.0}},
            "thermal_input" => %{"summary" => %{"diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1", "diagnostic_domain" => "thermal", "diagnostic_subject" => "thermal_result", "diagnostic_prefix" => "thermal", "diagnostic_node_count" => 4, "diagnostic_element_count" => 3, "diagnostic_metric_groups" => ["temperature"], "thermal_temperature_max" => 80.0}}
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    exported = get_in(payload, ["artifacts", "export.markdown"])

    assert payload["completed_nodes"] == ["electrostatic_input", "thermal_input", "bundle", "guard", "report", "export", "markdown_output"]
    assert exported["format"] == "markdown"
    assert String.contains?(exported["content"], "# Diagnostics Bundle Report")
    assert String.contains?(exported["content"], "## Key Highlights")
    assert String.contains?(exported["content"], "[attention] Thermal temperature peak: 80.0")
    assert String.contains?(exported["content"], "[attention] Electrostatic field peak: 10.0")
    assert String.contains?(exported["content"], "## Diagnostics Sources")
    assert String.contains?(exported["content"], "## Guard Decision")
    assert String.contains?(exported["content"], "hold_and_review")
  end
end
