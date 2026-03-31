defmodule KyuubikiWeb.Router do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.Library

  plug(Plug.Logger)
  plug(Plug.Parsers, parsers: [:json], pass: ["application/json"], json_decoder: Jason)

  plug(:match)
  plug(:dispatch)

  get "/" do
    respond_json(conn, 200, %{
      "service" => "kyuubiki-orchestrator",
      "ui" => "http://127.0.0.1:3000",
      "status" => "ok"
    })
  end

  get "/api/health" do
    respond_json(conn, 200, %{
      "service" => "kyuubiki-orchestrator",
      "status" => "ok",
      "transport" => %{
        "http" => 4000,
        "solver_agent_tcp" => 5001
      }
    })
  end

  post "/api/v1/fem/axial-bar/jobs" do
    case Analysis.submit_axial_bar(conn.body_params) do
      {:ok, payload} ->
        respond_json(conn, 202, payload)

      {:error, reason} ->
        respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/fem/truss-2d/jobs" do
    case Analysis.submit_truss_2d(conn.body_params) do
      {:ok, payload} ->
        respond_json(conn, 202, payload)

      {:error, reason} ->
        respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/fem/truss-3d/jobs" do
    case Analysis.submit_truss_3d(conn.body_params) do
      {:ok, payload} ->
        respond_json(conn, 202, payload)

      {:error, reason} ->
        respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/fem/plane-triangle-2d/jobs" do
    case Analysis.submit_plane_triangle_2d(conn.body_params) do
      {:ok, payload} ->
        respond_json(conn, 202, payload)

      {:error, reason} ->
        respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  get "/api/v1/jobs" do
    respond_json(conn, 200, Analysis.list_jobs())
  end

  get "/api/v1/jobs/:job_id" do
    case Analysis.fetch_job(job_id) do
      {:ok, payload} ->
        respond_json(conn, 200, payload)

      {:error, reason} ->
        respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  get "/api/v1/projects" do
    {:ok, projects} = Library.list_projects()
    respond_json(conn, 200, %{"projects" => projects})
  end

  post "/api/v1/projects" do
    case Library.create_project(conn.body_params) do
      {:ok, project} -> respond_json(conn, 201, %{"project" => project})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  get "/api/v1/projects/:project_id" do
    case Library.get_project(project_id) do
      {:ok, project} -> respond_json(conn, 200, %{"project" => project})
      :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
    end
  end

  patch "/api/v1/projects/:project_id" do
    case Library.update_project(project_id, conn.body_params) do
      {:ok, project} -> respond_json(conn, 200, %{"project" => project})
      :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  delete "/api/v1/projects/:project_id" do
    case Library.delete_project(project_id) do
      {:ok, project} -> respond_json(conn, 200, %{"project" => project})
      :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
    end
  end

  get "/api/v1/projects/:project_id/models" do
    case Library.list_models(project_id) do
      {:ok, models} -> respond_json(conn, 200, %{"models" => models})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/projects/:project_id/models" do
    case Library.create_model(project_id, conn.body_params) do
      {:ok, model} -> respond_json(conn, 201, %{"model" => model})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  get "/api/v1/models/:model_id" do
    case Library.get_model(model_id) do
      {:ok, model} -> respond_json(conn, 200, %{"model" => model})
      :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
    end
  end

  patch "/api/v1/models/:model_id" do
    case Library.update_model(model_id, conn.body_params) do
      {:ok, model} -> respond_json(conn, 200, %{"model" => model})
      :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  delete "/api/v1/models/:model_id" do
    case Library.delete_model(model_id) do
      {:ok, model} -> respond_json(conn, 200, %{"model" => model})
      :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
    end
  end

  get "/api/v1/models/:model_id/versions" do
    case Library.list_versions(model_id) do
      {:ok, versions} -> respond_json(conn, 200, %{"versions" => versions})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/models/:model_id/versions" do
    case Library.create_version(model_id, conn.body_params) do
      {:ok, version} -> respond_json(conn, 201, %{"version" => version})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  get "/api/v1/model-versions/:version_id" do
    case Library.get_version(version_id) do
      {:ok, version} -> respond_json(conn, 200, %{"version" => version})
      :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
    end
  end

  patch "/api/v1/model-versions/:version_id" do
    case Library.update_version(version_id, conn.body_params) do
      {:ok, version} -> respond_json(conn, 200, %{"version" => version})
      :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  delete "/api/v1/model-versions/:version_id" do
    case Library.delete_version(version_id) do
      {:ok, version} -> respond_json(conn, 200, %{"version" => version})
      :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
    end
  end

  post "/api/playground/run" do
    case Analysis.submit_axial_bar(conn.body_params) do
      {:ok, payload} -> respond_json(conn, 202, payload)
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  match _ do
    Plug.Conn.send_resp(conn, 404, "not found")
  end

  defp respond_json(conn, status, payload) do
    conn
    |> Plug.Conn.put_resp_content_type("application/json")
    |> Plug.Conn.send_resp(status, Jason.encode!(payload))
  end
end
