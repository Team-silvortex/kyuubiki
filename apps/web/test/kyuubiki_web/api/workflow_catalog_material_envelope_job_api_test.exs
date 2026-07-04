defmodule KyuubikiWeb.Api.WorkflowCatalogMaterialEnvelopeJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "submits a material study envelope catalog workflow as an asynchronous job" do
    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.material-study-envelope-ranking-json",
        %{
          "material_rows" => %{
            "rows" => [
              %{
                "case_id" => "cool_stiff",
                "summaries" => %{
                  "thermal" => %{"max_temperature" => 90.0},
                  "structural" => %{"max_stress" => 180.0}
                }
              },
              %{
                "case_id" => "hot_light",
                "summaries" => %{
                  "thermal" => %{"max_temperature" => 130.0},
                  "structural" => %{"max_stress" => 120.0}
                }
              }
            ]
          }
        }
      )

    assert result_payload["job"]["status"] == "completed"

    assert result_payload["result"]["workflow_id"] ==
             "workflow.material-study-envelope-ranking-json"

    assert result_payload["result"]["dataset_contract"]["id"] ==
             "kyuubiki.dataset.material_study_envelope_ranking/v1"

    assert length(result_payload["result"]["completed_nodes"]) == 7

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"

    summary = Jason.decode!(exported["content"])
    assert summary["bundle_source_count"] == 2
    assert MapSet.new(summary["bundle_sources"]) == MapSet.new(["ranking", "pareto"])
    assert summary["bundle_payloads"]["ranking"]["material_best_candidate_id"] == "cool_stiff"

    assert summary["bundle_payloads"]["pareto"]["material_pareto_best_candidate_id"] ==
             "cool_stiff"
  end

  test "rejects oversized material envelope catalog requests" do
    conn =
      conn(
        :post,
        "/api/v1/workflows/catalog/workflow.material-study-envelope-ranking-json/jobs",
        Jason.encode!(%{
          "input_artifacts" => %{
            "material_rows" => %{
              "rows" =>
                Enum.map(1..129, fn index ->
                  %{
                    "case_id" => "candidate-#{index}",
                    "summaries" => %{"thermal" => %{"max_temperature" => 90.0}}
                  }
                end)
            }
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> KyuubikiWeb.Router.call(@opts)

    assert conn.status == 422
    assert Jason.decode!(conn.resp_body)["error"] == "invalid_material_envelope_catalog_request"
  end
end
