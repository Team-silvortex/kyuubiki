defmodule KyuubikiWeb.Storage.HistoryPartialExportTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.{Analysis, Storage, Workloads}

  @moduletag skip: not Storage.sql?()

  setup do
    {:ok, project} = Library.create_project(%{"name" => "No partial history"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Versioned model",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1},
        "request_id" => "partial-export-request-0001"
      })

    %{project: project, model: model}
  end

  test "version query failure cannot be replaced by an empty exported history", fixture do
    without_table("kyuubiki_model_versions", fn ->
      for path <- [
            "/api/v1/projects/#{fixture.project["project_id"]}/bundle",
            "/api/v1/export/database",
            "/api/v1/workloads/catalog",
            "/api/v1/models/#{fixture.model["model_id"]}",
            "/api/v1/models/#{fixture.model["model_id"]}/versions",
            "/api/v1/checkpoints/model/#{fixture.project["project_id"]}/partial-export-request-0001"
          ] do
        assert_unavailable(path)
      end

      assert {:error, :analysis_store_unavailable} =
               Workloads.export_project_bundle(fixture.project["project_id"])

      assert {:error, :analysis_store_unavailable} = Analysis.export_database()

      assert {:error, :analysis_store_unavailable} =
               Workloads.workload_catalog("http://localhost")
    end)

    assert {:ok, model} = Library.get_model(fixture.model["model_id"])
    assert [version] = model["versions"]
    assert version["version_id"] == fixture.model["latest_version_id"]

    assert {:ok, %{"model_versions" => [^version]}} =
             Workloads.export_project_bundle(fixture.project["project_id"])
  end

  test "security query failure does not turn a database export into a successful partial map",
       fixture do
    without_table("kyuubiki_security_events", fn ->
      for path <- [
            "/api/v1/export/database",
            "/api/v1/security-events",
            "/api/v1/export/security-events",
            "/api/v1/export/security-events.csv"
          ] do
        assert_unavailable(path)
      end

      assert {:error, :analysis_store_unavailable} = Analysis.export_database()
      assert {:ok, _} = Workloads.export_project_bundle(fixture.project["project_id"])
      assert is_map(Workloads.workload_catalog("http://localhost"))
    end)

    assert is_map(Analysis.export_database())
  end

  defp without_table(table, callback) do
    repo = Storage.repo_module!()
    renamed = "history_temporarily_unavailable"
    SQL.query!(repo, "ALTER TABLE #{table} RENAME TO #{renamed}", [])

    try do
      callback.()
    after
      SQL.query!(repo, "ALTER TABLE #{renamed} RENAME TO #{table}", [])
    end
  end

  defp assert_unavailable(path) do
    response = conn(:get, path) |> Router.call(@opts)
    assert response.status == 503
    assert Jason.decode!(response.resp_body) == %{"error" => "analysis_store_unavailable"}
    assert get_resp_header(response, "content-disposition") == []
    assert get_resp_header(response, "content-type") == ["application/json; charset=utf-8"]
  end
end
