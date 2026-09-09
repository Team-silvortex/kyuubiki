defmodule KyuubikiWeb.Api.WorkflowCatalogMaterialEnvelopeJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  @tag :tmp_dir
  test "submits a material study envelope catalog workflow as an asynchronous job", %{
    tmp_dir: tmp_dir
  } do
    on_exit(fn -> File.rm_rf!(tmp_dir) end)

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

    assert summary["bundle_domains"] == []
    assert summary["bundle_domain_counts"] == %{}

    job_id = result_payload["job"]["job_id"]
    assert {:ok, stored} = AnalysisResultStore.get(job_id)
    path = Path.join(tmp_dir, "material-results.json")
    KyuubikiWeb.Persistence.write_json!(path, %{job_id => stored})
    reloaded = KyuubikiWeb.Persistence.read_json(path, %{})
    assert reloaded[job_id] == stored |> Jason.encode!() |> Jason.decode!()
    refute File.exists?("#{path}.corrupt")
    refute File.exists?("#{path}.recovery.json")
    assert :ok = KyuubikiWeb.Persistence.write_json!(path, %{})
    assert KyuubikiWeb.Persistence.read_json("#{path}.previous", %{}) == reloaded
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
