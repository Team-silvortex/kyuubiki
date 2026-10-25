defmodule KyuubikiWeb.Api.WorkflowSummaryComparisonApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowSummaryAdmission, as: Fixture

  test "comparison range errors retain raw evidence and a fresh zero-baseline request succeeds" do
    for sample <- Fixture.comparison_cases(),
        Map.has_key?(sample, "error"),
        mode <- ["full", "compact"] do
      {422, rejected} = post_graph("run", Fixture.request(sample, %{}))
      assert rejected["error"] =~ "transform.compare_summary_pair"
      assert rejected["error"] =~ sample["error"]

      request =
        Fixture.request(sample, %{}, "left", true)
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
      assert reason =~ sample["error"]
      assert {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["node_failures"]

      if mode == "full" do
        refute Map.has_key?(result["artifacts"], "assess.payload")
        refute Map.has_key?(result["artifacts"], "result.payload")
        assert result["artifacts"]["raw.payload"] == sample["payload"]["left"]
        assert retained["artifacts"] == result["artifacts"]
      else
        refute Map.has_key?(result, "artifacts")
      end

      {200, fresh} = post_graph("run", Fixture.request(hd(Fixture.comparison_cases()), %{}))
      assert fresh["failed_nodes"] == []
      assert fresh["artifacts"]["result.payload"]["delta_metric"] == 4.0
      refute Map.has_key?(fresh["artifacts"]["result.payload"], "ratio_metric")
      assert {:ok, ^retained} = AnalysisResultStore.get(job_id)
    end
  end

  test "asynchronous fail-fast comparison records a failure envelope without an assessment" do
    for sample <- Fixture.comparison_cases(), Map.has_key?(sample, "error") do
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
