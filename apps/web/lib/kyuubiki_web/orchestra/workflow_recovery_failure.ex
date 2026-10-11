defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryFailure do
  @moduledoc false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowNativeResultRecovery, WorkflowRecoveryEnvelope}

  @active ~w(queued preprocessing partitioning solving postprocessing)a

  def finish(job_id, claim, message, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, runtime, recovery} <- fetch_runtime(job_id),
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
           {:ok, ended, text} <- transition(recovery, message),
           replacement <- Map.put(runtime, WorkflowRecoveryEnvelope.internal_key(), ended),
           {:ok, _} <-
             Store.apply_progress_with_result(
               %{job_id: job_id, stage: "failed", progress: 1.0, message: text},
               job,
               runtime,
               replacement
             ) do
        :ok
      else
        false -> {:error, :stale_workflow_execution_claim}
        {:error, _} = error -> error
        {:legacy_workflow, _} -> {:error, :legacy_workflow_recovery_unavailable}
        _ -> {:error, {:workflow_recovery_not_found, job_id}}
      end
    end)
  end

  defp fetch_runtime(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, runtime} when is_map(runtime) ->
        case runtime[WorkflowRecoveryEnvelope.internal_key()] do
          %{} = recovery -> {:ok, runtime, recovery}
          _ -> if is_binary(runtime["workflow_id"]), do: {:legacy_workflow, runtime}, else: :error
        end

      _ ->
        :error
    end
  end

  defp transition(recovery, message) when is_binary(message) do
    {:ok, WorkflowRecoveryEnvelope.transition(recovery, "failed", %{"message" => message}),
     message}
  end

  defp transition(recovery, {:native_workflow_recovery_blocked, reason}) do
    if WorkflowNativeResultRecovery.blocking_reason?(reason) do
      text = "workflow recovery blocked: native_workflow_recovery_#{reason}"

      details = %{
        "reason" => "native_workflow_recovery_#{reason}",
        "next_action" => "restore_original_native_receipt_or_explicit_resubmit"
      }

      {:ok, WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", details), text}
    else
      {:error, :invalid_native_workflow_recovery_reason}
    end
  end

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end
end
