defmodule KyuubikiWeb.Library.HistoryReadSnapshotTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.TestSupport.ExportReadInterleave, as: Interleave

  @moduletag skip: not Storage.sql?()
  @request_id "history-snapshot-request-0001"

  setup do
    repo = Storage.repo_module!()
    writer_repo = start_supervised!({repo, name: nil, pool_size: 1}, id: :history_writer)
    {:ok, project} = Library.create_project(%{"name" => "History snapshot"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Snapshot model",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1},
        "request_id" => @request_id
      })

    %{project: project, model: model, writer_repo: writer_repo}
  end

  for read <- [:projects, :project] do
    test "#{read} keeps the project and embedded models from one SQL snapshot", fixture do
      kind = unquote(read)
      original = read(kind, fixture)

      handler =
        Interleave.after_read("kyuubiki_projects", fn ->
          with_writer(fixture, fn -> Library.delete_project(fixture.project["project_id"]) end)
        end)

      concurrent = read(kind, fixture)
      assert_received {:export_writer_committed, ^handler, {:ok, _}}
      assert concurrent == original
      assert read(kind, fixture) == missing(kind)
    end
  end

  test "model detail keeps its latest pointer and version list from one SQL snapshot", fixture do
    original = read(:model, fixture)

    handler =
      Interleave.after_read("kyuubiki_models", fn ->
        with_writer(fixture, fn ->
          Library.create_version(fixture.model["model_id"], %{"payload" => %{"revision" => 2}})
        end)
      end)

    assert read(:model, fixture) == original
    assert_received {:export_writer_committed, ^handler, {:ok, version}}
    assert {:ok, current} = read(:model, fixture)
    assert current["latest_version_id"] == version["version_id"]
    assert hd(current["versions"])["version_id"] == version["version_id"]
  end

  test "checkpoint reconciliation does not mix a receipt with later deletion", fixture do
    original = read(:checkpoint, fixture)
    assert {:ok, %{"status" => "committed"}} = original

    handler =
      Interleave.after_read("kyuubiki_checkpoint_requests", fn ->
        with_writer(fixture, fn -> Library.delete_project(fixture.project["project_id"]) end)
      end)

    assert read(:checkpoint, fixture) == original
    assert_received {:export_writer_committed, ^handler, {:ok, _}}
    assert read(:checkpoint, fixture) == {:ok, %{"status" => "unknown"}}
  end

  for kind <- [:projects, :project, :model, :checkpoint] do
    test "#{kind} HTTP history retains a coherent view during concurrent deletion", fixture do
      kind = unquote(kind)
      original = request(kind, fixture)
      assert original.status == 200

      handler =
        Interleave.after_read(table(kind), fn ->
          with_writer(fixture, fn -> Library.delete_project(fixture.project["project_id"]) end)
        end)

      concurrent = request(kind, fixture)
      assert_received {:export_writer_committed, ^handler, {:ok, _}}
      assert concurrent.status == 200
      assert concurrent.resp_body == original.resp_body

      assert_cache_policy(kind, concurrent)
    end
  end

  defp request(kind, fixture) do
    path =
      case kind do
        :projects -> "/api/v1/projects"
        :project -> "/api/v1/projects/#{fixture.project["project_id"]}"
        :model -> "/api/v1/models/#{fixture.model["model_id"]}"
        :checkpoint -> "/api/v1/checkpoints/model/#{fixture.project["project_id"]}/#{@request_id}"
      end

    conn(:get, path) |> Router.call(@opts)
  end

  defp table(:projects), do: "kyuubiki_projects"
  defp table(:project), do: "kyuubiki_projects"
  defp table(:model), do: "kyuubiki_models"
  defp table(:checkpoint), do: "kyuubiki_checkpoint_requests"

  defp assert_cache_policy(:checkpoint, response),
    do: assert(get_resp_header(response, "cache-control") == ["no-store"])

  defp assert_cache_policy(_, _), do: :ok

  defp read(:projects, _), do: Library.list_projects()
  defp read(:project, f), do: Library.get_project(f.project["project_id"])
  defp read(:model, f), do: Library.get_model(f.model["model_id"])

  defp read(:checkpoint, f),
    do: Library.get_checkpoint(:model, f.project["project_id"], @request_id)

  defp missing(:projects), do: {:ok, []}
  defp missing(:project), do: :error

  defp with_writer(fixture, callback) do
    apply(Storage.repo_module!(), :put_dynamic_repo, [fixture.writer_repo])
    callback.()
  end
end
