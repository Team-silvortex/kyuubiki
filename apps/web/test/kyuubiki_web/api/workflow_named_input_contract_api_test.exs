defmodule KyuubikiWeb.Api.WorkflowNamedInputContractApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.TestSupport.WorkflowNamedInputContract, as: Fixture

  test "HTTP named-input graphs route all shared operators without dropping a source" do
    for sample <- Fixture.cases() do
      {200, run} = post_graph("run", Fixture.request(sample))
      result = run["artifacts"]["result.summary"]
      assert Map.take(result, Map.keys(sample["expected"])) == sample["expected"]
      assert run["failed_nodes"] == []
      trace = Enum.find(run["node_runs"], &(&1["node_id"] == "combine"))
      assert trace["consumed_artifacts"] == ["left.payload", "right.payload"]

      if sample["operator_id"] == "transform.join_parameter_sweep_results" do
        assert Enum.map(result["cases"], &{&1["id"], &1["summary"]["cost"]}) == [
                 {"a", 10},
                 {"b", 4}
               ]
      end
    end
  end

  test "async named-input result and both source receipts survive storage" do
    sample = hd(Fixture.cases())
    {202, submitted} = post_graph("jobs", Fixture.request(sample))
    job_id = submitted["job"]["job_id"]
    completed = WorkflowApi.wait_for_job(job_id, @opts)
    assert completed["job"]["status"] == "completed"
    result = completed["result"]
    assert result["artifacts"]["result.summary"]["benchmark_winner"] == "right"
    assert result["failed_nodes"] == []
    assert result["skipped_nodes"] == []
    {:ok, retained} = AnalysisResultStore.get(job_id)
    assert retained["artifacts"] == result["artifacts"]
    lineage = Enum.find(retained["artifact_lineage"], &(&1["node_id"] == "combine"))
    assert lineage["source_artifacts"] == ["left.payload", "right.payload"]
  end

  test "invalid right input returns 422 or a persisted recovery receipt, never a winner" do
    sample = hd(Fixture.cases())
    bad = sample |> Fixture.request() |> put_in(["input_artifacts", "right"], nil)
    {422, error} = post_graph("run", bad)
    assert error["error"] =~ "combine"
    refute Map.has_key?(error, "artifacts")

    for mode <- ["full", "compact"] do
      request =
        bad
        |> Map.put("response_options", %{"response_mode" => mode})
        |> update_in(["graph", "nodes"], fn nodes ->
          Enum.map(
            nodes,
            &if(&1["id"] == "combine", do: put_in(&1, ["config", "on_error"], "skip"), else: &1)
          )
        end)

      {202, submitted} = post_graph("jobs", request)
      job_id = submitted["job"]["job_id"]
      completed = WorkflowApi.wait_for_job(job_id, @opts)
      assert completed["job"]["status"] == "completed"
      assert completed["job"]["message"] =~ "1 failed node(s)"
      result = completed["result"]
      assert result["failed_nodes"] == ["combine"]
      assert result["skipped_nodes"] == ["result"]
      assert [%{"node_id" => "combine"}] = result["node_failures"]
      {:ok, retained} = AnalysisResultStore.get(job_id)
      assert retained["node_failures"] == result["node_failures"]

      if mode == "full" do
        refute Map.has_key?(result["artifacts"], "result.summary")
        refute Map.has_key?(result["artifacts"], "combine.summary")
        assert result["artifacts"]["raw.payload"] == hd(sample["values"])
      else
        refute Map.has_key?(result, "artifacts")
      end

      {200, clean} = post_graph("run", Fixture.request(sample))
      assert clean["failed_nodes"] == []
      assert clean["artifacts"]["result.summary"]["benchmark_winner"] == "right"
      assert {:ok, ^retained} = AnalysisResultStore.get(job_id)
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
