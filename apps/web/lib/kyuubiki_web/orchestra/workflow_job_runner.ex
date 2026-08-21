defmodule KyuubikiWeb.Orchestra.WorkflowJobRunner do
  @moduledoc """
  Fenced background workflow lifecycle for the compact Orchestra engine.
  """

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.WorkflowGraphResponse

  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry

  @spec orchestration_context_from_params(map()) :: map()
  def orchestration_context_from_params(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    %{}
    |> maybe_put_orchestration_value("control_mode", Map.get(normalized, "control_mode"))
    |> maybe_put_orchestration_value("orch_id", Map.get(normalized, "orch_id"))
    |> maybe_put_orchestration_value("orch_session_id", Map.get(normalized, "orch_session_id"))
    |> maybe_put_orchestration_value("cluster_id", Map.get(normalized, "cluster_id"))
  end

  @spec initialize_runtime(String.t(), map(), map(), map(), map()) :: :ok | {:error, term()}
  def initialize_runtime(job_id, graph, input_artifacts, orchestration_context, response_options)
      when is_binary(job_id) and is_map(graph) and is_map(input_artifacts) and
             is_map(orchestration_context) and is_map(response_options) do
    WorkflowRecoveryCoordinator.initialize(
      job_id,
      graph,
      input_artifacts,
      orchestration_context,
      response_options
    )
  end

  @spec start(String.t()) :: {:ok, pid()} | {:error, term()}
  def start(job_id) when is_binary(job_id), do: WorkflowRecoveryCoordinator.dispatch(job_id)

  @doc false
  @spec start_claimed(String.t(), map(), map()) :: {:ok, pid()} | {:error, term()}
  def start_claimed(job_id, claim, %{
        "graph" => graph,
        "input_artifacts" => input_artifacts,
        "orchestration_context" => orchestration_context,
        "response_options" => response_options
      })
      when is_binary(job_id) and is_map(claim) and is_map(graph) and is_map(input_artifacts) and
             is_map(orchestration_context) and is_map(response_options) do
    try do
      Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
        case Registry.register(@registry, job_id, claim) do
          {:ok, _value} ->
            execute(
              job_id,
              claim,
              graph,
              input_artifacts,
              orchestration_context,
              response_options
            )

          {:error, {:already_registered, pid}} ->
            exit({:duplicate_workflow_runner, job_id, pid})
        end
      end)
    catch
      :exit, _reason -> {:error, :workflow_runner_supervisor_unavailable}
    end
  end

  def start_claimed(_job_id, _claim, _envelope),
    do: {:error, :invalid_workflow_execution_envelope}

  @spec running(String.t()) :: {:ok, pid()} | :error
  def running(job_id) when is_binary(job_id) do
    case Registry.lookup(@registry, job_id) do
      [{pid, _claim}] when is_pid(pid) -> {:ok, pid}
      _ -> :error
    end
  end

  defp execute(
         job_id,
         claim,
         graph,
         input_artifacts,
         orchestration_context,
         response_options
       ) do
    timeout_ms = watchdog_job_timeout_ms()

    task =
      Task.async(fn ->
        Engine.execute_workflow_graph(
          graph,
          input_artifacts,
          orchestration_context,
          fn progress -> apply_progress(job_id, claim, progress) end,
          response_options
        )
      end)

    case Task.yield(task, timeout_ms) || Task.shutdown(task, :brutal_kill) do
      {:ok, {:ok, result}} ->
        complete_job(job_id, claim, graph, result, response_options)

      {:ok, {:error, {:workflow_cancelled, _node_id}}} ->
        if not cancelled?(job_id), do: fail_job(job_id, claim, "workflow execution was fenced")

      {:ok, {:error, reason}} ->
        if not cancelled?(job_id), do: fail_job(job_id, claim, inspect(reason))

      {:exit, reason} ->
        exit({:workflow_execution_process_lost, reason})

      nil ->
        if not cancelled?(job_id) do
          fail_job(job_id, claim, "workflow execution timed out after #{timeout_ms} ms")
        end
    end
  end

  defp complete_job(job_id, claim, graph, result, response_options) do
    unless cancelled?(job_id) do
      shaped_result = WorkflowGraphResponse.shape(graph, result, response_options)

      case WorkflowRecoveryCoordinator.commit_result(job_id, claim, shaped_result) do
        :ok ->
          :ok

        {:error, :stale_workflow_execution_claim} ->
          :ok

        {:error, reason} ->
          fail_job(job_id, claim, "failed to persist workflow result: #{inspect(reason)}")
      end
    end
  end

  defp apply_progress(job_id, claim, %{"node_id" => node_id} = progress)
       when is_binary(job_id) and is_binary(node_id) do
    case WorkflowRecoveryCoordinator.record_progress(job_id, claim, progress) do
      :ok -> :ok
      {:error, _reason} -> throw({:workflow_cancelled, node_id})
    end
  end

  defp apply_progress(_job_id, _claim, _progress), do: :ok

  defp fail_job(job_id, claim, message) do
    case WorkflowRecoveryCoordinator.fail(job_id, claim, message) do
      :ok -> :ok
      {:error, :stale_workflow_execution_claim} -> :ok
      {:error, _reason} -> :ok
    end
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
