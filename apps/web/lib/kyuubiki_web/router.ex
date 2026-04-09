defmodule KyuubikiWeb.Router do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.Protocol
  alias KyuubikiWeb.Security

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
    agent_endpoints = KyuubikiWeb.Playground.AgentPool.endpoints()
    deployment = KyuubikiWeb.Playground.AgentPool.deployment_info()
    remote_registry = KyuubikiWeb.Playground.AgentRegistry.status_snapshot()
    watchdog = KyuubikiWeb.Jobs.Watchdog.status_snapshot()

    respond_json(conn, 200, %{
      "service" => "kyuubiki-orchestrator",
      "status" => "ok",
      "protocol" => Protocol.descriptor(),
      "security" => Security.descriptor(),
      "deployment" => deployment,
      "remote_solver_registry" => remote_registry,
      "watchdog" => watchdog,
      "transport" => %{
        "http" => 4000,
        "solver_agent_tcp" => (List.first(agent_endpoints) || %{})[:port] || 5001,
        "solver_agents" => agent_endpoints
      },
      "solver_agents" => agent_endpoints
    })
  end

  get "/api/v1/protocol" do
    respond_json(conn, 200, Protocol.descriptor())
  end

  get "/api/v1/protocol/control-plane" do
    respond_json(conn, 200, Protocol.control_plane_protocol())
  end

  get "/api/v1/protocol/solver-rpc" do
    respond_json(conn, 200, Protocol.solver_rpc_protocol())
  end

  get "/api/v1/protocol/agents" do
    respond_json(conn, 200, %{"agents" => Protocol.describe_agents()})
  end

  get "/api/v1/agents" do
    respond_json(conn, 200, %{
      "agents" => KyuubikiWeb.Playground.AgentRegistry.agents(),
      "summary" => KyuubikiWeb.Playground.AgentRegistry.status_snapshot()
    })
  end

  post "/api/v1/agents/register" do
    with_auth(conn, :cluster, fn conn ->
      case Security.validate_cluster_registration_identity(conn, conn.body_params) do
        :ok ->
          case KyuubikiWeb.Playground.AgentRegistry.register(conn.body_params) do
            {:ok, agent} ->
              _ = KyuubikiWeb.Playground.AgentPool.reload()
              respond_json(conn, 201, %{"agent" => agent})

            {:error, {:invalid_agent_field, field}} ->
              respond_json(conn, 422, %{"error" => "invalid_agent_field", "field" => field})

            {:error, reason} ->
              respond_json(conn, 422, %{"error" => inspect(reason)})
          end

        {:error, status, payload} ->
          respond_json(conn, status, payload)
      end
    end)
  end

  post "/api/v1/agents/:agent_id/heartbeat" do
    with_auth(conn, :cluster, fn conn ->
      case Security.validate_cluster_agent_identity(conn, agent_id, conn.body_params) do
        :ok ->
          case KyuubikiWeb.Playground.AgentRegistry.heartbeat(agent_id, conn.body_params) do
            {:ok, agent} ->
              _ = KyuubikiWeb.Playground.AgentPool.reload()
              respond_json(conn, 200, %{"agent" => agent})

            {:error, {:invalid_agent_field, field}} ->
              respond_json(conn, 422, %{"error" => "invalid_agent_field", "field" => field})

            {:error, reason} ->
              respond_json(conn, 422, %{"error" => inspect(reason)})
          end

        {:error, status, payload} ->
          respond_json(conn, status, payload)
      end
    end)
  end

  delete "/api/v1/agents/:agent_id" do
    with_auth(conn, :cluster, fn conn ->
      case Security.validate_cluster_agent_identity(conn, agent_id) do
        :ok ->
          :ok = KyuubikiWeb.Playground.AgentRegistry.unregister(agent_id)
          _ = KyuubikiWeb.Playground.AgentPool.reload()
          respond_json(conn, 200, %{"agent_id" => agent_id, "status" => "removed"})

        {:error, status, payload} ->
          respond_json(conn, status, payload)
      end
    end)
  end

  post "/api/v1/fem/axial-bar/jobs" do
    with_auth(conn, :write, fn conn ->
      case Analysis.submit_axial_bar(conn.body_params) do
        {:ok, payload} ->
          respond_json(conn, 202, payload)

        {:error, reason} ->
          respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  post "/api/v1/fem/truss-2d/jobs" do
    with_auth(conn, :write, fn conn ->
      case Analysis.submit_truss_2d(conn.body_params) do
        {:ok, payload} ->
          respond_json(conn, 202, payload)

        {:error, reason} ->
          respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  post "/api/v1/fem/truss-3d/jobs" do
    with_auth(conn, :write, fn conn ->
      case Analysis.submit_truss_3d(conn.body_params) do
        {:ok, payload} ->
          respond_json(conn, 202, payload)

        {:error, reason} ->
          respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  post "/api/v1/fem/plane-triangle-2d/jobs" do
    with_auth(conn, :write, fn conn ->
      case Analysis.submit_plane_triangle_2d(conn.body_params) do
        {:ok, payload} ->
          respond_json(conn, 202, payload)

        {:error, reason} ->
          respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
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

  patch "/api/v1/jobs/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Analysis.update_job(job_id, conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:job_not_found, _}} -> respond_json(conn, 404, %{"error" => "job_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  post "/api/v1/jobs/:job_id/cancel" do
    with_auth(conn, :write, fn conn ->
      case Analysis.cancel_job(job_id) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:job_not_found, _}} -> respond_json(conn, 404, %{"error" => "job_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  delete "/api/v1/jobs/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Analysis.delete_job(job_id) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:job_not_found, _}} -> respond_json(conn, 404, %{"error" => "job_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  get "/api/v1/results" do
    respond_json(conn, 200, Analysis.list_results())
  end

  get "/api/v1/results/:job_id" do
    case Analysis.fetch_result(job_id) do
      {:ok, payload} -> respond_json(conn, 200, payload)
      {:error, {:result_not_found, _}} -> respond_json(conn, 404, %{"error" => "result_not_found"})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  get "/api/v1/results/:job_id/chunks/:kind" do
    case Analysis.fetch_result_chunk(job_id, kind, conn.query_params) do
      {:ok, payload} -> respond_json(conn, 200, payload)
      {:error, {:result_not_found, _}} -> respond_json(conn, 404, %{"error" => "result_not_found"})
      {:error, {:unsupported_chunk_kind, _}} -> respond_json(conn, 422, %{"error" => "unsupported_chunk_kind"})
      {:error, {:invalid_chunk_param, key}} -> respond_json(conn, 422, %{"error" => "invalid_chunk_param", "field" => key})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  patch "/api/v1/results/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Map.get(conn.body_params, "result") do
        result when is_map(result) ->
          case Analysis.update_result(job_id, result) do
            {:ok, payload} -> respond_json(conn, 200, payload)
            {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
          end

        _ ->
          respond_json(conn, 422, %{"error" => "missing_result_payload"})
      end
    end)
  end

  delete "/api/v1/results/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Analysis.delete_result(job_id) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:result_not_found, _}} -> respond_json(conn, 404, %{"error" => "result_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  get "/api/v1/export/database" do
    respond_json(conn, 200, Analysis.export_database())
  end

  get "/api/v1/projects" do
    {:ok, projects} = Library.list_projects()
    respond_json(conn, 200, %{"projects" => projects})
  end

  post "/api/v1/projects" do
    with_auth(conn, :write, fn conn ->
      case Library.create_project(conn.body_params) do
        {:ok, project} -> respond_json(conn, 201, %{"project" => project})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  get "/api/v1/projects/:project_id" do
    case Library.get_project(project_id) do
      {:ok, project} -> respond_json(conn, 200, %{"project" => project})
      :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
    end
  end

  patch "/api/v1/projects/:project_id" do
    with_auth(conn, :write, fn conn ->
      case Library.update_project(project_id, conn.body_params) do
        {:ok, project} -> respond_json(conn, 200, %{"project" => project})
        :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  delete "/api/v1/projects/:project_id" do
    with_auth(conn, :write, fn conn ->
      case Library.delete_project(project_id) do
        {:ok, project} -> respond_json(conn, 200, %{"project" => project})
        :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
      end
    end)
  end

  get "/api/v1/projects/:project_id/models" do
    case Library.list_models(project_id) do
      {:ok, models} -> respond_json(conn, 200, %{"models" => models})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/projects/:project_id/models" do
    with_auth(conn, :write, fn conn ->
      case Library.create_model(project_id, conn.body_params) do
        {:ok, model} -> respond_json(conn, 201, %{"model" => model})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  get "/api/v1/models/:model_id" do
    case Library.get_model(model_id) do
      {:ok, model} -> respond_json(conn, 200, %{"model" => model})
      :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
    end
  end

  patch "/api/v1/models/:model_id" do
    with_auth(conn, :write, fn conn ->
      case Library.update_model(model_id, conn.body_params) do
        {:ok, model} -> respond_json(conn, 200, %{"model" => model})
        :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  delete "/api/v1/models/:model_id" do
    with_auth(conn, :write, fn conn ->
      case Library.delete_model(model_id) do
        {:ok, model} -> respond_json(conn, 200, %{"model" => model})
        :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
      end
    end)
  end

  get "/api/v1/models/:model_id/versions" do
    case Library.list_versions(model_id) do
      {:ok, versions} -> respond_json(conn, 200, %{"versions" => versions})
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/v1/models/:model_id/versions" do
    with_auth(conn, :write, fn conn ->
      case Library.create_version(model_id, conn.body_params) do
        {:ok, version} -> respond_json(conn, 201, %{"version" => version})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  get "/api/v1/model-versions/:version_id" do
    case Library.get_version(version_id) do
      {:ok, version} -> respond_json(conn, 200, %{"version" => version})
      :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
    end
  end

  patch "/api/v1/model-versions/:version_id" do
    with_auth(conn, :write, fn conn ->
      case Library.update_version(version_id, conn.body_params) do
        {:ok, version} -> respond_json(conn, 200, %{"version" => version})
        :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  delete "/api/v1/model-versions/:version_id" do
    with_auth(conn, :write, fn conn ->
      case Library.delete_version(version_id) do
        {:ok, version} -> respond_json(conn, 200, %{"version" => version})
        :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
      end
    end)
  end

  post "/api/playground/run" do
    with_auth(conn, :write, fn conn ->
      case Analysis.submit_axial_bar(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 202, payload)
        {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
      end
    end)
  end

  match _ do
    Plug.Conn.send_resp(conn, 404, "not found")
  end

  defp respond_json(conn, status, payload) do
    conn
    |> Plug.Conn.put_resp_content_type("application/json")
    |> Plug.Conn.send_resp(status, Jason.encode!(payload))
  end

  defp with_auth(conn, scope, fun) do
    case Security.authorize(conn, scope) do
      :ok ->
        fun.(conn)

      {:error, status, payload} ->
        respond_json(conn, status, payload)
    end
  end
end
