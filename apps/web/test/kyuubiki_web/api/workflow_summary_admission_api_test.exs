defmodule KyuubikiWeb.Api.WorkflowSummaryAdmissionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowSummaryAdmission, as: Fixture

  test "summary failure is rejected synchronously and retained without assessment on recovery" do
    for sample <- Fixture.cases(),
        marker <- [%{"converged" => false}, %{"stability_result" => nil}],
        mode <- ["full", "compact"] do
      request = Fixture.request(sample, marker)
      {422, error} = post_graph("run", request)
      assert error["error"] =~ "assess"
      assert error["error"] =~ sample["operator_id"]

      request =
        Fixture.request(sample, marker, "left", true)
        |> Map.put("response_options", %{"response_mode" => mode})

      {202, submitted} = post_graph("jobs", request)
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "completed"
      assert completed["job"]["message"] =~ "1 failed node(s)"
      result = completed["result"]
      assert result["failed_nodes"] == ["assess"]
      assert result["skipped_nodes"] == ["result"]
      assert [%{"node_id" => "assess", "error_message" => reason}] = result["node_failures"]
      assert reason =~ sample["operator_id"]
      assert {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["node_failures"]

      if mode == "full" do
        refute Map.has_key?(result["artifacts"], "assess.payload")
        refute Map.has_key?(result["artifacts"], "result.payload")
        assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["left"]
        assert retained["artifacts"] == result["artifacts"]
      else
        refute Map.has_key?(result, "artifacts")
      end

      {200, fresh} = post_graph("run", Fixture.request(sample, %{}))
      assert fresh["failed_nodes"] == []

      assert Map.take(fresh["artifacts"]["result.payload"], Map.keys(sample["expected"])) ==
               sample["expected"]

      assert {:ok, ^retained} = AnalysisResultStore.get(job_id)
    end
  end

  test "fail-fast async summaries finish failed rather than publish an assessment" do
    for sample <- Fixture.cases() do
      {202, submitted} = post_graph("jobs", Fixture.request(sample, %{"converged" => false}))
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "failed"
      assert completed["job"]["message"] =~ "assess"
      assert completed["job"]["message"] =~ sample["operator_id"]
      assert completed["result"]["artifacts"] == %{}
      assert completed["result"]["recovery"]["state"] == "failed"
      assert {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["artifacts"] == %{}
    end
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
