defmodule KyuubikiWeb.Api.JobAuditApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "lists persisted jobs in reverse chronological order" do
    {:ok, _job_1} =
      Store.create(%{
        job_id: "job-old",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    {:ok, _job_1_completed} =
      Store.apply_progress(%{
        job_id: "job-old",
        stage: "completed",
        progress: 1.0,
        iteration: 5,
        residual: 1.0e-3
      })

    :ok =
      AnalysisResultStore.put("job-old", %{
        "kind" => "axial_bar_1d",
        "max_displacement" => 1.0e-6
      })

    Process.sleep(5)

    {:ok, _job_2} =
      Store.create(%{
        job_id: "job-new",
        project_id: "project-2",
        simulation_case_id: "case-2"
      })

    {:ok, _job_2_updated} =
      Store.apply_progress(%{
        job_id: "job-new",
        stage: "solving",
        progress: 0.5,
        iteration: 2,
        residual: 5.0e-1
      })

    conn =
      :get
      |> conn("/api/v1/jobs")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    jobs = payload["jobs"]

    assert Enum.map(jobs, & &1["job_id"]) == ["job-new", "job-old"]

    assert Enum.at(jobs, 0)["status"] == "solving"
    assert Enum.at(jobs, 0)["has_result"] == false
    assert Enum.at(jobs, 1)["status"] == "completed"
    assert Enum.at(jobs, 1)["has_result"] == true
    assert is_binary(Enum.at(jobs, 0)["updated_at"])
    assert is_binary(Enum.at(jobs, 1)["created_at"])
  end

  test "supports CRUD and export for persisted jobs and results" do
    {:ok, _job} =
      Store.create(%{
        job_id: "job-admin",
        project_id: "project-admin",
        simulation_case_id: "case-admin",
        message: "queued"
      })

    :ok =
      AnalysisResultStore.put("job-admin", %{
        "kind" => "truss_2d",
        "max_displacement" => 2.0e-6
      })

    {:ok, _event_payload} =
      SecurityEventStore.create(%{
        "event_id" => "event-admin",
        "event_type" => "security_high_risk_action",
        "source" => "assistant",
        "action" => "data/exportDatabase",
        "risk" => "sensitive",
        "status" => "completed",
        "note" => "database export finished",
        "context" => %{"study_kind" => "truss_2d"},
        "occurred_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
      })

    update_job_conn =
      :patch
      |> conn("/api/v1/jobs/job-admin", Jason.encode!(%{"message" => "reviewed"}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_job_conn.status == 200
    assert Jason.decode!(update_job_conn.resp_body)["job"]["message"] == "reviewed"

    list_results_conn =
      :get
      |> conn("/api/v1/results")
      |> Router.call(@opts)

    assert list_results_conn.status == 200
    assert [%{"job_id" => "job-admin"}] = Jason.decode!(list_results_conn.resp_body)["results"]

    update_result_conn =
      :patch
      |> conn(
        "/api/v1/results/job-admin",
        Jason.encode!(%{"result" => %{"kind" => "truss_2d", "max_displacement" => 4.0e-6}})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_result_conn.status == 200
    assert Jason.decode!(update_result_conn.resp_body)["result"]["max_displacement"] == 4.0e-6

    export_conn =
      :get
      |> conn("/api/v1/export/database")
      |> Router.call(@opts)

    assert export_conn.status == 200
    export_payload = Jason.decode!(export_conn.resp_body)
    assert [%{"job_id" => "job-admin"}] = export_payload["jobs"]
    assert [%{"job_id" => "job-admin"}] = export_payload["results"]
    assert [%{"event_id" => "event-admin"}] = export_payload["security_events"]
    assert is_list(export_payload["projects"])
    assert is_list(export_payload["models"])
    assert is_list(export_payload["model_versions"])

    security_export_conn =
      :get
      |> conn("/api/v1/export/security-events?status=completed")
      |> Router.call(@opts)

    assert security_export_conn.status == 200
    security_export_payload = Jason.decode!(security_export_conn.resp_body)
    assert security_export_payload["schema"]["name"] == "kyuubiki.security-events.export/v1"
    assert security_export_payload["filters"]["status"] == "completed"
    assert security_export_payload["summary"]["total"] == 1

    assert [%{"event_id" => "event-admin", "status" => "completed"}] =
             security_export_payload["events"]

    security_export_csv_conn =
      :get
      |> conn("/api/v1/export/security-events.csv?status=completed")
      |> Router.call(@opts)

    assert security_export_csv_conn.status == 200

    assert get_resp_header(security_export_csv_conn, "content-type") == [
             "text/csv; charset=utf-8"
           ]

    assert String.contains?(
             security_export_csv_conn.resp_body,
             "event_id,event_type,source,action"
           )

    assert String.contains?(security_export_csv_conn.resp_body, "event-admin")

    delete_result_conn =
      :delete
      |> conn("/api/v1/results/job-admin")
      |> Router.call(@opts)

    assert delete_result_conn.status == 200

    delete_job_conn =
      :delete
      |> conn("/api/v1/jobs/job-admin")
      |> Router.call(@opts)

    assert delete_job_conn.status == 200

    missing_job_conn =
      :get
      |> conn("/api/v1/jobs/job-admin")
      |> Router.call(@opts)

    assert missing_job_conn.status == 422
  end

  test "supports append-only security event ingestion and listing" do
    create_conn =
      :post
      |> conn(
        "/api/v1/security-events",
        Jason.encode!(%{
          "event_id" => "event-1",
          "event_type" => "security_high_risk_action",
          "source" => "script",
          "action" => "project/deleteSelected",
          "risk" => "destructive",
          "status" => "cancelled",
          "note" => "operator cancelled confirmation",
          "context" => %{"project_id" => "proj-1", "study_kind" => "truss_3d"},
          "occurred_at" => "2026-04-29T08:00:00Z"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_conn.status == 201
    assert Jason.decode!(create_conn.resp_body)["event"]["action"] == "project/deleteSelected"

    list_conn =
      :get
      |> conn("/api/v1/security-events")
      |> Router.call(@opts)

    assert list_conn.status == 200

    assert [
             %{
               "event_id" => "event-1",
               "source" => "script",
               "risk" => "destructive",
               "status" => "cancelled"
             }
           ] = Jason.decode!(list_conn.resp_body)["events"]

    {:ok, _second_event} =
      SecurityEventStore.create(%{
        "event_id" => "event-2",
        "event_type" => "security_high_risk_action",
        "source" => "assistant",
        "action" => "data/exportDatabase",
        "risk" => "sensitive",
        "status" => "completed",
        "note" => "assistant finished export",
        "context" => %{"study_kind" => "truss_2d", "project_id" => "proj-2"},
        "occurred_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
      })

    filtered_conn =
      :get
      |> conn("/api/v1/security-events?source=assistant&risk=sensitive&action=export")
      |> Router.call(@opts)

    assert filtered_conn.status == 200

    assert [%{"event_id" => "event-2", "source" => "assistant"}] =
             Jason.decode!(filtered_conn.resp_body)["events"]

    window_filtered_conn =
      :get
      |> conn("/api/v1/security-events?occurred_after=2026-04-29T09:00:00Z")
      |> Router.call(@opts)

    assert window_filtered_conn.status == 200
    assert [%{"event_id" => "event-2"}] = Jason.decode!(window_filtered_conn.resp_body)["events"]
  end
end
