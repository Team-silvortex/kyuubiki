defmodule KyuubikiWeb.Api.AnalysisCollectionOutageTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Analysis, Workloads}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @collections ["jobs", "results", "database", "catalog", "bundle"]

  setup do
    Outage.setup()
    assert {:ok, project} = Library.create_project(%{"name" => "Collection recovery"})

    assert {:ok, model} =
             Library.create_model(project["project_id"], %{
               "name" => "Read fixture",
               "kind" => "truss_2d",
               "payload" => %{"nodes" => [], "elements" => []}
             })

    assert {:ok, version} =
             Library.create_version(model["model_id"], %{
               "kind" => "truss_2d",
               "payload" => %{"nodes" => [], "elements" => []}
             })

    assert {:ok, model} = Library.get_model(model["model_id"])

    fixture = Fixture.terminal_job()

    assert {:ok, job} =
             Store.update_metadata(fixture.id, %{
               "project_id" => project["project_id"],
               "model_version_id" => version["version_id"]
             })

    %{project: project, model: model, version: version, execution: %{fixture | job: job}}
  end

  for collection <- @collections do
    test "#{collection} collection fails closed during outage and restores original records",
         fixture do
      path = path(unquote(collection), fixture.project["project_id"])
      original = request(path)
      assert original.status == 200

      Outage.without(Outage.store_child(), fn ->
        reply = request(path)
        assert reply.status == 503
        assert Jason.decode!(reply.resp_body) == %{"error" => "analysis_store_unavailable"}
        assert get_resp_header(reply, "content-type") == ["application/json; charset=utf-8"]
        assert get_resp_header(reply, "content-disposition") == []
      end)

      restored = request(path)
      assert restored.status == 200
      assert stable_payload(restored) == stable_payload(original)

      assert get_resp_header(restored, "content-disposition") ==
               get_resp_header(original, "content-disposition")

      Fixture.unchanged(fixture.execution)
      assert {:ok, model} = Library.get_model(fixture.model["model_id"])
      assert model == fixture.model
      assert {:ok, version} = Library.get_version(fixture.version["version_id"])
      assert version == fixture.version
    end
  end

  test "headless collections and composite reads return the storage error without partial data",
       fixture do
    Outage.without(Outage.store_child(), fn ->
      for read <- [
            &Analysis.list_jobs/0,
            &Analysis.list_results/0,
            &Analysis.export_database/0,
            fn -> Workloads.workload_catalog("http://localhost") end,
            fn -> Workloads.export_project_bundle(fixture.project["project_id"]) end
          ] do
        assert {:error, :analysis_store_unavailable} = read.()
      end
    end)

    Fixture.unchanged(fixture.execution)
  end

  test "real empty job and result collections remain successful, not unavailable" do
    Store.reset()
    AnalysisResultStore.reset()

    for {collection, key} <- [{"jobs", "jobs"}, {"results", "results"}] do
      reply = request(path(collection, "unused"))
      assert reply.status == 200
      assert Jason.decode!(reply.resp_body) == %{key => []}
    end
  end

  test "collection authorization precedes storage reads even during an outage", fixture do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "collection-test-token",
      protect_reads?: true
    )

    Outage.without(Outage.store_child(), fn ->
      for collection <- @collections do
        path = path(collection, fixture.project["project_id"])
        rejected = request(path)
        assert rejected.status == 401
        assert Jason.decode!(rejected.resp_body)["error"] == "unauthorized"

        authorized =
          conn(:get, path)
          |> put_req_header("authorization", "Bearer collection-test-token")
          |> Router.call(@opts)

        assert authorized.status == 503
        assert Jason.decode!(authorized.resp_body) == %{"error" => "analysis_store_unavailable"}
      end
    end)
  end

  test "result collections and exports never expose private recovery records", fixture do
    for collection <- ["results", "database", "bundle"] do
      reply = request(path(collection, fixture.project["project_id"]))
      assert reply.status == 200
      assert [entry] = Jason.decode!(reply.resp_body)["results"]
      assert entry["job_id"] == fixture.execution.id
      assert entry["result"]["recovery"]["state"] == "completed"
      refute Map.has_key?(entry["result"], WorkflowRecoveryEnvelope.internal_key())
      refute String.contains?(reply.resp_body, fixture.execution.claim["owner_session_id"])
    end

    Fixture.unchanged(fixture.execution)
  end

  test "an absent project keeps its original not-found response" do
    reply = request(path("bundle", "collection-project-missing"))
    assert reply.status == 404
    assert Jason.decode!(reply.resp_body) == %{"error" => "project_not_found"}
  end

  defp path("jobs", _), do: "/api/v1/jobs"
  defp path("results", _), do: "/api/v1/results"
  defp path("database", _), do: "/api/v1/export/database"
  defp path("catalog", _), do: "/api/v1/workloads/catalog"
  defp path("bundle", project), do: "/api/v1/projects/#{project}/bundle"
  defp request(path), do: conn(:get, path) |> Router.call(@opts)

  defp stable_payload(reply),
    do: reply.resp_body |> Jason.decode!() |> Map.drop(["generated_at", "exported_at"])
end
