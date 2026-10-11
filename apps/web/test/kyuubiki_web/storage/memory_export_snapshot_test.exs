defmodule KyuubikiWeb.Storage.MemoryExportSnapshotTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Analysis, Storage, Workloads}
  alias KyuubikiWeb.Library.MemoryBackend, as: LibraryMemory
  alias KyuubikiWeb.SecurityEvents.MemoryBackend, as: SecurityMemory
  alias KyuubikiWeb.Storage.AnalysisMemoryState
  alias KyuubikiWeb.TestSupport.MemoryReadInterleave, as: Interleave
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.memory?()
  @exports [:bundle, :database, :catalog]

  setup do
    Outage.setup()
    {:ok, project} = Library.create_project(%{"name" => "JSON export"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Read fixture",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1}
      })

    fixture = Fixture.terminal_job()

    {:ok, job} =
      Store.update_metadata(fixture.id, %{
        "project_id" => project["project_id"],
        "model_version_id" => model["latest_version_id"]
      })

    %{project: project, model: model, execution: %{fixture | job: job}}
  end

  for export <- @exports do
    test "#{export} retries a concurrent model/version edit without mixed records", fixture do
      kind = unquote(export)

      handler =
        Interleave.after_capture(LibraryMemory, fn ->
          Library.create_version(fixture.model["model_id"], %{"payload" => %{"revision" => 2}})
        end)

      actual = request(kind, fixture)
      assert_received {:memory_writer_committed, ^handler, 1, {:ok, _version}}
      assert actual.status == 200
      assert stable(actual) == stable(request(kind, fixture))
      assert_models(kind, Jason.decode!(actual.resp_body))
    end

    test "#{export} retries joint job/result deletion without an orphan task", fixture do
      kind = unquote(export)

      handler =
        Interleave.after_capture(AnalysisMemoryState, fn ->
          Store.delete_with_result(fixture.execution.id)
        end)

      actual = request(kind, fixture)
      assert_received {:memory_writer_committed, ^handler, 1, {:ok, _job}}
      assert actual.status == 200
      assert stable(actual) == stable(request(kind, fixture))
      assert_empty_jobs(kind, Jason.decode!(actual.resp_body))
    end

    test "#{export} fails closed after bounded read conflicts and recovers without an attachment",
         fixture do
      kind = unquote(export)

      handler =
        Interleave.after_capture(
          LibraryMemory,
          fn ->
            Library.update_project(fixture.project["project_id"], %{"name" => "Concurrent"})
          end,
          repeat: true
        )

      actual = request(kind, fixture)

      for attempt <- 1..3 do
        assert_received {:memory_writer_committed, ^handler, ^attempt, {:ok, _project}}
      end

      refute_received {:memory_writer_committed, ^handler, 4, _}
      assert actual.status == 503
      assert Jason.decode!(actual.resp_body) == %{"error" => "analysis_store_unavailable"}
      assert get_resp_header(actual, "content-disposition") == []
      :telemetry.detach(handler)
      assert request(kind, fixture).status == 200
      Fixture.unchanged(fixture.execution)
    end

    test "#{export} contains project-library outage before reading other stores", fixture do
      kind = unquote(export)

      Outage.without(LibraryMemory, fn ->
        actual = request(kind, fixture)
        assert actual.status == 503
        assert Jason.decode!(actual.resp_body) == %{"error" => "analysis_store_unavailable"}
        assert get_resp_header(actual, "content-disposition") == []
      end)

      Fixture.unchanged(fixture.execution)
    end

    test "#{export} handles a concurrent project deletion without publishing partial data",
         fixture do
      kind = unquote(export)

      handler =
        Interleave.after_capture(LibraryMemory, fn ->
          Library.delete_project(fixture.project["project_id"])
        end)

      actual = request(kind, fixture)
      assert_received {:memory_writer_committed, ^handler, 1, {:ok, _project}}
      assert stable(actual) == stable(request(kind, fixture))
      assert_deleted_project(kind, actual)
    end

    test "#{export} rejects a mid-read analysis outage and restores original data", fixture do
      kind = unquote(export)
      original = request(kind, fixture)

      handler =
        Interleave.after_capture(AnalysisMemoryState, fn -> Outage.stop(AnalysisMemoryState) end)

      try do
        actual = request(kind, fixture)
        assert_received {:memory_writer_committed, ^handler, 1, :ok}
        assert actual.status == 503
        assert Jason.decode!(actual.resp_body) == %{"error" => "analysis_store_unavailable"}
        assert get_resp_header(actual, "content-disposition") == []
      after
        Outage.start(AnalysisMemoryState)
      end

      restored = request(kind, fixture)
      assert restored.status == 200
      assert stable(restored) == stable(original)
      Fixture.unchanged(fixture.execution)
    end
  end

  test "database export retries security-event publication within the read boundary", fixture do
    handler =
      Interleave.after_capture(SecurityMemory, fn ->
        SecurityEventStore.create(%{
          "event_id" => "json-snapshot-event",
          "event_type" => "automation",
          "source" => "snapshot-test",
          "action" => "export-check",
          "risk" => "low",
          "status" => "allowed",
          "occurred_at" => DateTime.to_iso8601(DateTime.utc_now())
        })
      end)

    actual = request(:database, fixture)
    assert actual.status == 200
    assert_received {:memory_writer_committed, ^handler, 1, {:ok, event}}
    assert Jason.decode!(actual.resp_body)["security_events"] == [event]
    assert stable(actual) == stable(request(:database, fixture))
  end

  test "only database export requires the separate security-event store", fixture do
    Outage.without(SecurityMemory, fn ->
      actual = request(:database, fixture)
      assert actual.status == 503
      assert Jason.decode!(actual.resp_body) == %{"error" => "analysis_store_unavailable"}
      assert request(:bundle, fixture).status == 200
      assert request(:catalog, fixture).status == 200
    end)

    assert request(:database, fixture).status == 200
  end

  test "direct composite helpers contain the separate library/security store failures", fixture do
    Outage.without(LibraryMemory, fn ->
      assert {:error, :analysis_store_unavailable} = Analysis.export_database()

      assert {:error, :analysis_store_unavailable} =
               Workloads.workload_catalog("http://localhost")

      assert {:error, :analysis_store_unavailable} =
               Workloads.export_project_bundle(fixture.project["project_id"])
    end)

    Outage.without(SecurityMemory, fn ->
      assert {:error, :analysis_store_unavailable} = Analysis.export_database()
    end)
  end

  defp request(kind, fixture), do: conn(:get, path(kind, fixture)) |> Router.call(@opts)
  defp path(:bundle, fixture), do: "/api/v1/projects/#{fixture.project["project_id"]}/bundle"
  defp path(:database, _fixture), do: "/api/v1/export/database"
  defp path(:catalog, _fixture), do: "/api/v1/workloads/catalog"

  defp stable(reply),
    do: Jason.decode!(reply.resp_body) |> Map.drop(["exported_at", "generated_at"])

  defp assert_models(:catalog, payload),
    do: assert(hd(payload["workloads"])["version_count"] == 2)

  defp assert_models(_kind, payload) do
    assert [latest, _initial] = payload["model_versions"]
    assert hd(payload["models"])["latest_version_id"] == latest["version_id"]
    assert hd(payload["models"])["payload"] == latest["payload"]
  end

  defp assert_empty_jobs(:catalog, payload) do
    assert hd(payload["workloads"])["job_count"] == 0
    assert hd(payload["workloads"])["result_count"] == 0
  end

  defp assert_empty_jobs(_kind, payload) do
    assert payload["jobs"] == []
    assert payload["results"] == []
  end

  defp assert_deleted_project(:bundle, reply) do
    assert reply.status == 404
    assert Jason.decode!(reply.resp_body) == %{"error" => "project_not_found"}
    assert get_resp_header(reply, "content-disposition") == []
  end

  defp assert_deleted_project(:database, reply) do
    assert reply.status == 200
    assert Jason.decode!(reply.resp_body)["projects"] == []
    assert Jason.decode!(reply.resp_body)["model_versions"] == []
  end

  defp assert_deleted_project(:catalog, reply) do
    assert reply.status == 200
    assert Jason.decode!(reply.resp_body)["workloads"] == []
  end
end
