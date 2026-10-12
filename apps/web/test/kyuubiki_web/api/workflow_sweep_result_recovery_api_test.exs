defmodule KyuubikiWeb.Api.WorkflowSweepResultRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.SweepResultRecovery, as: Fixture

  test "failed result collection retains history while fresh jobs publish corrected winners" do
    for {name, input, _node, overrides} <- Fixture.corruptions() do
      failed = submit(input, overrides)
      assert failed["job"]["status"] == "failed", name
      id = failed["job"]["job_id"]
      assert {:ok, retained} = AnalysisResultStore.get(id)
      assert retained["_workflow_recovery"]["state"] == "failed"
      refute retained["artifacts"]["score.result"]
      refute retained["artifacts"]["export.json"]

      success = submit(Fixture.valid(), %{})
      assert success["job"]["status"] == "completed"
      refute success["job"]["job_id"] == id
      report = Jason.decode!(success["result"]["artifacts"]["export.json"]["content"])
      assert report["best"]["case_id"] == "a"
      assert {:ok, old} = AnalysisResultStore.get(id)
      assert old["_workflow_recovery"]["state"] == "failed"
      refute old["artifacts"]["export.json"]
    end
  end

  test "actual batch assessments reach research selection through asynchronous service jobs" do
    {input, config} = Fixture.actual_batch()
    response = submit(input, config)
    assert response["job"]["status"] == "completed", inspect(response)
    report = Jason.decode!(response["result"]["artifacts"]["export.json"]["content"])
    Fixture.assert_actual(report)
  end

  test "a completed all-infeasible evaluation is diagnostic not a selected winner" do
    response =
      submit(Fixture.valid(), %{
        "score" => %{"objectives" => [%{"field" => "mass", "max_allowed" => 0}]}
      })

    assert response["job"]["status"] == "completed"
    report = Jason.decode!(response["result"]["artifacts"]["export.json"]["content"])
    assert report["best"] == nil
    assert report["selection_ready"] == false
    assert report["scored_count"] == 2
    assert report["feasible_count"] == 0
  end

  defp submit(input, config) do
    response =
      :post
      |> conn(
        "/api/v1/workflows/graph/jobs",
        Jason.encode!(%{
          "graph" => Fixture.graph("fail", config),
          "input_artifacts" => %{"input" => input}
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 202, response.resp_body
    WorkflowApi.wait_for_job(Jason.decode!(response.resp_body)["job"]["job_id"], @opts, 400)
  end
end
