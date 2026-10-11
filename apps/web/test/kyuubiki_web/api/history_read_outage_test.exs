defmodule KyuubikiWeb.Api.HistoryReadOutageTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Analysis, Storage}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @library_reads ~w(projects project models model versions version model_checkpoint version_checkpoint)
  @security_reads ~w(security_events security_json security_csv)
  @model_request "history-model-request-0001"
  @version_request "history-version-request-0001"

  setup do
    Outage.setup()
    {:ok, project} = Library.create_project(%{"name" => "Independent history"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "History model",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1},
        "request_id" => @model_request
      })

    {:ok, version} =
      Library.create_version(model["model_id"], %{
        "payload" => %{"revision" => 2},
        "request_id" => @version_request
      })

    {:ok, event} =
      SecurityEventStore.create(%{
        "event_id" => "history-read-event",
        "event_type" => "automation",
        "source" => "history-test",
        "action" => "read-history",
        "risk" => "low",
        "status" => "allowed",
        "occurred_at" => DateTime.to_iso8601(DateTime.utc_now())
      })

    %{
      project: project,
      model: model,
      version: version,
      event: event,
      execution: Fixture.terminal_job()
    }
  end

  for read <- @library_reads do
    test "#{read} reports a library outage without replacing history with empty data", fixture do
      name = unquote(read)
      original = request(path(name, fixture))
      assert original.status == 200

      Outage.without(library_child(), fn ->
        assert_unavailable(request(path(name, fixture)))
      end)

      if Storage.sql?() do
        assert request(path(name, fixture)).resp_body == original.resp_body
      else
        # The lightweight library is volatile; its restart is not data recovery.
        assert request(path("projects", fixture)).resp_body == ~s({"projects":[]})
        assert request(path("model", fixture)).status == 404

        assert {:ok, %{"status" => "unknown"}} =
                 Library.get_checkpoint(:model, fixture.project["project_id"], @model_request)

        assert {:ok, _} = Library.create_project(%{"name" => "Usable after restart"})
      end

      Fixture.unchanged(fixture.execution)
    end
  end

  for read <- @security_reads do
    test "#{read} reports a security-store outage and restores the original event", fixture do
      name = unquote(read)
      original = request(path(name, fixture))
      assert original.status == 200

      Outage.without(security_child(), fn ->
        assert_unavailable(request(path(name, fixture)))
      end)

      restored = request(path(name, fixture))
      assert restored.status == 200
      assert stable(restored, name) == stable(original, name)
      assert SecurityEventStore.list() == [fixture.event]
      assert {:ok, _} = Library.get_model(fixture.model["model_id"])
      Fixture.unchanged(fixture.execution)
    end
  end

  test "headless history helpers return an opaque error, not successful error payloads",
       fixture do
    Outage.without(library_child(), fn ->
      for read <- [
            &Library.list_projects/0,
            fn -> Library.get_project(fixture.project["project_id"]) end,
            fn -> Library.list_models(fixture.project["project_id"]) end,
            fn -> Library.get_model(fixture.model["model_id"]) end,
            fn -> Library.list_versions(fixture.model["model_id"]) end,
            fn -> Library.get_version(fixture.version["version_id"]) end,
            fn ->
              Library.get_checkpoint(:model, fixture.project["project_id"], @model_request)
            end,
            fn ->
              Library.get_checkpoint(:version, fixture.model["model_id"], @version_request)
            end
          ] do
        assert {:error, :analysis_store_unavailable} = read.()
      end
    end)

    Outage.without(security_child(), fn ->
      for read <- [
            &Analysis.list_security_events/0,
            &Analysis.export_security_events/0,
            &Analysis.export_security_events_csv/0
          ] do
        assert {:error, :analysis_store_unavailable} = read.()
      end
    end)

    Fixture.unchanged(fixture.execution)
  end

  test "history authentication precedes storage access, including CSV export", fixture do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "history-read-test-token",
      protect_reads?: true
    )

    for {child, reads} <- [{library_child(), @library_reads}, {security_child(), @security_reads}] do
      Outage.without(child, fn ->
        for name <- reads do
          path = path(name, fixture)
          rejected = request(path)
          assert rejected.status == 401
          assert Jason.decode!(rejected.resp_body)["error"] == "unauthorized"

          authorized =
            conn(:get, path)
            |> put_req_header("authorization", "Bearer history-read-test-token")
            |> Router.call(@opts)

          assert_unavailable(authorized)
        end
      end)
    end
  end

  test "version context derivation propagates unavailability without creating a task", fixture do
    before_jobs = Store.list()

    Outage.without(library_child(), fn ->
      assert {:error, :analysis_store_unavailable} =
               KyuubikiWeb.AnalysisJobSupport.derive_job_context(%{
                 "model_version_id" => fixture.version["version_id"]
               })
    end)

    assert Store.list() == before_jobs
    Fixture.unchanged(fixture.execution)
  end

  test "security history keeps filters, summary and CSV projections", fixture do
    suffix = "?source=history-test&risk=low&action=READ-HISTORY&limit=1"
    listed = request(path("security_events", fixture) <> suffix)
    assert listed.status == 200
    assert Jason.decode!(listed.resp_body) == %{"events" => [fixture.event]}

    exported = request(path("security_json", fixture) <> suffix)
    assert exported.status == 200
    body = Jason.decode!(exported.resp_body)
    assert body["events"] == [fixture.event]

    assert body["summary"] == %{
             "total" => 1,
             "by_source" => %{"history-test" => 1},
             "by_risk" => %{"low" => 1},
             "by_status" => %{"allowed" => 1}
           }

    assert body["filters"]["source"] == "history-test"

    csv = request(path("security_csv", fixture) <> suffix)
    assert csv.status == 200
    assert get_resp_header(csv, "content-type") == ["text/csv; charset=utf-8"]
    assert length(String.split(String.trim(csv.resp_body), "\n")) == 2
    assert csv.resp_body =~ fixture.event["event_id"]

    excluded = request(path("security_events", fixture) <> "?source=another-source")
    assert Jason.decode!(excluded.resp_body) == %{"events" => []}
  end

  test "real empty and missing history keeps its successful or not-found response", fixture do
    Library.reset()
    SecurityEventStore.reset()

    for {name, key} <- [{"projects", "projects"}, {"models", "models"}, {"versions", "versions"}] do
      reply = request(path(name, fixture))
      assert reply.status == 200
      assert Jason.decode!(reply.resp_body) == %{key => []}
    end

    for {name, error} <- [
          {"project", "project_not_found"},
          {"model", "model_not_found"},
          {"version", "version_not_found"}
        ] do
      reply = request(path(name, fixture))
      assert reply.status == 404
      assert Jason.decode!(reply.resp_body) == %{"error" => error}
    end

    reply = request(path("model_checkpoint", fixture))
    assert reply.status == 200
    assert get_resp_header(reply, "cache-control") == ["no-store"]
    assert Jason.decode!(reply.resp_body) == %{"checkpoint" => %{"status" => "unknown"}}
    assert request(path("security_events", fixture)).resp_body == ~s({"events":[]})

    assert Jason.decode!(request(path("security_json", fixture)).resp_body)["summary"]["total"] ==
             0

    csv = request(path("security_csv", fixture))
    assert csv.status == 200
    assert get_resp_header(csv, "content-type") == ["text/csv; charset=utf-8"]
    assert length(String.split(String.trim(csv.resp_body), "\n")) == 1
  end

  defp library_child, do: Storage.repo_module() || KyuubikiWeb.Library.MemoryBackend
  defp security_child, do: Storage.repo_module() || KyuubikiWeb.SecurityEvents.MemoryBackend

  defp assert_unavailable(reply) do
    assert reply.status == 503
    assert Jason.decode!(reply.resp_body) == %{"error" => "analysis_store_unavailable"}
    assert get_resp_header(reply, "content-type") == ["application/json; charset=utf-8"]
    assert get_resp_header(reply, "content-disposition") == []
  end

  defp path("projects", _), do: "/api/v1/projects"
  defp path("project", f), do: "/api/v1/projects/#{f.project["project_id"]}"
  defp path("models", f), do: path("project", f) <> "/models"
  defp path("model", f), do: "/api/v1/models/#{f.model["model_id"]}"
  defp path("versions", f), do: path("model", f) <> "/versions"
  defp path("version", f), do: "/api/v1/model-versions/#{f.version["version_id"]}"

  defp path("model_checkpoint", f),
    do: "/api/v1/checkpoints/model/#{f.project["project_id"]}/#{@model_request}"

  defp path("version_checkpoint", f),
    do: "/api/v1/checkpoints/version/#{f.model["model_id"]}/#{@version_request}"

  defp path("security_events", _), do: "/api/v1/security-events"
  defp path("security_json", _), do: "/api/v1/export/security-events"
  defp path("security_csv", _), do: "/api/v1/export/security-events.csv"
  defp request(path), do: conn(:get, path) |> Router.call(@opts)
  defp stable(reply, "security_csv"), do: reply.resp_body
  defp stable(reply, _), do: reply.resp_body |> Jason.decode!() |> Map.delete("exported_at")
end
