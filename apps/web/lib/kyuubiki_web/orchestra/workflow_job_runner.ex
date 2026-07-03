defmodule KyuubikiWeb.Orchestra.WorkflowJobRunner do
  @moduledoc """
  Background workflow job lifecycle for the compact orchestra engine.
  """

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.WorkflowGraphResponse

  @spec orchestration_context_from_params(map()) :: map()
  def orchestration_context_from_params(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    %{}
    |> maybe_put_orchestration_value("control_mode", Map.get(normalized, "control_mode"))
    |> maybe_put_orchestration_value("orch_id", Map.get(normalized, "orch_id"))
    |> maybe_put_orchestration_value("orch_session_id", Map.get(normalized, "orch_session_id"))
    |> maybe_put_orchestration_value("cluster_id", Map.get(normalized, "cluster_id"))
  end

  @spec initialize_runtime(String.t(), map(), map(), map()) :: :ok | {:error, term()}
  def initialize_runtime(job_id, graph, orchestration_context, response_options)
      when is_binary(job_id) and is_map(graph) and is_map(orchestration_context) and
             is_map(response_options) do
    AnalysisResultStore.put(job_id, %{
      "workflow_id" => Map.get(graph, "id"),
      "current_node" => nil,
      "progress_events" => [],
      "completed_nodes" => [],
      "artifacts" => %{},
      "response_options" => response_options,
      "orchestration_context" => orchestration_context
    })
  end

  @spec start(String.t(), map(), map(), map(), map()) :: {:ok, pid()} | {:error, term()}
  def start(job_id, graph, input_artifacts, orchestration_context, response_options)
      when is_binary(job_id) and is_map(graph) and is_map(input_artifacts) and
             is_map(orchestration_context) and is_map(response_options) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      execute(job_id, graph, input_artifacts, orchestration_context, response_options)
    end)
  end

  defp execute(job_id, graph, input_artifacts, orchestration_context, response_options) do
    timeout_ms = watchdog_job_timeout_ms()

    task =
      Task.async(fn ->
        Engine.execute_workflow_graph(
          graph,
          input_artifacts,
          orchestration_context,
          fn progress ->
            apply_progress(job_id, progress)
          end,
          response_options
        )
      end)

    case Task.yield(task, timeout_ms) || Task.shutdown(task, :brutal_kill) do
      {:ok, {:ok, result}} ->
        complete_job(job_id, graph, result, response_options)

      {:ok, {:error, {:workflow_cancelled, _node_id}}} ->
        apply_terminal_progress(job_id, "cancelled", "workflow cancelled by operator")

      {:ok, {:error, reason}} ->
        unless cancelled?(job_id), do: apply_terminal_progress(job_id, "failed", inspect(reason))

      nil ->
        unless cancelled?(job_id) do
          apply_terminal_progress(
            job_id,
            "failed",
            "workflow execution timed out after #{timeout_ms} ms"
          )
        end
    end
  end

  defp complete_job(job_id, graph, result, response_options) do
    unless cancelled?(job_id) do
      shaped_result = WorkflowGraphResponse.shape(graph, result, response_options)

      case AnalysisResultStore.put(
             job_id,
             shaped_result
             |> Map.put("workflow_id", Map.get(graph, "id"))
             |> Map.put("current_node", nil)
             |> Map.put("progress_events", progress_events(job_id))
             |> Map.put("response_options", response_options)
           ) do
        :ok ->
          _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})

        {:error, reason} ->
          apply_terminal_progress(job_id, "failed", "failed to persist workflow result: #{inspect(reason)}")
      end
    end
  end

  defp apply_progress(job_id, %{
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
        progress_event = progress_event(node_id, completed_nodes, total_nodes, progress)

        case update_runtime(job_id, fn runtime ->
               runtime
               |> Map.put("current_node", node_id)
               |> Map.update("progress_events", [progress_event], fn events ->
                 (events ++ [progress_event]) |> Enum.take(-25)
               end)
             end) do
          :ok ->
            _ =
              Store.apply_progress(%{
                job_id: job_id,
                stage: "solving",
                progress: progress,
                iteration: completed_nodes,
                message: "completed workflow node #{node_id}"
              })

            :ok

          {:error, reason} ->
            apply_terminal_progress(job_id, "failed", "failed to persist workflow progress: #{inspect(reason)}")
        end

      :error ->
        :ok
    end
  end

  defp progress_event(node_id, completed_nodes, total_nodes, progress) do
    %{
      "node_id" => node_id,
      "completed_nodes" => completed_nodes,
      "total_nodes" => total_nodes,
      "progress" => progress,
      "emitted_at" => DateTime.utc_now() |> DateTime.to_iso8601()
    }
  end

  defp progress_events(job_id) when is_binary(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, %{"progress_events" => events}} when is_list(events) -> events
      _ -> []
    end
  end

  defp update_runtime(job_id, updater) when is_binary(job_id) and is_function(updater, 1) do
    runtime =
      case AnalysisResultStore.get(job_id) do
        {:ok, payload} when is_map(payload) -> payload
        _ -> %{}
      end

    AnalysisResultStore.put(job_id, updater.(runtime))
  end

  defp apply_terminal_progress(job_id, stage, message)
       when is_binary(job_id) and is_binary(stage) and is_binary(message) do
    _ = Store.apply_progress(%{job_id: job_id, stage: stage, progress: 1.0, message: message})
    :ok
  end

  defp cancelled?(job_id) when is_binary(job_id) do
    match?({:ok, %{status: :cancelled}}, Store.get(job_id))
  end

  defp watchdog_job_timeout_ms do
    Application.get_env(:kyuubiki_web, KyuubikiWeb.Jobs.Watchdog, [])
    |> Keyword.get(:workflow_timeout_ms, 30_000)
  end

  defp maybe_put_orchestration_value(context, _key, nil), do: context

  defp maybe_put_orchestration_value(context, key, value) when is_binary(value) and value != "",
    do: Map.put(context, key, value)

  defp maybe_put_orchestration_value(context, _key, _value), do: context
end
