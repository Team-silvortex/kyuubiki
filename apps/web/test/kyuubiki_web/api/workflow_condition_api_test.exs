defmodule KyuubikiWeb.Api.WorkflowConditionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs a condition branch workflow and skips the inactive path" do
    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => %{
            "schema_version" => "kyuubiki.workflow-graph/v1",
            "id" => "workflow.condition-branch",
            "name" => "Condition branch",
            "version" => "1.0.0",
            "entry_nodes" => ["summary_input"],
            "output_nodes" => ["true_output", "false_output"],
            "nodes" => [
              %{
                "id" => "summary_input",
                "kind" => "input",
                "outputs" => [%{"id" => "value", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "gate",
                "kind" => "condition",
                "config" => %{
                  "predicate" => %{
                    "path" => "summary.max_displacement",
                    "operator" => "gt",
                    "value" => 1.0
                  }
                },
                "inputs" => [%{"id" => "value", "artifact_type" => "artifact/json"}],
                "outputs" => [
                  %{"id" => "if_true", "artifact_type" => "artifact/json"},
                  %{"id" => "if_false", "artifact_type" => "artifact/json"}
                ]
              },
              %{
                "id" => "true_output",
                "kind" => "output",
                "inputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}],
                "outputs" => []
              },
              %{
                "id" => "false_output",
                "kind" => "output",
                "inputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}],
                "outputs" => []
              }
            ],
            "edges" => [
              %{
                "id" => "input-to-gate",
                "from" => %{"node" => "summary_input", "port" => "value"},
                "to" => %{"node" => "gate", "port" => "value"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "gate-to-true",
                "from" => %{"node" => "gate", "port" => "if_true"},
                "to" => %{"node" => "true_output", "port" => "result"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "gate-to-false",
                "from" => %{"node" => "gate", "port" => "if_false"},
                "to" => %{"node" => "false_output", "port" => "result"},
                "artifact_type" => "artifact/json"
              }
            ]
          },
          "input_artifacts" => %{
            "summary_input" => %{
              "summary" => %{"max_displacement" => 2.5, "max_stress" => 14.0}
            }
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["completed_nodes"] == ["summary_input", "gate", "true_output"]
    assert payload["skipped_nodes"] == ["false_output"]

    assert payload["branch_decisions"] == [
             %{
               "node_id" => "gate",
               "chosen_output" => "if_true",
               "predicate_result" => true
             }
           ]

    assert Enum.at(payload["node_runs"], 3)["status"] == "skipped"

    assert get_in(payload, ["artifacts", "true_output.result", "summary", "max_displacement"]) ==
             2.5

    refute Map.has_key?(payload["artifacts"], "gate.if_false")
    refute Map.has_key?(payload["artifacts"], "false_output.result")
  end

  test "runs a condition merge workflow through transform.first_available" do
    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => %{
            "schema_version" => "kyuubiki.workflow-graph/v1",
            "id" => "workflow.condition-merge",
            "name" => "Condition merge",
            "version" => "1.0.0",
            "entry_nodes" => ["summary_input"],
            "output_nodes" => ["merged_output"],
            "nodes" => [
              %{
                "id" => "summary_input",
                "kind" => "input",
                "outputs" => [%{"id" => "value", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "gate",
                "kind" => "condition",
                "config" => %{
                  "predicate" => %{
                    "path" => "summary.max_stress",
                    "operator" => "gt",
                    "value" => 10.0
                  }
                },
                "inputs" => [%{"id" => "value", "artifact_type" => "artifact/json"}],
                "outputs" => [
                  %{"id" => "if_true", "artifact_type" => "artifact/json"},
                  %{"id" => "if_false", "artifact_type" => "artifact/json"}
                ]
              },
              %{
                "id" => "join",
                "kind" => "transform",
                "operator_id" => "transform.first_available",
                "inputs" => [
                  %{"id" => "left", "artifact_type" => "artifact/json"},
                  %{"id" => "right", "artifact_type" => "artifact/json"}
                ],
                "outputs" => [%{"id" => "merged", "artifact_type" => "artifact/json"}]
              },
              %{
                "id" => "merged_output",
                "kind" => "output",
                "inputs" => [%{"id" => "result", "artifact_type" => "artifact/json"}],
                "outputs" => []
              }
            ],
            "edges" => [
              %{
                "id" => "input-to-gate",
                "from" => %{"node" => "summary_input", "port" => "value"},
                "to" => %{"node" => "gate", "port" => "value"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "gate-true-to-join",
                "from" => %{"node" => "gate", "port" => "if_true"},
                "to" => %{"node" => "join", "port" => "left"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "gate-false-to-join",
                "from" => %{"node" => "gate", "port" => "if_false"},
                "to" => %{"node" => "join", "port" => "right"},
                "artifact_type" => "artifact/json"
              },
              %{
                "id" => "join-to-output",
                "from" => %{"node" => "join", "port" => "merged"},
                "to" => %{"node" => "merged_output", "port" => "result"},
                "artifact_type" => "artifact/json"
              }
            ]
          },
          "input_artifacts" => %{
            "summary_input" => %{
              "summary" => %{"max_displacement" => 0.4, "max_stress" => 12.0}
            }
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["completed_nodes"] == ["summary_input", "gate", "join", "merged_output"]
    assert payload["skipped_nodes"] == []

    assert payload["branch_decisions"] == [
             %{
               "node_id" => "gate",
               "chosen_output" => "if_true",
               "predicate_result" => true
             }
           ]

    assert Enum.any?(payload["artifact_lineage"], fn entry ->
             entry["artifact_key"] == "join.merged" and
               entry["source_artifacts"] == ["gate.if_true"]
           end)

    assert payload["artifacts"]["join.merged"] == payload["artifacts"]["gate.if_true"]
    assert get_in(payload, ["artifacts", "merged_output.result", "summary", "max_stress"]) == 12.0
  end
end
