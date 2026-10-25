defmodule KyuubikiWeb.Api.WorkflowSummaryReductionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowSummaryAdmission, as: Fixture

  test "summary reduction errors retain failure evidence and leave fresh requests usable" do
    cases = Fixture.reduction_cases()

    for sample <- cases, Map.has_key?(sample, "error"), mode <- ["full", "compact"] do
      request = Fixture.request(sample, %{})
      {422, rejected} = post_graph("run", request)
      assert rejected["error"] =~ sample["operator_id"]
      assert rejected["error"] =~ sample["error"]

      recoverable =
        Fixture.request(sample, %{}, "left", true)
        |> Map.put("response_options", %{"response_mode" => mode})

      {202, submitted} = post_graph("jobs", recoverable)
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "completed"
      assert completed["job"]["message"] =~ "1 failed node(s)"
      result = completed["result"]
      assert result["failed_nodes"] == ["assess"]
      assert result["skipped_nodes"] == ["result"]
      assert [%{"node_id" => "assess", "error_message" => reason}] = result["node_failures"]
      assert reason =~ sample["operator_id"]
      assert reason =~ sample["error"]
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

      clean =
        Enum.find(
          cases,
          &(&1["operator_id"] == sample["operator_id"] and not Map.has_key?(&1, "error"))
        )

      {200, fresh} = post_graph("run", Fixture.request(clean, %{}))
      assert fresh["failed_nodes"] == []

      assert Map.take(fresh["artifacts"]["result.payload"], Map.keys(clean["expected"])) ==
               clean["expected"]

      assert {:ok, ^retained} = AnalysisResultStore.get(job_id)
    end
  end

  test "fail-fast summary reductions finish failed without storing a partial assessment" do
    for sample <- Fixture.reduction_cases(), Map.has_key?(sample, "error") do
      {202, submitted} = post_graph("jobs", Fixture.request(sample, %{}))
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "failed"
      assert completed["job"]["message"] =~ sample["error"]
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
