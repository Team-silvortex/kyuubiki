defmodule KyuubikiWeb.Api.WorkflowReportingIntegrityApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowReportingContract, as: Fixture

  test "graph API rejects invalid reporting evidence and remains usable after corrected replay" do
    for hotspots <- [false, true] do
      clean = Fixture.request(hotspots)

      for {request, node, path} <- Fixture.failures(clean) do
        {422, error} = post_graph(request)
        assert inspect(error) =~ node
        assert inspect(error) =~ path
        refute Map.has_key?(error, "artifacts")
        {200, result} = post_graph(clean)
        report = Jason.decode!(result["artifacts"]["output.summary"]["content"])
        assert report["report_guard_status"] == "pass"
        assert report["bundle_total_node_count"] == nil
      end
    end
  end

  test "graph API returns a real blocked report for a valid exceeded threshold" do
    for hotspots <- [false, true] do
      request =
        Fixture.request(hotspots)
        |> Fixture.node("guard", &put_in(&1, ["config", "rules", Access.at(0), "threshold"], 2))

      {200, result} = post_graph(request)
      report = Jason.decode!(result["artifacts"]["output.summary"]["content"])
      assert report["report_guard_status"] == "block"
      assert report["guard_payload"]["guard_passed"] == false
    end
  end

  defp post_graph(request) do
    conn =
      :post
      |> conn("/api/v1/workflows/graph/run", Jason.encode!(request))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {conn.status, Jason.decode!(conn.resp_body)}
  end
end
