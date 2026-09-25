defmodule KyuubikiWeb.Api.WorkflowBranchRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowReportingContract, as: Fixture

  test "synchronous recovery retains failure receipts in full and compact API results" do
    for compact <- [false, true], {bad, id, path} <- Fixture.failures(Fixture.request()) do
      request = recover(bad, id, compact)
      {200, result} = post_graph("run", request)
      assert result["failed_nodes"] == [id]
      assert [%{"node_id" => ^id, "error_message" => error}] = result["node_failures"]
      assert error =~ path
      assert result["performance"]["failed_node_count"] == 1

      if compact do
        refute Map.has_key?(result, "node_runs")
        refute Map.has_key?(result, "artifacts")
      else
        assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["input"]
        refute Map.has_key?(result["artifacts"], "output.summary")
      end
    end
  end

  test "asynchronous jobs persist isolated failure receipts and honest terminal progress" do
    for compact <- [false, true] do
      {bad, id, path} = hd(Fixture.failures(Fixture.request()))
      {202, submitted} = post_graph("jobs", recover(bad, id, compact))
      job_id = submitted["job"]["job_id"]
      payload = WorkflowApi.wait_for_job(job_id, @opts)
      assert payload["job"]["status"] == "completed"
      result = payload["result"]
      assert result["failed_nodes"] == [id]
      assert hd(result["node_failures"])["error_message"] =~ path
      assert result["recovery"]["state"] == "completed"
      assert result["recovery"]["attempt"] == 1
      events = result["progress_events"]
      assert length(events) == 9
      assert Enum.find(events, &(&1["node_id"] == id))["status"] == "failed"
      last = List.last(events)
      assert last["resolved_nodes"] == 9
      assert last["completed_nodes"] == 3
      assert last["failed_nodes"] == 1
      assert last["skipped_nodes"] == 5
      assert last["execution_progress"] == 0.98
      assert last["progress"] == 0.98
      if compact, do: refute(Map.has_key?(result, "node_runs"))

      {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["node_failures"]
      assert {:ok, %{iteration: 9, message: message}} = Store.get(job_id)
      assert message =~ "1 failed node(s)"
    end
  end

  test "default failure remains an error and corrected replay cannot inherit recovery state" do
    {bad, _, _} = hd(Fixture.failures(Fixture.request()))
    {422, error} = post_graph("run", bad)
    refute Map.has_key?(error, "artifacts")
    {202, submitted} = post_graph("jobs", bad)
    payload = WorkflowApi.wait_for_job(submitted["job"]["job_id"], @opts)
    assert payload["job"]["status"] == "failed"
    {200, clean} = post_graph("run", Fixture.request())
    assert clean["failed_nodes"] == []
    assert clean["node_failures"] == []

    assert Jason.decode!(clean["artifacts"]["output.summary"]["content"])["report_guard_status"] ==
             "pass"
  end

  test "invalid recovery policy cannot be hidden behind a canonical skip" do
    request =
      Fixture.request()
      |> Fixture.node(
        "guard",
        &Map.put(&1, "config", %{
          "on_error" => "skip",
          "recovery" => %{"on_error" => nil}
        })
      )

    {422, error} = post_graph("run", request)
    assert inspect(error) =~ "config.recovery.on_error"
    refute Map.has_key?(error, "artifacts")
  end

  defp recover(request, id, compact) do
    request
    |> Fixture.node(id, &put_in(&1, ["config", "on_error"], "skip"))
    |> Map.put("response_options", %{"response_mode" => if(compact, do: "compact", else: "full")})
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
