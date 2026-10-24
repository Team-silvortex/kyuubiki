defmodule KyuubikiWeb.Api.WorkflowExecutionEvidenceApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowExecutionEvidence, as: Fixture

  test "HTTP condition traces agree with actual branch publication" do
    for sample <- Fixture.cases(), reverse <- [false, true] do
      {200, result} = post_graph("run", Fixture.request(sample["value"], reverse))
      assert result["failed_nodes"] == []

      assert Fixture.trace(result, "branch")["produced_artifacts"] == [
               "branch.#{sample["chosen_output"]}"
             ]

      assert_publications(result)

      assert Map.fetch!(result["artifacts"], "branch.#{sample["chosen_output"]}") ==
               sample["value"]
    end
  end

  test "async branch receipts stay consistent after SQLite result readback" do
    for sample <- Fixture.cases() do
      {202, submitted} = post_graph("jobs", Fixture.request(sample["value"], true, true))
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "completed"
      result = completed["result"]

      assert Fixture.trace(result, "branch")["produced_artifacts"] == [
               "branch.#{sample["chosen_output"]}"
             ]

      assert Fixture.trace(result, "choose")["consumed_artifacts"] == ["b.payload"]
      assert_publications(result)
      assert {:ok, retained} = AnalysisResultStore.get(job_id)

      assert Map.take(retained, ~w(node_runs artifact_lineage branch_decisions artifacts)) ==
               Map.take(result, ~w(node_runs artifact_lineage branch_decisions artifacts))
    end
  end

  test "persisted failed fallback receipts preserve selected input and compact failure cause" do
    for options <- [
          %{"response_mode" => "full"},
          %{"include_artifacts" => false},
          %{"response_mode" => "compact"}
        ] do
      request = Fixture.failure("b", false, true) |> Map.put("response_options", options)
      {202, submitted} = post_graph("jobs", request)
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "completed"
      result = completed["result"]
      assert result["failed_nodes"] == ["choose"]
      assert [%{"node_id" => "choose", "error_message" => reason}] = result["node_failures"]
      assert reason =~ "length security budget"

      if Map.has_key?(result, "node_runs") do
        assert Fixture.trace(result, "choose")["consumed_artifacts"] == ["b.payload"]
        assert_publications(result)
      else
        assert options["response_mode"] == "compact"
      end

      assert {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["node_failures"]

      assert Map.take(retained, ~w(node_runs artifact_lineage branch_decisions artifacts)) ==
               Map.take(result, ~w(node_runs artifact_lineage branch_decisions artifacts))
    end
  end

  defp assert_publications(result) do
    for trace <- result["node_runs"] do
      assert Enum.sort(trace["produced_artifacts"]) == Fixture.published(result, trace["node_id"])
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
