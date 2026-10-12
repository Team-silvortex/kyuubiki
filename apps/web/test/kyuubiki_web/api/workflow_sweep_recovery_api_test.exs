defmodule KyuubikiWeb.Api.WorkflowSweepRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture

  test "fresh corrected sweep reaches public batch execution without changing failed history" do
    failed = submit(put_in(Fixture.request(), ["request_payload", "search_space", "broken"], nil))
    assert failed["job"]["status"] == "failed"
    id = failed["job"]["job_id"]
    assert {:ok, retained} = AnalysisResultStore.get(id)
    assert retained["_workflow_recovery"]["state"] == "failed"
    refute retained["artifacts"]["batch.result"]
    refute retained["artifacts"]["export.json"]

    complete = submit(Fixture.request())
    assert complete["job"]["status"] == "completed", inspect(complete)
    refute complete["job"]["job_id"] == id
    batch = Jason.decode!(complete["result"]["artifacts"]["export.json"]["content"])
    Fixture.assert_batch(batch)
    prepared = call_batch("prepare-batch", batch)
    assert prepared["verified_count"] == 2
    executed = call_batch("execute-batch", batch)
    assert executed["status"] == "executed"
    Fixture.assert_execution(executed)
    assert {:ok, old} = AnalysisResultStore.get(id)
    assert old["_workflow_recovery"]["state"] == "failed"
    refute old["artifacts"]["export.json"]
  end

  test "incomplete ranking produces no executable artifact through the API" do
    failed = submit(Map.put(Fixture.request(), "source_ranking_complete", false))
    assert failed["job"]["status"] == "failed"
    assert {:ok, retained} = AnalysisResultStore.get(failed["job"]["job_id"])
    refute retained["artifacts"]["batch.result"]
    refute retained["artifacts"]["export.json"]
  end

  test "direct case jobs preserve failed history and HTTP batch gates reject changed evidence" do
    graph = Fixture.batch_graph("fail")
    cases = Fixture.expanded_cases()
    invalid = put_in(cases, ["cases", Access.at(1), "metadata"], %{"sweep_enabled" => false})
    failed = submit(invalid, graph)
    assert failed["job"]["status"] == "failed"
    failed_id = failed["job"]["job_id"]

    repair = %{
      "coupled_readiness" => %{"coupled_readiness_state" => "block"},
      "optimization_hint" => %{"action" => "fix_coupled_readiness"}
    }

    corrected = put_in(cases, ["cases", Access.at(1), "metadata"], repair)
    complete = submit(corrected, graph)
    assert complete["job"]["status"] == "completed"
    refute complete["job"]["job_id"] == failed_id
    batch = Jason.decode!(complete["result"]["artifacts"]["export.json"]["content"])
    assert List.last(batch["tasks"])["metadata"] == repair

    changed = put_in(batch, ["tasks", Access.at(1), "metadata", "source_ranking_complete"], false)

    for action <- ~w(prepare-batch execute-batch) do
      response =
        :post
        |> conn("/api/v1/operator-tasks/#{action}", Jason.encode!(%{"batch" => changed}))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert response.status == 422
      error = Jason.decode!(response.resp_body)
      assert error["error_code"] == "operator_task_batch_quality_rejected"
      assert error["error"] =~ "payload.tasks[1].metadata"
    end

    assert call_batch("prepare-batch", batch)["verified_count"] == 2
    Fixture.assert_execution(call_batch("execute-batch", batch))
    assert {:ok, retained} = AnalysisResultStore.get(failed_id)
    assert retained["_workflow_recovery"]["state"] == "failed"
    refute retained["artifacts"]["batch.result"]
    refute retained["artifacts"]["export.json"]
  end

  defp submit(input, graph \\ Fixture.graph()) do
    response =
      :post
      |> conn(
        "/api/v1/workflows/graph/jobs",
        Jason.encode!(%{"graph" => graph, "input_artifacts" => %{"input" => input}})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 202, response.resp_body
    WorkflowApi.wait_for_job(Jason.decode!(response.resp_body)["job"]["job_id"], @opts, 400)
  end

  defp call_batch(action, batch) do
    response =
      :post
      |> conn("/api/v1/operator-tasks/#{action}", Jason.encode!(%{"batch" => batch}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 200, response.resp_body
    Jason.decode!(response.resp_body)
  end
end
