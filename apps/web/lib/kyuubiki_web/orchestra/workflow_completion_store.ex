defmodule KyuubikiWeb.Orchestra.WorkflowCompletionStore do
  @moduledoc false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope

  @active [:queued, :preprocessing, :partitioning, :solving, :postprocessing]

  def stage(job_id, claim, result, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job, runtime, recovery} <- current(job_id),
           true <- Envelope.fenced?(recovery, claim),
           {:ok, updated, disposition} <- Checkpoint.prepare(job, recovery, claim, result) do
        if disposition == :unchanged do
          :ok
        else
          Store.apply_progress_with_result(
            %{job_id: job_id, stage: job.status, progress: job.progress},
            job,
            runtime,
            Map.put(runtime, Envelope.internal_key(), updated)
          )
          |> reply()
        end
      else
        false -> {:error, :stale_workflow_execution_claim}
        error -> error
      end
    end)
  end

  def pending(job_id, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job, _runtime, recovery} <- current(job_id),
           do: Checkpoint.read(job, recovery)
    end)
  end

  def commit(job_id, claim, result, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job, runtime, recovery} <- current(job_id),
           true <- Envelope.fenced?(recovery, claim),
           :ok <- Checkpoint.matches(job, recovery, result),
           {:ok, failed_count} <- failed_node_count(result) do
        completed =
          Envelope.transition(recovery, "completed", %{
            "committed_generation" => claim["generation"]
          })

        final =
          result
          |> Map.put("workflow_id", Map.get(runtime, "workflow_id"))
          |> Map.put("current_node", nil)
          |> Map.put("progress_events", Map.get(runtime, "progress_events", []))
          |> Map.put("response_options", Map.get(runtime, "response_options", %{}))
          |> Map.put(Envelope.internal_key(), completed)

        Store.apply_progress_with_result(
          %{
            job_id: job_id,
            stage: "completed",
            progress: 1.0,
            message:
              if(failed_count == 0,
                do: "workflow completed",
                else:
                  "workflow completed with #{failed_count} failed node(s); inspect node_failures"
              )
          },
          job,
          runtime,
          final
        )
        |> reply()
      else
        false -> {:error, :stale_workflow_execution_claim}
        error -> error
      end
    end)
  end

  defp current(job_id) do
    with {:ok, job} <- active_job(job_id),
         {:ok, runtime} when is_map(runtime) <- AnalysisResultStore.get(job_id),
         %{} = recovery <- runtime[Envelope.internal_key()] do
      {:ok, job, runtime, recovery}
    else
      {:error, _} = error -> error
      _ -> {:error, {:workflow_recovery_not_found, job_id}}
    end
  end

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end

  defp failed_node_count(result) do
    case Map.get(result, "failed_nodes", []) do
      nodes when is_list(nodes) -> {:ok, length(nodes)}
      _ -> {:error, {:invalid_workflow_result, :failed_nodes}}
    end
  end

  defp reply({:ok, _}), do: :ok
  defp reply(error), do: error
end
