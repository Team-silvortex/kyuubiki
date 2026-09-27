defmodule KyuubikiWeb.Api.WorkflowArtifactPublicationApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowArtifactContract, as: Fixture

  test "generated report overflow retains failure receipts in full and compact HTTP responses" do
    for compact <- [false, true] do
      request = options(Fixture.exported("skip", true), compact)
      {200, result} = post_graph("run", request)
      assert_rejected_output(result, request, compact)
      {422, error} = post_graph("run", options(Fixture.exported("fail", true), compact))
      assert error["error"] =~ "producer"
      assert error["error"] =~ "length security budget"
      refute Map.has_key?(error, "artifacts")
    end

    {200, clean} = post_graph("run", Fixture.exported("fail", false))
    assert clean["failed_nodes"] == []

    assert Jason.decode!(clean["artifacts"]["output.payload"]["content"]) == %{
             "text" => "healthy"
           }
  end

  test "async output rejection persists honest failure and a subsequent healthy job remains usable" do
    for compact <- [false, true] do
      request = options(Fixture.exported("skip", true), compact)
      {202, submitted} = post_graph("jobs", request)
      job_id = submitted["job"]["job_id"]
      result = WorkflowApi.wait_for_job(job_id, @opts)
      assert result["job"]["status"] == "completed"
      assert result["job"]["message"] =~ "1 failed node(s)"
      assert_rejected_output(result["result"], request, compact)
      {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["result"]["node_failures"]
      events = result["result"]["progress_events"]
      assert Enum.find(events, &(&1["node_id"] == "producer"))["status"] == "failed"
      last = List.last(events)
      assert last["failed_nodes"] == 1
      assert last["skipped_nodes"] == 2
      assert last["completed_nodes"] == 2
      assert last["resolved_nodes"] == 5
    end

    {202, submitted} = post_graph("jobs", Fixture.exported("fail", true))
    failed_id = submitted["job"]["job_id"]
    failed = WorkflowApi.wait_for_job(failed_id, @opts)
    assert failed["job"]["status"] == "failed"
    assert failed["job"]["message"] =~ "length security budget"
    assert failed["result"]["artifacts"] == %{}
    assert failed["result"]["recovery"]["state"] == "failed"
    {:ok, retained} = AnalysisResultStore.get(failed_id)
    assert retained["artifacts"] == %{}

    {202, submitted} = post_graph("jobs", Fixture.exported("fail", false))
    clean = WorkflowApi.wait_for_job(submitted["job"]["job_id"], @opts)
    assert clean["job"]["status"] == "completed"
    assert clean["result"]["failed_nodes"] == []

    assert Jason.decode!(clean["result"]["artifacts"]["output.payload"]["content"]) == %{
             "text" => "healthy"
           }
  end

  defp assert_rejected_output(result, request, compact) do
    assert result["failed_nodes"] == ["producer"]
    assert Enum.sort(result["skipped_nodes"]) == ["consumer", "output"]
    assert [%{"node_id" => "producer", "error_message" => reason}] = result["node_failures"]
    assert reason =~ "length security budget"

    if compact do
      refute Map.has_key?(result, "artifacts")
      refute Map.has_key?(result, "artifact_lineage")
    else
      assert Map.keys(result["artifacts"]) |> Enum.sort() == ["input.payload", "raw.payload"]
      assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["input"]
      refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == "producer"))
    end
  end

  defp options(request, compact),
    do:
      Map.put(request, "response_options", %{
        "response_mode" => if(compact, do: "compact", else: "full")
      })

  defp post_graph(action, request) do
    conn =
      :post
      |> conn("/api/v1/workflows/graph/#{action}", Jason.encode!(request))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {conn.status, Jason.decode!(conn.resp_body)}
  end
end
