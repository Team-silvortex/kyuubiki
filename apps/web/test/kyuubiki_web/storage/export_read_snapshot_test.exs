defmodule KyuubikiWeb.Storage.ExportReadSnapshotTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Analysis, Storage, Workloads}
  alias KyuubikiWeb.TestSupport.ExportReadInterleave, as: Interleave
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.sql?()
  @exports [:bundle, :database, :catalog]

  setup do
    repo = Storage.repo_module!()
    writer_repo = start_supervised!({repo, name: nil, pool_size: 1}, id: :export_writer)
    {:ok, project} = Library.create_project(%{"name" => "Consistent export"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Snapshot model",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1}
      })

    fixture = Fixture.terminal_job()

    {:ok, job} =
      Store.update_metadata(fixture.id, %{
        "project_id" => project["project_id"],
        "model_version_id" => model["latest_version_id"]
      })

    %{project: project, model: model, execution: %{fixture | job: job}, writer_repo: writer_repo}
  end

  for export <- @exports do
    test "#{export} keeps model and version collections from one generation", fixture do
      kind = unquote(export)
      original = read(kind, fixture)

      handler =
        Interleave.after_read("kyuubiki_models", fn ->
          with_writer(fixture, fn ->
            Library.create_version(fixture.model["model_id"], %{"payload" => %{"revision" => 2}})
          end)
        end)

      concurrent = read(kind, fixture)
      assert_received {:export_writer_committed, ^handler, {:ok, version}}
      assert stable(concurrent) == stable(original)
      current = read(kind, fixture)
      refute stable(current) == stable(original)

      assert_model_generation(kind, concurrent, current, fixture.model, version)
    end

    test "#{export} keeps job and result collections from one joint commit", fixture do
      kind = unquote(export)
      original = read(kind, fixture)
      execution = fixture.execution

      handler =
        Interleave.after_read("kyuubiki_jobs", fn ->
          with_writer(fixture, fn ->
            Store.delete_with_result(execution.id)
          end)
        end)

      concurrent = read(kind, fixture)
      assert_received {:export_writer_committed, ^handler, {:ok, _deleted}}
      assert stable(concurrent) == stable(original)
      current = read(kind, fixture)
      refute stable(current) == stable(original)

      assert_job_generation(kind, concurrent, current)
    end

    test "#{export} keeps a project removed after its first model read", fixture do
      kind = unquote(export)
      original = read(kind, fixture)

      handler =
        Interleave.after_read("kyuubiki_models", fn ->
          with_writer(fixture, fn -> Library.delete_project(fixture.project["project_id"]) end)
        end)

      concurrent = read(kind, fixture)
      assert_received {:export_writer_committed, ^handler, {:ok, _deleted}}
      assert stable(concurrent) == stable(original)
      assert_project_removed(kind, fixture)
    end
  end

  test "database security events share the same snapshot as project and analysis records",
       fixture do
    original = read(:database, fixture)

    handler =
      Interleave.after_read("kyuubiki_jobs", fn ->
        with_writer(fixture, fn ->
          SecurityEventStore.create(%{
            "event_id" => "export-event",
            "event_type" => "automation",
            "source" => "snapshot-test",
            "action" => "export-check",
            "risk" => "low",
            "status" => "allowed",
            "occurred_at" => DateTime.to_iso8601(DateTime.utc_now())
          })
        end)
      end)

    assert stable(read(:database, fixture)) == stable(original)
    assert_received {:export_writer_committed, ^handler, {:ok, event}}
    assert read(:database, fixture)["security_events"] == [event]
  end

  test "public exports refuse an ambient transaction without publishing downgraded data",
       fixture do
    repo = Storage.repo_module!()

    assert {:ok, :checked} =
             apply(repo, :transaction, [
               fn ->
                 assert {:ok, [_]} = Library.list_models(fixture.project["project_id"])
                 id = fixture.project["project_id"]

                 assert {:error, :analysis_store_unavailable} =
                          Workloads.export_project_bundle(id)

                 assert {:error, :analysis_store_unavailable} = Analysis.export_database()

                 assert {:error, :analysis_store_unavailable} =
                          Workloads.workload_catalog("http://localhost")

                 :checked
               end
             ])

    assert {:ok, _} = Workloads.export_project_bundle(fixture.project["project_id"])
    assert is_map(Analysis.export_database())
    assert is_map(Workloads.workload_catalog("http://localhost"))
  end

  defp read(:bundle, fixture) do
    {:ok, payload} = Workloads.export_project_bundle(fixture.project["project_id"])
    payload
  end

  defp read(:database, _fixture), do: Analysis.export_database()
  defp read(:catalog, _fixture), do: Workloads.workload_catalog("http://localhost")
  defp stable(payload), do: Map.drop(payload, ["exported_at", "generated_at"])

  defp with_writer(fixture, callback) do
    apply(Storage.repo_module!(), :put_dynamic_repo, [fixture.writer_repo])
    callback.()
  end

  defp assert_model_generation(:catalog, concurrent, current, _model, _version) do
    assert hd(concurrent["workloads"])["version_count"] == 1
    assert hd(current["workloads"])["version_count"] == 2
  end

  defp assert_model_generation(_kind, concurrent, current, model, version) do
    assert hd(concurrent["models"])["latest_version_id"] == model["latest_version_id"]
    assert hd(current["models"])["latest_version_id"] == version["version_id"]
    assert length(concurrent["model_versions"]) == 1
    assert length(current["model_versions"]) == 2
  end

  defp assert_job_generation(:catalog, concurrent, current) do
    assert hd(concurrent["workloads"])["job_count"] == 1
    assert hd(concurrent["workloads"])["result_count"] == 1
    assert hd(current["workloads"])["job_count"] == 0
    assert hd(current["workloads"])["result_count"] == 0
  end

  defp assert_job_generation(_kind, concurrent, current) do
    assert [job] = concurrent["jobs"]
    assert job["has_result"] == true
    assert [result] = concurrent["results"]
    assert result["job_id"] == job["job_id"]
    assert current["jobs"] == []
    assert current["results"] == []
  end

  defp assert_project_removed(:bundle, fixture) do
    assert {:error, :project_not_found} =
             Workloads.export_project_bundle(fixture.project["project_id"])
  end

  defp assert_project_removed(:database, fixture) do
    current = read(:database, fixture)
    assert current["projects"] == []
    assert current["models"] == []
    assert current["model_versions"] == []
  end

  defp assert_project_removed(:catalog, fixture),
    do: assert(read(:catalog, fixture)["workloads"] == [])
end
