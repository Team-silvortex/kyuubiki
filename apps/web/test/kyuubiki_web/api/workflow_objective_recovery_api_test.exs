defmodule KyuubikiWeb.Api.WorkflowObjectiveRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.TestSupport.QualityRecovery, as: Fixture

  for kind <- ~w(objective ranking round) do
    @kind kind
    test "#{kind}: failed asynchronous job stays failed after a corrected fresh submission" do
      failed = submit(@kind, Fixture.invalid(@kind))
      assert failed["job"]["status"] == "failed"
      id = failed["job"]["job_id"]
      assert {:ok, retained} = AnalysisResultStore.get(id)
      assert retained["_workflow_recovery"]["state"] == "failed"
      refute Map.has_key?(retained["artifacts"], "export.json")
      refute Map.has_key?(retained["artifacts"], "#{@kind}.result")

      corrected = submit(@kind, Fixture.valid(@kind))
      assert corrected["job"]["status"] == "completed", inspect(corrected)
      refute corrected["job"]["job_id"] == id
      report = report(corrected)

      if @kind == "objective",
        do: assert(report["composite_quality_ready"]),
        else: assert(report["action"] == "stop")

      assert {:ok, still_failed} = AnalysisResultStore.get(id)
      assert still_failed["_workflow_recovery"]["state"] == "failed"
      refute Map.has_key?(still_failed["artifacts"], "export.json")
    end
  end

  test "partial ranking is a completed job but not a completed research conclusion" do
    partial = submit("ranking", Fixture.partial())
    assert partial["job"]["status"] == "completed"
    assert report(partial)["action"] == "replan"
    refute report(partial)["source_ranking_complete"]
    corrected = submit("ranking", Fixture.repaired())
    assert corrected["job"]["status"] == "completed"
    assert report(corrected)["action"] == "stop"
    assert report(corrected)["source_ranking_complete"]
    assert {:ok, retained} = AnalysisResultStore.get(partial["job"]["job_id"])
    assert retained["artifacts"]["round.result"]["action"] == "replan"
  end

  defp report(result), do: Jason.decode!(result["result"]["artifacts"]["export.json"]["content"])

  defp submit(kind, input) do
    response =
      :post
      |> conn(
        "/api/v1/workflows/graph/jobs",
        Jason.encode!(%{"graph" => Fixture.graph(kind), "input_artifacts" => %{"input" => input}})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 202, response.resp_body
    WorkflowApi.wait_for_job(Jason.decode!(response.resp_body)["job"]["job_id"], @opts, 400)
  end
end
