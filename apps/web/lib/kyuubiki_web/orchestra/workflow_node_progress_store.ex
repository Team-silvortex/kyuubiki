defmodule KyuubikiWeb.Orchestra.WorkflowNodeProgressStore do
  @moduledoc false
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowNodeProgress, WorkflowRecoveryEnvelope}

  @active ~w(queued preprocessing partitioning solving postprocessing)a

  def record(job_id, claim, update, lease) do
    with {:ok, update} <- WorkflowNodeProgress.normalize(update) do
      node_id = update["node_id"]
      resolved_nodes = update["resolved_nodes"]

      LeaseStore.with_lease(lease, fn ->
        with {:ok, job} <- active_job(job_id),
             {:ok, runtime, recovery} <- fetch_runtime(job_id),
             true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
             execution_progress <- min(resolved_nodes / update["total_nodes"], 0.98),
             replay? <- claim["generation"] > 1,
             progress <-
               if(replay?, do: max(job.progress, execution_progress), else: execution_progress),
             progress_event <- WorkflowNodeProgress.event(update, progress),
             progress_event <-
               Map.merge(progress_event, %{
                 "generation" => claim["generation"],
                 "attempt" => claim["attempt"],
                 "execution_progress" => execution_progress
               }),
             updated_runtime <-
               runtime
               |> Map.put("current_node", node_id)
               |> Map.update("progress_events", [progress_event], fn events ->
                 (List.wrap(events) ++ [progress_event]) |> Enum.take(-25)
               end),
             {:ok, _} <-
               Store.apply_progress_with_result(
                 %{
                   job_id: job_id,
                   stage: "solving",
                   progress: progress,
                   iteration:
                     if(replay?,
                       do: max(job.iteration || 0, resolved_nodes),
                       else: resolved_nodes
                     ),
                   message: "#{update["status"]} workflow node #{node_id}"
                 },
                 job,
                 runtime,
                 updated_runtime
               ) do
          :ok
        else
          false -> {:error, :stale_workflow_execution_claim}
          {:error, _} = error -> error
          :error -> {:error, {:workflow_recovery_not_found, job_id}}
          {:legacy_workflow, _} -> {:error, :legacy_workflow_recovery_unavailable}
        end
      end)
    end
  end

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end

  defp fetch_runtime(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, runtime} when is_map(runtime) ->
        case runtime[WorkflowRecoveryEnvelope.internal_key()] do
          %{} = recovery -> {:ok, runtime, recovery}
          _ -> if is_binary(runtime["workflow_id"]), do: {:legacy_workflow, runtime}, else: :error
        end

      {:error, _} = error ->
        error

      _ ->
        :error
    end
  end
end
