defmodule KyuubikiWeb.Analysis do
  @moduledoc """
  Asynchronous orchestration for FEM study jobs.
  """

  alias KyuubikiWeb.AnalysisExports
  alias KyuubikiWeb.AnalysisJobRecords
  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.FemModelNormalizer
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowGraphResponse
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime
  alias KyuubikiWeb.WorkflowTemplateCatalog

  @spec submit_axial_bar(map()) :: {:ok, map()} | {:error, term()}
  def submit_axial_bar(params) when is_map(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_axial_bar/1, "solve_bar_1d")

  @spec submit_thermal_bar_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_bar_1d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_bar_1d/1,
        "solve_thermal_bar_1d"
      )

  @spec submit_heat_bar_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_heat_bar_1d(params) when is_map(params),
    do:
      submit_solver_job(params, &FemModelNormalizer.normalize_heat_bar_1d/1, "solve_heat_bar_1d")

  @spec submit_electrostatic_bar_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_electrostatic_bar_1d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_bar_1d/1,
        "solve_electrostatic_bar_1d"
      )

  @spec submit_electrostatic_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_electrostatic_plane_triangle_2d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_plane_triangle_2d/1,
        "solve_electrostatic_plane_triangle_2d"
      )

  @spec submit_electrostatic_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_electrostatic_plane_quad_2d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_plane_quad_2d/1,
        "solve_electrostatic_plane_quad_2d"
      )

  @spec submit_heat_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_heat_plane_triangle_2d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_heat_plane_triangle_2d/1,
        "solve_heat_plane_triangle_2d"
      )

  @spec submit_heat_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_heat_plane_quad_2d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_heat_plane_quad_2d/1,
        "solve_heat_plane_quad_2d"
      )

  @spec submit_thermal_truss_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_truss_2d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_truss_2d/1,
        "solve_thermal_truss_2d"
      )

  @spec submit_thermal_truss_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_truss_3d(params) when is_map(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_truss_3d/1,
        "solve_thermal_truss_3d"
      )

  @spec submit_beam_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_beam_1d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_beam_1d/1, "solve_beam_1d")
  end

  @spec submit_thermal_beam_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_beam_1d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_thermal_beam_1d/1,
      "solve_thermal_beam_1d"
    )
  end

  @spec submit_torsion_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_torsion_1d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_torsion_1d/1, "solve_torsion_1d")
  end

  @spec submit_spring_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_spring_1d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_spring_1d/1, "solve_spring_1d")
  end

  @spec submit_spring_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_spring_2d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_spring_2d/1, "solve_spring_2d")
  end

  @spec submit_spring_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_spring_3d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_spring_3d/1, "solve_spring_3d")
  end

  @spec submit_truss_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_truss_2d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_truss_2d/1, "solve_truss_2d")
  end

  @spec submit_truss_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_truss_3d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_truss_3d/1, "solve_truss_3d")
  end

  @spec submit_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_plane_triangle_2d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_plane_triangle_2d/1,
      "solve_plane_triangle_2d"
    )
  end

  @spec submit_thermal_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_plane_triangle_2d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_thermal_plane_triangle_2d/1,
      "solve_thermal_plane_triangle_2d"
    )
  end

  @spec submit_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_plane_quad_2d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_plane_quad_2d/1,
      "solve_plane_quad_2d"
    )
  end

  @spec submit_thermal_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_plane_quad_2d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_thermal_plane_quad_2d/1,
      "solve_thermal_plane_quad_2d"
    )
  end

  @spec submit_frame_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_frame_2d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_frame_2d/1, "solve_frame_2d")
  end

  @spec submit_frame_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_frame_3d(params) when is_map(params) do
    submit_solver_job(params, &FemModelNormalizer.normalize_frame_3d/1, "solve_frame_3d")
  end

  @spec submit_thermal_frame_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_frame_2d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_thermal_frame_2d/1,
      "solve_thermal_frame_2d"
    )
  end

  @spec submit_thermal_frame_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_frame_3d(params) when is_map(params) do
    submit_solver_job(
      params,
      &FemModelNormalizer.normalize_thermal_frame_3d/1,
      "solve_thermal_frame_3d"
    )
  end

  defp submit_solver_job(params, normalizer, method)
       when is_map(params) and is_function(normalizer, 1) and is_binary(method) do
    with {:ok, normalized} <- normalizer.(params),
         {:ok, job_context} <- AnalysisJobSupport.derive_job_context(params),
         {:ok, job} <- AnalysisJobSupport.create_job(job_context) do
      start_background_job(
        job.job_id,
        method,
        normalized,
        orchestration_context_from_params(params)
      )

      {:ok, AnalysisJobSupport.serialize_payload(job)}
    end
  end

  @spec list_workflow_catalog(map()) :: map()
  def list_workflow_catalog(filters \\ %{}) when is_map(filters),
    do: %{"workflows" => WorkflowTemplateCatalog.list(filters)}

  @spec list_operator_catalog(map()) :: map()
  def list_operator_catalog(filters \\ %{}) when is_map(filters),
    do: %{"operators" => WorkflowOperatorCatalog.list(filters)}

  @spec fetch_workflow_catalog_entry(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_workflow_catalog_entry(workflow_id) when is_binary(workflow_id),
    do: WorkflowTemplateCatalog.fetch(workflow_id)

  @spec fetch_operator_catalog_entry(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_operator_catalog_entry(operator_id) when is_binary(operator_id),
    do: WorkflowOperatorCatalog.fetch(operator_id)

  @spec run_workflow_graph(map()) :: {:ok, map()} | {:error, term()}
  def run_workflow_graph(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    with %{} = graph <- Map.get(normalized, "graph"),
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         {:ok, result} <- execute_workflow_graph(graph, input_artifacts) do
      {:ok, WorkflowGraphResponse.shape(graph, result, response_options)}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  @spec submit_catalog_workflow(String.t(), map()) :: {:ok, map()} | {:error, term()}
  def submit_catalog_workflow(workflow_id, params)
      when is_binary(workflow_id) and is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    with {:ok, graph} <- WorkflowTemplateCatalog.graph_by_id(workflow_id),
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         {:ok, payload} <-
           submit_workflow_graph(%{
             "graph" => graph,
             "input_artifacts" => input_artifacts,
             "response_options" => response_options,
             "tags" => Map.get(normalized, "tags", []),
             "requested_agent_id" => Map.get(normalized, "requested_agent_id"),
             "requested_capability" => Map.get(normalized, "requested_capability"),
             "control_mode" => Map.get(normalized, "control_mode"),
             "orch_id" => Map.get(normalized, "orch_id"),
             "orch_session_id" => Map.get(normalized, "orch_session_id")
           }) do
      {:ok, payload}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  @spec submit_workflow_graph(map()) :: {:ok, map()} | {:error, term()}
  def submit_workflow_graph(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    with %{} = graph <- Map.get(normalized, "graph"),
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         {:ok, job_context} <- AnalysisJobSupport.derive_job_context(params),
         {:ok, job} <- AnalysisJobSupport.create_job(job_context) do
      orchestration_context = orchestration_context_from_params(params)

      :ok =
        AnalysisResultStore.put(job.job_id, %{
          "workflow_id" => Map.get(graph, "id"),
          "current_node" => nil,
          "progress_events" => [],
          "completed_nodes" => [],
          "artifacts" => %{},
          "response_options" => response_options,
          "orchestration_context" => orchestration_context
        })

      start_background_workflow_job(
        job.job_id,
        graph,
        input_artifacts,
        orchestration_context,
        response_options
      )
      {:ok, AnalysisJobSupport.serialize_payload(job)}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  @spec fetch_job(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_job(job_id) when is_binary(job_id), do: AnalysisJobRecords.fetch_job(job_id)

  @spec list_jobs() :: map()
  def list_jobs, do: AnalysisJobRecords.list_jobs()

  def update_job(job_id, attrs) when is_binary(job_id) and is_map(attrs),
    do: AnalysisJobRecords.update_job(job_id, attrs)

  def cancel_job(job_id) when is_binary(job_id), do: AnalysisJobRecords.cancel_job(job_id)

  def delete_job(job_id) when is_binary(job_id), do: AnalysisJobRecords.delete_job(job_id)

  def list_results, do: AnalysisJobRecords.list_results()

  def fetch_result(job_id) when is_binary(job_id), do: AnalysisJobRecords.fetch_result(job_id)

  def fetch_result_chunk(job_id, kind, params \\ %{})
      when is_binary(job_id) and is_binary(kind) and is_map(params) do
    AnalysisJobRecords.fetch_result_chunk(job_id, kind, params)
  end

  def update_result(job_id, result) when is_binary(job_id) and is_map(result),
    do: AnalysisJobRecords.update_result(job_id, result)

  def delete_result(job_id) when is_binary(job_id), do: AnalysisJobRecords.delete_result(job_id)

  def create_security_event(attrs) when is_map(attrs),
    do: AnalysisExports.create_security_event(attrs)

  def list_security_events(filters \\ %{}) when is_map(filters),
    do: AnalysisExports.list_security_events(filters)

  def export_security_events(filters \\ %{}) when is_map(filters),
    do: AnalysisExports.export_security_events(filters)

  def export_security_events_csv(filters \\ %{}) when is_map(filters),
    do: AnalysisExports.export_security_events_csv(filters)

  def export_database, do: AnalysisExports.export_database()

  defp execute_workflow_graph(graph, input_artifacts) do
    execute_workflow_graph(graph, input_artifacts, nil, %{})
  end

  defp execute_workflow_graph(
         %{} = graph,
         input_artifacts,
         progress_callback,
         orchestration_context
       )
       when is_map(input_artifacts) and is_map(orchestration_context) and
              (is_nil(progress_callback) or is_function(progress_callback, 1)) do
    WorkflowGraphRunner.run(
      graph,
      input_artifacts,
      dataset_contract: Map.get(graph, "dataset_contract"),
      progress_callback: progress_callback,
      execute_solve: fn operator_id, payload, node ->
        WorkflowOperatorRuntime.run_solve_operator(
          operator_id,
          payload,
          Map.put(node, "orchestration_context", orchestration_context)
        )
      end,
      execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
      execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
      execute_export: &WorkflowOperatorRuntime.run_export_operator/3
    )
  end

  defp execute_workflow_graph(
         _graph,
         _input_artifacts,
         _progress_callback,
         _orchestration_context
       ),
       do: {:error, :invalid_workflow_graph}

  defp start_background_job(job_id, method, params, orchestration_context) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      execute_background_job(job_id, method, params, orchestration_context)
    end)
  end

  defp start_background_workflow_job(
         job_id,
         graph,
         input_artifacts,
         orchestration_context,
         response_options
       ) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      execute_background_workflow_job(
        job_id,
        graph,
        input_artifacts,
        orchestration_context,
        response_options
      )
    end)
  end

  defp apply_agent_progress(job_id, progress) when is_binary(job_id) and is_map(progress) do
    case Store.get(job_id) do
      {:ok, %{status: :cancelled}} ->
        :ok

      {:ok, _job} ->
        attrs =
          progress
          |> Map.take(["stage", "progress", "residual", "iteration", "peak_memory", "message"])
          |> Enum.into(%{}, fn {key, value} -> {String.to_atom(key), value} end)
          |> Map.put(:job_id, job_id)

        _ = Store.apply_progress(attrs)
        :ok

      :error ->
        :ok
    end
  end

  defp execute_background_job(job_id, method, params, orchestration_context) do
    timeout_ms = watchdog_job_timeout_ms()

    task =
      Task.async(fn ->
        AgentClient.request_with_agent(
          method,
          params,
          &apply_agent_progress(job_id, &1),
          orchestration: orchestration_context,
          job_id: job_id
        )
      end)

    case Task.yield(task, timeout_ms) || Task.shutdown(task, :brutal_kill) do
      {:ok, {:ok, result, endpoint}} ->
        unless cancelled?(job_id) do
          {:ok, _job} = Store.assign_worker(job_id, AgentClient.worker_id(endpoint))
          :ok = AnalysisResultStore.put(job_id, result)
          _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})
        end

      {:ok, {:error, {:rpc_error, "cancelled", message}}} ->
        cancel_job_with_message(job_id, message)

      {:ok, {:error, reason}} ->
        unless cancelled?(job_id) do
          fail_job(job_id, inspect(reason))
        end

      nil ->
        unless cancelled?(job_id) do
          fail_job(job_id, "job execution timed out after #{timeout_ms} ms")
        end
    end
  end

  defp execute_background_workflow_job(
         job_id,
         graph,
         input_artifacts,
         orchestration_context,
         response_options
       ) do
    timeout_ms = watchdog_job_timeout_ms()

    task =
      Task.async(fn ->
        execute_workflow_graph(
          graph,
          input_artifacts,
          fn progress ->
            apply_workflow_progress(job_id, progress)
          end,
          orchestration_context
        )
      end)

    case Task.yield(task, timeout_ms) || Task.shutdown(task, :brutal_kill) do
      {:ok, {:ok, result}} ->
        unless cancelled?(job_id) do
          shaped_result = WorkflowGraphResponse.shape(graph, result, response_options)

          :ok =
            AnalysisResultStore.put(
              job_id,
              shaped_result
              |> Map.put("workflow_id", Map.get(graph, "id"))
              |> Map.put("current_node", nil)
              |> Map.put("progress_events", workflow_progress_events(job_id))
              |> Map.put("response_options", response_options)
            )

          _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})
        end

      {:ok, {:error, {:workflow_cancelled, _node_id}}} ->
        cancel_job_with_message(job_id, "workflow cancelled by operator")

      {:ok, {:error, reason}} ->
        unless cancelled?(job_id) do
          fail_job(job_id, inspect(reason))
        end

      nil ->
        unless cancelled?(job_id) do
          fail_job(job_id, "workflow execution timed out after #{timeout_ms} ms")
        end
    end
  end

  defp apply_workflow_progress(job_id, %{
         "node_id" => node_id,
         "completed_nodes" => completed_nodes,
         "total_nodes" => total_nodes
       })
       when is_binary(job_id) and is_binary(node_id) and is_integer(completed_nodes) and
              is_integer(total_nodes) and total_nodes > 0 do
    case Store.get(job_id) do
      {:ok, %{status: :cancelled}} ->
        throw({:workflow_cancelled, node_id})

      {:ok, _job} ->
        progress = min(completed_nodes / total_nodes, 0.98)

        progress_event = %{
          "node_id" => node_id,
          "completed_nodes" => completed_nodes,
          "total_nodes" => total_nodes,
          "progress" => progress,
          "emitted_at" => DateTime.utc_now() |> DateTime.to_iso8601()
        }

        :ok =
          update_workflow_runtime(job_id, fn runtime ->
            runtime
            |> Map.put("current_node", node_id)
            |> Map.update("progress_events", [progress_event], fn events ->
              (events ++ [progress_event]) |> Enum.take(-25)
            end)
          end)

        _ =
          Store.apply_progress(%{
            job_id: job_id,
            stage: "solving",
            progress: progress,
            iteration: completed_nodes,
            message: "completed workflow node #{node_id}"
          })

        :ok

      :error ->
        :ok
    end
  end

  defp workflow_progress_events(job_id) when is_binary(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, %{"progress_events" => events}} when is_list(events) -> events
      _ -> []
    end
  end

  defp update_workflow_runtime(job_id, updater)
       when is_binary(job_id) and is_function(updater, 1) do
    runtime =
      case AnalysisResultStore.get(job_id) do
        {:ok, payload} when is_map(payload) -> payload
        _ -> %{}
      end

    AnalysisResultStore.put(job_id, updater.(runtime))
  end

  defp orchestration_context_from_params(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    %{}
    |> maybe_put_orchestration_value("control_mode", Map.get(normalized, "control_mode"))
    |> maybe_put_orchestration_value("orch_id", Map.get(normalized, "orch_id"))
    |> maybe_put_orchestration_value("orch_session_id", Map.get(normalized, "orch_session_id"))
    |> maybe_put_orchestration_value("cluster_id", Map.get(normalized, "cluster_id"))
  end

  defp maybe_put_orchestration_value(context, _key, nil), do: context

  defp maybe_put_orchestration_value(context, key, value) when is_binary(value) and value != "",
    do: Map.put(context, key, value)

  defp maybe_put_orchestration_value(context, _key, _value), do: context

  defp fail_job(job_id, message) when is_binary(job_id) and is_binary(message) do
    _ =
      Store.apply_progress(%{
        job_id: job_id,
        stage: "failed",
        progress: 1.0,
        message: message
      })

    :ok
  end

  defp cancel_job_with_message(job_id, message) when is_binary(job_id) and is_binary(message) do
    _ =
      Store.apply_progress(%{
        job_id: job_id,
        stage: "cancelled",
        progress: 1.0,
        message: message
      })

    :ok
  end

  defp cancelled?(job_id) when is_binary(job_id) do
    match?({:ok, %{status: :cancelled}}, Store.get(job_id))
  end

  defp watchdog_job_timeout_ms do
    Application.get_env(:kyuubiki_web, KyuubikiWeb.Jobs.Watchdog, [])
    |> Keyword.get(:job_timeout_ms, 120_000)
  end
end
