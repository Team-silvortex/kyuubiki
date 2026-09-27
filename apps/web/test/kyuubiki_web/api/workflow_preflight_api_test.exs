defmodule KyuubikiWeb.Api.WorkflowPreflightApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowPreflightContract, as: Fixture

  test "invalid graphs are rejected before dispatch and leave no job or result rows" do
    for test_case <- Fixture.cases(), not test_case["accept"], action <- ["run", "jobs"] do
      {422, payload} = post_graph(action, Fixture.request(test_case))
      assert payload["error"] =~ test_case["error"]
      assert Store.list() == []
      assert AnalysisResultStore.list() == []
    end

    {200, result} = post_graph("run", Fixture.request())
    assert result["artifacts"]["output.payload"] == %{"value" => 42}
    {202, submitted} = post_graph("jobs", Fixture.request())
    job_id = submitted["job"]["job_id"]
    result = WorkflowApi.wait_for_job(job_id, @opts)
    assert result["job"]["status"] == "completed"
    assert result["result"]["artifacts"]["output.payload"] == %{"value" => 42}
    assert length(Store.list()) == 1
    assert length(AnalysisResultStore.list()) == 1
  end

  test "invalid recovery policy is rejected before async persistence too" do
    request =
      put_in(Fixture.request(), ["graph", "nodes", Access.at(1), "config", "on_error"], nil)

    {422, result} = post_graph("jobs", request)
    assert result["error"] =~ "config.on_error"
    assert Store.list() == []
    assert AnalysisResultStore.list() == []
  end

  test "budget overflow cannot be queued even with explicit skip recovery" do
    for budget <- Fixture.budgets(), not budget["accept"] do
      {422, _} = post_graph("jobs", Fixture.budget_request(budget))
      assert Store.list() == []
      assert AnalysisResultStore.list() == []
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
