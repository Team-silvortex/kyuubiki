defmodule KyuubikiWeb.Orchestra.WorkflowJobRunner do
  @moduledoc """
  Fenced background workflow lifecycle for the compact Orchestra engine.
  """

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator

  alias KyuubikiWeb.Playground.{
    AgentDispatchCancellation,
    AgentExecutionGate
  }

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

  @spec admit(map(), map(), map(), map(), map()) ::
          {:ok, KyuubikiWeb.Jobs.Job.t()} | {:error, term()}
  defdelegate admit(attrs, graph, inputs, context, options), to: WorkflowRecoveryCoordinator

  @spec initialize_runtime(String.t(), map(), map(), map(), map()) :: :ok | {:error, term()}
  defdelegate initialize_runtime(job_id, graph, inputs, context, options),
    to: WorkflowRecoveryCoordinator,
    as: :initialize

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

  @doc false
  def start_finalizing(job_id, claim, result) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      case Registry.register(@registry, job_id, claim) do
        {:ok, _} -> publish_result(job_id, claim, result)
        {:error, {:already_registered, pid}} -> exit({:duplicate_workflow_runner, job_id, pid})
      end
    end)
  catch
    :exit, _ -> {:error, :workflow_runner_supervisor_unavailable}
  end

  @doc false
  def registry_status do
    case lookup({__MODULE__, :availability}) do
      {:error, _} = error -> error
      _entries -> :ok
    end
  end

  @spec running(String.t()) ::
          {:ok, pid()} | :error | {:error, :workflow_runner_registry_unavailable}
  def running(job_id) when is_binary(job_id) do
    case lookup(job_id) do
      [{pid, _claim}] when is_pid(pid) -> {:ok, pid}
      {:error, _} = error -> error
      _ -> :error
    end
  end

  defp lookup(key) do
    Registry.lookup(@registry, key)
  rescue
    # Registry is fixed and keys are unrestricted; its missing ETS tables are unavailable,
    # not evidence that a runner has stopped. Keep this boundary around lookup only.
    _ in ArgumentError -> {:error, :workflow_runner_registry_unavailable}
  end

  @doc false
  def stop_failed(job_id, request_cancel? \\ true) do
    targets = cancellation_targets(job_id)

    case running(job_id) do
      {:ok, pid} -> Process.exit(pid, :shutdown)
      :error -> :ok
      {:error, :workflow_runner_registry_unavailable} -> :ok
    end

    if request_cancel? and match?({:ok, _}, targets),
      do: request_cancellation(job_id, elem(targets, 1))

    :ok
  end

  defp cancellation_targets(job_id) do
    AgentExecutionGate.cancel_job_targets(job_id)
  catch
    :exit, _ -> {:error, :execution_gate_unavailable}
  end

  defp request_cancellation(job_id, targets) do
    if targets.endpoints != [] do
      Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
        AgentDispatchCancellation.cancel(job_id, targets)
      end)
    end
  catch
    :exit, _ -> :ok
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
          response_options,
          recover_native_results: claim["generation"] > 1,
          native_dispatch_evidence: fn action ->
            WorkflowRecoveryCoordinator.native_dispatch_evidence(job_id, claim, action)
          end,
          native_result_checkpoint: fn action ->
            WorkflowRecoveryCoordinator.native_result_checkpoint(job_id, claim, action)
          end,
          solve_checkpoint: fn action ->
            WorkflowRecoveryCoordinator.solve_checkpoint(job_id, claim, action)
          end,
          branch_checkpoint: fn action ->
            WorkflowRecoveryCoordinator.branch_checkpoint(job_id, claim, action)
          end
        )
      end)

    case await_execution(task, job_id, claim, timeout_ms) do
      {:ok, {:ok, result}} ->
        complete_job(job_id, claim, graph, result, response_options)

      {:ok, {:error, {:workflow_cancelled, _node_id}}} ->
        if not cancelled?(job_id), do: fail_job(job_id, claim, "workflow execution was fenced")

      {:ok, {:error, {:workflow_node_error, _node, {:native_workflow_recovery_blocked, reason}}}} ->
        await_terminal_commit(fn ->
          WorkflowRecoveryCoordinator.block_native_recovery(job_id, claim, reason)
        end)

      {:ok, {:error, {:workflow_node_error, _node, {:workflow_solve_checkpoint_blocked, reason}}}} ->
        await_terminal_commit(fn ->
          WorkflowRecoveryCoordinator.block_solve_checkpoint(job_id, claim, reason)
        end)

      {:ok,
       {:error, {:workflow_node_error, _node, {:workflow_branch_checkpoint_blocked, reason}}}} ->
        await_terminal_commit(fn ->
          WorkflowRecoveryCoordinator.block_branch_checkpoint(job_id, claim, reason)
        end)

      {:ok, {:error, reason}} ->
        if not cancelled?(job_id), do: fail_job(job_id, claim, inspect(reason))

      {:exit, reason} ->
        exit({:workflow_execution_process_lost, reason})

      :deadline_retired ->
        :ok
    end
  end

  defp await_execution(task, job_id, claim, timeout_ms) do
    case Task.yield(task, timeout_ms) do
      nil ->
        await_deadline(task, job_id, claim, "workflow execution timed out after #{timeout_ms} ms")

      reply ->
        reply
    end
  end

  defp await_deadline(task, job_id, claim, message) do
    case WorkflowRecoveryCoordinator.expire(job_id, claim, message) do
      :ok ->
        cancellation_targets(job_id)
        Task.shutdown(task, :brutal_kill)
        :deadline_retired

      {:error, reason} ->
        if retryable_retirement?(reason) do
          # A missed deadline stays missed: discard late output, never publish it as success.
          # Re-read under the claim/lease on each retry; do not replay an unchecked write.
          Task.yield(task, 0)
          Process.sleep(250)
          await_deadline(task, job_id, claim, message)
        else
          # Lost authority must not cancel other work sharing this job ID.
          Task.shutdown(task, :brutal_kill)
          :deadline_retired
        end
    end
  end

  defp retryable_retirement?({:completion_persistence_failed, {:invalid_payload, _}}), do: false
  defp retryable_retirement?({:completion_persistence_failed, _}), do: true
  defp retryable_retirement?({:stale_job_snapshot, _}), do: true

  defp retryable_retirement?(reason),
    do:
      reason in [
        :stale_analysis_result,
        :execution_gate_unavailable,
        :workflow_coordinator_unavailable
      ]

  defp complete_job(job_id, claim, graph, result, response_options) do
    unless cancelled?(job_id) do
      shaped_result = WorkflowGraphResponse.shape(graph, result, response_options)

      with :ok <-
             await_terminal_commit(fn ->
               WorkflowRecoveryCoordinator.stage_result(job_id, claim, shaped_result)
             end) do
        publish_result(job_id, claim, shaped_result)
      else
        error -> completion_error(job_id, claim, error)
      end
    end
  end

  defp publish_result(job_id, claim, result) do
    case await_terminal_commit(fn ->
           WorkflowRecoveryCoordinator.commit_result(job_id, claim, result)
         end) do
      :ok -> :ok
      error -> completion_error(job_id, claim, error)
    end
  end

  defp completion_error(_job, _claim, {:error, :stale_workflow_execution_claim}), do: :ok
  defp completion_error(_job, _claim, {:error, {:workflow_job_terminal, _}}), do: :ok

  defp completion_error(job, claim, {:error, reason}),
    do: fail_job(job, claim, "failed to persist workflow result: #{inspect(reason)}")

  defp apply_progress(job_id, claim, %{"node_id" => node_id} = progress)
       when is_binary(job_id) and is_binary(node_id) do
    case WorkflowRecoveryCoordinator.record_progress(job_id, claim, progress) do
      :ok ->
        :ok

      {:error, {:workflow_job_terminal, :cancelled}} ->
        # Keep the dispatch owner alive until targeted cancellation and its receipt arrive.
        if progress["event"] == "operator_activity",
          do: :ok,
          else: throw({:workflow_cancelled, node_id})

      {:error, _reason} ->
        throw({:workflow_cancelled, node_id})
    end
  end

  defp apply_progress(_job_id, _claim, _progress), do: :ok

  defp fail_job(job_id, claim, message) do
    await_terminal_commit(fn -> WorkflowRecoveryCoordinator.fail(job_id, claim, message) end)
  end

  defp await_terminal_commit(commit) do
    case terminal_commit(commit) do
      {:error, reason} = error ->
        if retryable_retirement?(reason) do
          # Keep the registered runner and original outcome; retry only its fenced commit.
          # Exiting here would turn a rejected terminal write into runner-loss recovery.
          Process.sleep(250)
          await_terminal_commit(commit)
        else
          error
        end

      reply ->
        reply
    end
  end

  defp terminal_commit(commit) do
    commit.()
  catch
    :exit, {reason, {GenServer, :call, [WorkflowRecoveryCoordinator | _]}}
    when reason in [:noproc, :normal, :shutdown, :killed, :timeout] ->
      {:error, :workflow_coordinator_unavailable}
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
