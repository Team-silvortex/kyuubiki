defmodule KyuubikiWeb.Api.WorkflowConditionContractApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowConditionContract, as: Fixture

  test "HTTP graph decisions cannot accept the last array entry through an invalid index" do
    for id <- ["array-invalid-index", "array-negative-index", "array-partial-index"] do
      {200, result} = post_graph("run", request(id))

      assert [%{"predicate_result" => false, "chosen_output" => "if_false"}] =
               result["branch_decisions"]

      assert result["artifacts"]["rejected.payload"] == [false, true]
      refute Map.has_key?(result["artifacts"], "accepted.payload")
    end

    {200, result} = post_graph("run", request("integer-above-f64-precision"))
    assert [%{"predicate_result" => true}] = result["branch_decisions"]
    assert result["artifacts"]["accepted.payload"] == 9_007_199_254_740_993
  end

  test "malformed conditions return bounded 422 responses instead of crashing the API" do
    for id <- [
          "predicate-null",
          "predicate-array",
          "config-scalar",
          "path-number",
          "operator-null"
        ] do
      {422, error} = post_graph("run", request(id))
      assert error["error"] =~ "gate"
      assert error["error"] =~ "condition config"
      refute Map.has_key?(error, "artifacts")
    end

    {200, clean} = post_graph("run", request("true"))
    assert clean["failed_nodes"] == []
    assert clean["artifacts"]["accepted.payload"] == true
  end

  test "async condition failure retains honest progress and compact failure receipts" do
    for mode <- ["full", "compact"] do
      request =
        request("predicate-array", "skip")
        |> Map.put("response_options", %{"response_mode" => mode})

      {202, submitted} = post_graph("jobs", request)
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      result = completed["result"]
      assert completed["job"]["status"] == "completed"
      assert completed["job"]["message"] =~ "1 failed node(s)"
      assert result["failed_nodes"] == ["gate"]
      assert Enum.sort(result["skipped_nodes"]) == ["accepted", "consumer", "rejected"]
      assert [%{"node_id" => "gate", "error_message" => reason}] = result["node_failures"]
      assert reason =~ "config.predicate"
      last = List.last(result["progress_events"])
      assert last["failed_nodes"] == 1
      assert last["completed_nodes"] == 2
      assert last["skipped_nodes"] == 3
      assert last["resolved_nodes"] == 6
      {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["node_failures"]

      if mode == "full" do
        assert result["branch_decisions"] == []
        assert Enum.sort(Map.keys(result["artifacts"])) == ["input.payload", "raw.payload"]
      else
        refute Map.has_key?(result, "artifacts")
        refute Map.has_key?(result, "node_runs")
      end
    end
  end

  test "fail-fast condition jobs persist failure and a corrected fresh job remains usable" do
    {202, submitted} = post_graph("jobs", request("predicate-array"))
    job_id = submitted["job"]["job_id"]
    failed = WorkflowApi.wait_for_job(job_id, @opts)
    assert failed["job"]["status"] == "failed"
    assert failed["job"]["message"] =~ "gate"
    assert failed["job"]["message"] =~ "config.predicate"
    assert failed["result"]["artifacts"] == %{}
    assert failed["result"]["recovery"]["state"] == "failed"
    {:ok, retained} = AnalysisResultStore.get(job_id)
    assert retained["artifacts"] == %{}

    {202, submitted} = post_graph("jobs", request("nested-array-path"))
    clean = WorkflowApi.wait_for_job(submitted["job"]["job_id"], @opts)
    assert clean["job"]["status"] == "completed"
    assert clean["result"]["failed_nodes"] == []
    assert Map.has_key?(clean["result"]["artifacts"], "accepted.payload")
    {:ok, still_failed} = AnalysisResultStore.get(job_id)
    assert still_failed == retained
  end

  defp request(id, policy \\ "fail") do
    Fixture.cases() |> Enum.find(&(&1["id"] == id)) |> Fixture.request(policy)
  end

  defp post_graph(action, request) do
    conn =
      :post
      |> conn("/api/v1/workflows/graph/#{action}", Jason.encode!(request))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {conn.status, Jason.decode!(conn.resp_body)}
  end
end
