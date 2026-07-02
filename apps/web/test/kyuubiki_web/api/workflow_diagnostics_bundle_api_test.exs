defmodule KyuubikiWeb.Api.WorkflowDiagnosticsBundleApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs a diagnostics bundle then bundle guard workflow through the graph api" do
    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => %{
            "schema_version" => "kyuubiki.workflow-graph/v1",
            "id" => "workflow.diagnostics-bundle-guard",
            "name" => "Diagnostics bundle guard",
            "version" => "1.0.0",
            "entry_nodes" => ["electrostatic_input", "thermal_input"],
            "output_nodes" => ["guard_output"],
            "nodes" => [
              %{
                "id" => "electrostatic_input",
                "kind" => "input",
                "outputs" => [%{"id" => "summary", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "thermal_input",
                "kind" => "input",
                "outputs" => [%{"id" => "summary", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "bundle",
                "kind" => "transform",
                "operator_id" => "transform.compose_diagnostics_bundle",
                "config" => %{},
                "inputs" => [
                  %{"id" => "electrostatic", "artifact_type" => "artifact/json"},
                  %{"id" => "thermal", "artifact_type" => "artifact/json"}
                ],
                "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "guard",
                "kind" => "transform",
                "operator_id" => "transform.evaluate_diagnostics_bundle_guard",
                "config" => %{
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
                    }
                  ]
                },
                "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json"}],
                "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "guard_output",
                "kind" => "output",
                "inputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}],
                "outputs" => []
              }
            ],
            "edges" => [
              %{
                "id" => "electrostatic-to-bundle",
                "from" => %{"node" => "electrostatic_input", "port" => "summary"},
                "to" => %{"node" => "bundle", "port" => "electrostatic"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "thermal-to-bundle",
                "from" => %{"node" => "thermal_input", "port" => "summary"},
                "to" => %{"node" => "bundle", "port" => "thermal"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "bundle-to-guard",
                "from" => %{"node" => "bundle", "port" => "result"},
                "to" => %{"node" => "guard", "port" => "bundle"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "guard-to-output",
                "from" => %{"node" => "guard", "port" => "result"},
                "to" => %{"node" => "guard_output", "port" => "result"},
                "artifact_type" => "artifact/json"
              }
            ]
          },
          "input_artifacts" => %{
            "electrostatic_input" => %{
              "summary" => %{
                "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain" => "electrostatic",
                "diagnostic_subject" => "electrostatic_result",
                "diagnostic_prefix" => "electrostatic",
                "diagnostic_node_count" => 3,
                "diagnostic_element_count" => 2,
                "diagnostic_metric_groups" => ["field"],
                "electrostatic_field_peak_magnitude" => 10.0
              }
            },
            "thermal_input" => %{
              "summary" => %{
                "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain" => "thermal",
                "diagnostic_subject" => "thermal_result",
                "diagnostic_prefix" => "thermal",
                "diagnostic_node_count" => 4,
                "diagnostic_element_count" => 3,
                "diagnostic_metric_groups" => ["temperature"],
                "thermal_temperature_max" => 80.0
              }
            }
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["completed_nodes"] == [
             "electrostatic_input",
             "thermal_input",
             "bundle",
             "guard",
             "guard_output"
           ]

    assert get_in(payload, ["artifacts", "guard.result", "guard_contract"]) ==
             "kyuubiki.workflow_guard_result/v1"

    assert get_in(payload, ["artifacts", "guard.result", "guard_status"]) == "block"
    assert get_in(payload, ["artifacts", "guard.result", "guard_block_count"]) == 1
    assert get_in(payload, ["artifacts", "guard.result", "guard_warn_count"]) == 1

    assert get_in(payload, ["artifacts", "guard.result", "guard_recommendation"]) ==
             "hold_and_review"
  end
end
