defmodule KyuubikiWeb.Router do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.AssetStore
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.Protocol
  alias KyuubikiWeb.Security
  alias KyuubikiWeb.Workloads
  import KyuubikiWeb.RouterSupport

  @fem_submit_routes [
    {"/api/v1/fem/axial-bar/jobs", :submit_axial_bar},
    {"/api/v1/fem/thermal-bar-1d/jobs", :submit_thermal_bar_1d},
    {"/api/v1/fem/heat-bar-1d/jobs", :submit_heat_bar_1d},
    {"/api/v1/fem/electrostatic-bar-1d/jobs", :submit_electrostatic_bar_1d},
    {"/api/v1/fem/magnetostatic-bar-1d/jobs", :submit_magnetostatic_bar_1d},
    {"/api/v1/fem/electrostatic-plane-triangle-2d/jobs", :submit_electrostatic_plane_triangle_2d},
    {"/api/v1/fem/electrostatic-plane-quad-2d/jobs", :submit_electrostatic_plane_quad_2d},
    {"/api/v1/fem/magnetostatic-plane-triangle-2d/jobs", :submit_magnetostatic_plane_triangle_2d},
    {"/api/v1/fem/magnetostatic-plane-quad-2d/jobs", :submit_magnetostatic_plane_quad_2d},
    {"/api/v1/fem/heat-plane-triangle-2d/jobs", :submit_heat_plane_triangle_2d},
    {"/api/v1/fem/heat-plane-quad-2d/jobs", :submit_heat_plane_quad_2d},
    {"/api/v1/fem/thermal-truss-2d/jobs", :submit_thermal_truss_2d},
    {"/api/v1/fem/thermal-truss-3d/jobs", :submit_thermal_truss_3d},
    {"/api/v1/fem/beam-1d/jobs", :submit_beam_1d},
    {"/api/v1/fem/thermal-plane-triangle-2d/jobs", :submit_thermal_plane_triangle_2d},
    {"/api/v1/fem/thermal-plane-quad-2d/jobs", :submit_thermal_plane_quad_2d},
    {"/api/v1/fem/thermal-beam-1d/jobs", :submit_thermal_beam_1d},
    {"/api/v1/fem/thermal-frame-2d/jobs", :submit_thermal_frame_2d},
    {"/api/v1/fem/thermal-frame-3d/jobs", :submit_thermal_frame_3d},
    {"/api/v1/fem/torsion-1d/jobs", :submit_torsion_1d},
    {"/api/v1/fem/spring-1d/jobs", :submit_spring_1d},
    {"/api/v1/fem/spring-2d/jobs", :submit_spring_2d},
    {"/api/v1/fem/spring-3d/jobs", :submit_spring_3d},
    {"/api/v1/fem/truss-2d/jobs", :submit_truss_2d},
    {"/api/v1/fem/truss-3d/jobs", :submit_truss_3d},
    {"/api/v1/fem/plane-triangle-2d/jobs", :submit_plane_triangle_2d},
    {"/api/v1/fem/plane-quad-2d/jobs", :submit_plane_quad_2d},
    {"/api/v1/fem/frame-2d/jobs", :submit_frame_2d},
    {"/api/v1/fem/frame-3d/jobs", :submit_frame_3d}
  ]

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
    with_auth(conn, :read, fn conn ->
      agent_endpoints = KyuubikiWeb.Playground.AgentPool.endpoints()

      respond_json(conn, 200, %{
        "service" => "kyuubiki-orchestrator",
        "status" => "ok",
        "protocol" => Protocol.descriptor(),
        "security" => Security.descriptor(),
        "deployment" => KyuubikiWeb.Playground.AgentPool.deployment_info(),
        "remote_solver_registry" => KyuubikiWeb.Playground.AgentRegistry.status_snapshot(),
        "watchdog" => KyuubikiWeb.Jobs.Watchdog.status_snapshot(),
        "transport" => %{
          "http" => 4000,
          "solver_agent_tcp" => (List.first(agent_endpoints) || %{})[:port] || 5001,
          "solver_agents" => agent_endpoints
        },
        "solver_agents" => agent_endpoints
      })
    end)
  end

  get "/api/v1/protocol" do
    with_auth(conn, :read, fn conn -> respond_json(conn, 200, Protocol.descriptor()) end)
  end

  get "/api/v1/protocol/control-plane" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, Protocol.control_plane_protocol())
    end)
  end

  get "/api/v1/protocol/solver-rpc" do
    with_auth(conn, :read, fn conn -> respond_json(conn, 200, Protocol.solver_rpc_protocol()) end)
  end

  get "/api/v1/protocol/agents" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, %{"agents" => Protocol.describe_agents()})
    end)
  end

  get "/api/v1/agents" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, %{
        "agents" => KyuubikiWeb.Playground.AgentRegistry.public_agents(),
        "summary" => KyuubikiWeb.Playground.AgentRegistry.status_snapshot()
      })
    end)
  end

  post "/api/v1/agents/register" do
    with_auth(conn, :cluster, fn conn ->
      body_params = with_cluster_fingerprint(conn, conn.body_params)

      case Security.validate_cluster_registration_identity(conn, body_params) do
        :ok ->
          case KyuubikiWeb.Playground.AgentRegistry.register(body_params) do
            {:ok, agent} ->
              _ = KyuubikiWeb.Playground.AgentPool.reload()

              respond_json(conn, 201, %{
                "agent" => KyuubikiWeb.Playground.AgentRegistry.public_agent(agent)
              })

            {:error, {:invalid_agent_field, field}} ->
              respond_json(conn, 422, %{"error" => "invalid_agent_field", "field" => field})

            {:error, reason} ->
              unprocessable(conn, reason)
          end

        {:error, status, payload} ->
          respond_json(conn, status, payload)
      end
    end)
  end

  post "/api/v1/agents/:agent_id/heartbeat" do
    with_auth(conn, :cluster, fn conn ->
      body_params = with_cluster_fingerprint(conn, conn.body_params)

      with :ok <- Security.validate_cluster_agent_identity(conn, agent_id, body_params),
           :ok <- validate_registered_fingerprint(conn, agent_id),
           {:ok, agent} <- KyuubikiWeb.Playground.AgentRegistry.heartbeat(agent_id, body_params) do
        _ = KyuubikiWeb.Playground.AgentPool.reload()

        respond_json(conn, 200, %{
          "agent" => KyuubikiWeb.Playground.AgentRegistry.public_agent(agent)
        })
      else
        {:error, {:invalid_agent_field, field}} ->
          respond_json(conn, 422, %{"error" => "invalid_agent_field", "field" => field})

        {:error, status, payload} when is_integer(status) ->
          respond_json(conn, status, payload)

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  delete "/api/v1/agents/:agent_id" do
    with_auth(conn, :cluster, fn conn ->
      with :ok <- Security.validate_cluster_agent_identity(conn, agent_id),
           :ok <- validate_registered_fingerprint(conn, agent_id) do
        :ok = KyuubikiWeb.Playground.AgentRegistry.unregister(agent_id)
        _ = KyuubikiWeb.Playground.AgentPool.reload()
        respond_json(conn, 200, %{"agent_id" => agent_id, "status" => "removed"})
      else
        {:error, status, payload} -> respond_json(conn, status, payload)
      end
    end)
  end

  for {path, submit_fun} <- @fem_submit_routes do
    post path do
      with_auth(conn, :write, fn conn ->
        conn
        |> run_submit(unquote(submit_fun), conn.body_params)
        |> respond_submit()
      end)
    end
  end

  post "/api/v1/workflows/graph/run" do
    with_auth(conn, :write, fn conn ->
      conn
      |> run_analysis(:run_workflow_graph, [conn.body_params])
      |> respond_success(200)
    end)
  end

  post "/api/v1/workflows/graph/jobs" do
    with_auth(conn, :write, fn conn ->
      conn
      |> run_submit(:submit_workflow_graph, conn.body_params)
      |> respond_submit()
    end)
  end

  get "/api/v1/workflows/catalog" do
    with_auth(conn, :read, fn conn ->
      conn = fetch_query_params(conn)
      respond_json(conn, 200, Analysis.list_workflow_catalog(conn.query_params))
    end)
  end

  get "/api/v1/workflows/catalog/:workflow_id" do
    with_auth(conn, :read, fn conn ->
      case Analysis.fetch_workflow_catalog_entry(workflow_id) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:workflow_not_found, _}} ->
          respond_json(conn, 404, %{"error" => "workflow_not_found", "workflow_id" => workflow_id})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  post "/api/v1/workflows/catalog/:workflow_id/jobs" do
    with_auth(conn, :write, fn conn ->
      case Analysis.submit_catalog_workflow(workflow_id, conn.body_params) do
        {:ok, payload} ->
          respond_json(conn, 202, payload)

        {:error, {:workflow_not_found, _}} ->
          respond_json(conn, 404, %{"error" => "workflow_not_found", "workflow_id" => workflow_id})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/operators" do
    with_auth(conn, :read, fn conn ->
      conn = fetch_query_params(conn)
      respond_json(conn, 200, Analysis.list_operator_catalog(conn.query_params))
    end)
  end

  get "/api/v1/operators/:operator_id" do
    with_auth(conn, :read, fn conn ->
      case Analysis.fetch_operator_catalog_entry(operator_id) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:operator_not_found, _}} ->
          respond_json(conn, 404, %{"error" => "operator_not_found", "operator_id" => operator_id})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/store" do
    with_auth(conn, :read, fn conn ->
      conn = fetch_query_params(conn)
      respond_json(conn, 200, AssetStore.catalog(conn.query_params))
    end)
  end

  get "/api/v1/store/sources" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, %{"sources" => AssetStore.sources()})
    end)
  end

  get "/api/v1/store/:kind/:entry_id" do
    with_auth(conn, :read, fn conn ->
      case AssetStore.fetch(kind, entry_id) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:store_entry_not_found, _kind, _id}} ->
          respond_json(conn, 404, %{
            "error" => "store_entry_not_found",
            "kind" => kind,
            "id" => entry_id
          })

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/jobs" do
    with_auth(conn, :read, fn conn -> respond_json(conn, 200, Analysis.list_jobs()) end)
  end

  get "/api/v1/jobs/:job_id" do
    with_auth(conn, :read, fn conn ->
      conn |> run_analysis(:fetch_job, [job_id]) |> respond_success(200)
    end)
  end

  patch "/api/v1/jobs/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Analysis.update_job(job_id, conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:job_not_found, _}} -> respond_json(conn, 404, %{"error" => "job_not_found"})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/api/v1/jobs/:job_id/cancel" do
    with_auth(conn, :write, fn conn ->
      case Analysis.cancel_job(job_id) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:job_not_found, _}} -> respond_json(conn, 404, %{"error" => "job_not_found"})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  delete "/api/v1/jobs/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Analysis.delete_job(job_id) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, {:job_not_found, _}} -> respond_json(conn, 404, %{"error" => "job_not_found"})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/results" do
    with_auth(conn, :read, fn conn -> respond_json(conn, 200, Analysis.list_results()) end)
  end

  get "/api/v1/results/:job_id" do
    with_auth(conn, :read, fn conn ->
      case Analysis.fetch_result(job_id) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:result_not_found, _}} ->
          respond_json(conn, 404, %{"error" => "result_not_found"})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/results/:job_id/chunks/:kind" do
    with_auth(conn, :read, fn conn ->
      case Analysis.fetch_result_chunk(job_id, kind, conn.query_params) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:result_not_found, _}} ->
          respond_json(conn, 404, %{"error" => "result_not_found"})

        {:error, {:unsupported_chunk_kind, _}} ->
          respond_json(conn, 422, %{"error" => "unsupported_chunk_kind"})

        {:error, {:invalid_chunk_param, key}} ->
          respond_json(conn, 422, %{"error" => "invalid_chunk_param", "field" => key})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  patch "/api/v1/results/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Map.get(conn.body_params, "result") do
        result when is_map(result) ->
          conn |> run_analysis(:update_result, [job_id, result]) |> respond_success(200)

        _ ->
          respond_json(conn, 422, %{"error" => "missing_result_payload"})
      end
    end)
  end

  delete "/api/v1/results/:job_id" do
    with_auth(conn, :write, fn conn ->
      case Analysis.delete_result(job_id) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:result_not_found, _}} ->
          respond_json(conn, 404, %{"error" => "result_not_found"})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/export/database" do
    with_auth(conn, :read, fn conn -> respond_json(conn, 200, Analysis.export_database()) end)
  end

  get "/api/v1/export/security-events" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, Analysis.export_security_events(conn.query_params))
    end)
  end

  get "/api/v1/export/security-events.csv" do
    with_auth(conn, :read, fn conn ->
      respond_text(conn, 200, "text/csv", Analysis.export_security_events_csv(conn.query_params))
    end)
  end

  get "/api/v1/security-events" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, Analysis.list_security_events(conn.query_params))
    end)
  end

  get "/api/v1/workloads/catalog" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, Workloads.workload_catalog(request_base_url(conn)))
    end)
  end

  post "/api/v1/security-events" do
    with_auth(conn, :write, fn conn ->
      case Analysis.create_security_event(conn.body_params) do
        {:ok, payload} ->
          respond_json(conn, 201, payload)

        {:error, {:invalid_security_event_field, field}} ->
          respond_json(conn, 422, %{"error" => "invalid_security_event_field", "field" => field})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/projects" do
    with_auth(conn, :read, fn conn ->
      {:ok, projects} = Library.list_projects()
      respond_json(conn, 200, %{"projects" => projects})
    end)
  end

  post "/api/v1/projects" do
    with_auth(conn, :write, fn conn ->
      case Library.create_project(conn.body_params) do
        {:ok, project} -> respond_json(conn, 201, %{"project" => project})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/projects/:project_id" do
    with_auth(conn, :read, fn conn ->
      case Library.get_project(project_id) do
        {:ok, project} -> respond_json(conn, 200, %{"project" => project})
        :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
      end
    end)
  end

  get "/api/v1/projects/:project_id/bundle" do
    with_auth(conn, :read, fn conn ->
      case Workloads.export_project_bundle(project_id) do
        {:ok, bundle} ->
          filename = Workloads.bundle_filename(bundle["project"])

          conn
          |> Plug.Conn.put_resp_header(
            "content-disposition",
            ~s(attachment; filename="#{filename}")
          )
          |> respond_json(200, bundle)

        {:error, :project_not_found} ->
          respond_json(conn, 404, %{"error" => "project_not_found"})

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  patch "/api/v1/projects/:project_id" do
    with_auth(conn, :write, fn conn ->
      case Library.update_project(project_id, conn.body_params) do
        {:ok, project} -> respond_json(conn, 200, %{"project" => project})
        :error -> respond_json(conn, 404, %{"error" => "project_not_found"})
        {:error, reason} -> unprocessable(conn, reason)
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
    with_auth(conn, :read, fn conn ->
      case Library.list_models(project_id) do
        {:ok, models} -> respond_json(conn, 200, %{"models" => models})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/api/v1/projects/:project_id/models" do
    with_auth(conn, :write, fn conn ->
      case Library.create_model(project_id, conn.body_params) do
        {:ok, model} -> respond_json(conn, 201, %{"model" => model})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/models/:model_id" do
    with_auth(conn, :read, fn conn ->
      case Library.get_model(model_id) do
        {:ok, model} -> respond_json(conn, 200, %{"model" => model})
        :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
      end
    end)
  end

  patch "/api/v1/models/:model_id" do
    with_auth(conn, :write, fn conn ->
      case Library.update_model(model_id, conn.body_params) do
        {:ok, model} -> respond_json(conn, 200, %{"model" => model})
        :error -> respond_json(conn, 404, %{"error" => "model_not_found"})
        {:error, reason} -> unprocessable(conn, reason)
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
    with_auth(conn, :read, fn conn ->
      case Library.list_versions(model_id) do
        {:ok, versions} -> respond_json(conn, 200, %{"versions" => versions})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/api/v1/models/:model_id/versions" do
    with_auth(conn, :write, fn conn ->
      case Library.create_version(model_id, conn.body_params) do
        {:ok, version} -> respond_json(conn, 201, %{"version" => version})
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  get "/api/v1/model-versions/:version_id" do
    with_auth(conn, :read, fn conn ->
      case Library.get_version(version_id) do
        {:ok, version} -> respond_json(conn, 200, %{"version" => version})
        :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
      end
    end)
  end

  patch "/api/v1/model-versions/:version_id" do
    with_auth(conn, :write, fn conn ->
      case Library.update_version(version_id, conn.body_params) do
        {:ok, version} -> respond_json(conn, 200, %{"version" => version})
        :error -> respond_json(conn, 404, %{"error" => "version_not_found"})
        {:error, reason} -> unprocessable(conn, reason)
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
      conn
      |> run_submit(:submit_axial_bar, conn.body_params)
      |> respond_submit()
    end)
  end

  match _ do
    Plug.Conn.send_resp(conn, 404, "not found")
  end

  defp run_submit(conn, submit_fun, params) do
    {conn, apply(Analysis, submit_fun, [params])}
  end

  defp run_analysis(conn, fun, args) do
    {conn, apply(Analysis, fun, args)}
  end

  defp respond_submit({conn, {:ok, payload}}), do: respond_json(conn, 202, payload)
  defp respond_submit({conn, {:error, reason}}), do: unprocessable(conn, reason)

  defp respond_success({conn, {:ok, payload}}, status), do: respond_json(conn, status, payload)
  defp respond_success({conn, {:error, reason}}, _status), do: unprocessable(conn, reason)
end
