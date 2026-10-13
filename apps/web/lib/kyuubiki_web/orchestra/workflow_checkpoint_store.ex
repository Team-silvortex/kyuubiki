defmodule KyuubikiWeb.Orchestra.WorkflowCheckpointStore do
  @moduledoc false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store

  alias KyuubikiWeb.Orchestra.{
    LeaseStore,
    WorkflowRecoveryEnvelope,
    WorkflowSolveCheckpoint,
    WorkflowBranchCheckpoint,
    WorkflowNativeResultCheckpoint,
    WorkflowNativeDispatchEvidence
  }

  @active ~w(queued preprocessing partitioning solving postprocessing)a

  def access(job_id, claim, action, lease, checkpoint)
      when checkpoint in [
             WorkflowSolveCheckpoint,
             WorkflowBranchCheckpoint,
             WorkflowNativeResultCheckpoint,
             WorkflowNativeDispatchEvidence
           ] do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, runtime} when is_map(runtime) <- AnalysisResultStore.get(job_id),
           %{} = recovery <- runtime[WorkflowRecoveryEnvelope.internal_key()],
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
           :ok <- WorkflowRecoveryEnvelope.verify(recovery),
           true <-
             WorkflowRecoveryEnvelope.execution_value(recovery, [
               "orchestration_context",
               "job_id"
             ]) == job_id do
        apply_action(action, job, runtime, recovery, claim, checkpoint)
      else
        false -> {:error, :stale_workflow_execution_claim}
        {:error, _} = error -> error
        _ -> {:error, :invalid}
      end
    end)
  end

  defp apply_action({:lookup, request}, _job, _runtime, recovery, claim, checkpoint),
    do: checkpoint.lookup(recovery, request, claim)

  defp apply_action({:commit, request, result}, job, runtime, recovery, claim, checkpoint) do
    with {:ok, updated, disposition} <-
           checkpoint.put(recovery, request, result, claim) do
      if disposition == :retained do
        replacement = Map.put(runtime, WorkflowRecoveryEnvelope.internal_key(), updated)

        case Store.apply_progress_with_result(
               %{
                 job_id: job.job_id,
                 stage: job.status,
                 progress: job.progress,
                 iteration: job.iteration
               },
               job,
               runtime,
               replacement
             ) do
          {:ok, _} -> :ok
          error -> error
        end
      else
        :ok
      end
    end
  end

  defp apply_action(_action, _job, _runtime, _recovery, _claim, _checkpoint),
    do: {:error, :invalid}

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end
end
