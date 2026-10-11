defmodule KyuubikiWeb.Api.AnalysisReadOutageTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage

  setup do
    Outage.setup()
    id = "analysis-read-outage-#{System.unique_integer([:positive])}"
    result = %{"nodes" => [%{"id" => 0}], "elements" => []}

    assert {:ok, job} =
             Store.create_with_result(
               %{job_id: id, project_id: "read-outage", simulation_case_id: "case"},
               result
             )

    %{id: id, job: job, result: result}
  end

  for suffix <- ["job", "status", "result", "chunk"] do
    test "#{suffix} read reports unavailable storage and succeeds unchanged after restoration",
         fixture do
      path = path(unquote(suffix), fixture.id)
      before = request(path)
      assert before.status == 200

      Outage.without(Outage.store_child(), fn ->
        reply = request(path)
        assert reply.status == 503
        assert Jason.decode!(reply.resp_body) == %{"error" => "analysis_store_unavailable"}
        assert get_resp_header(reply, "content-type") == ["application/json; charset=utf-8"]
      end)

      restored = request(path)
      assert restored.status == 200
      assert Jason.decode!(restored.resp_body) == Jason.decode!(before.resp_body)
      assert {:ok, job} = Store.get(fixture.id)
      assert job == fixture.job
      assert {:ok, result} = AnalysisResultStore.get(fixture.id)
      assert result == fixture.result
    end
  end

  test "headless read helpers return a domain error instead of raising on storage loss",
       fixture do
    Outage.without(Outage.store_child(), fn ->
      for callback <- [
            fn -> Analysis.fetch_job(fixture.id) end,
            fn -> Analysis.fetch_job_status(fixture.id) end,
            fn -> Analysis.fetch_result(fixture.id) end,
            fn -> Analysis.fetch_result_chunk(fixture.id, "nodes", %{}) end
          ] do
        assert {:error, :analysis_store_unavailable} = callback.()
      end
    end)
  end

  defp path("job", id), do: "/api/v1/jobs/#{id}"
  defp path("status", id), do: "/api/v1/jobs/#{id}/status"
  defp path("result", id), do: "/api/v1/results/#{id}"
  defp path("chunk", id), do: "/api/v1/results/#{id}/chunks/nodes"
  defp request(path), do: conn(:get, path) |> Router.call(Router.init([]))
end
