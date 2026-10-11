defmodule KyuubikiWeb.Orchestra.WorkflowActivityStore do
  @moduledoc false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryEnvelope}

  @active ~w(queued preprocessing partitioning solving postprocessing)a

  def touch(job_id, claim, node_id, lease, persist?) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, _runtime, recovery} <- fetch_runtime(job_id),
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim) do
        if persist? do
          # Activity cannot complete a node or reset its execution deadline.
          case Store.apply_progress_if_current(
                 %{
                   job_id: job_id,
                   stage: Atom.to_string(job.status),
                   progress: job.progress,
                   iteration: job.iteration,
                   message: "workflow node #{node_id} active"
                 },
                 job
               ) do
            {:ok, _job} -> :ok
            error -> error
          end
        else
          :ok
        end
      else
        false -> {:error, :stale_workflow_execution_claim}
        error -> error
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

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end
end
