defmodule KyuubikiWeb.AnalysisResultMutation do
  @moduledoc false

  alias KyuubikiWeb.Jobs.Job
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope

  @internal WorkflowRecoveryEnvelope.internal_key()
  @terminal_jobs [:completed, :failed, :cancelled]
  @terminal_recovery ["completed", "failed", "cancelled", "recovery_blocked"]

  def prepare(%Job{} = job, current, action) do
    with :ok <- mutable?(job, current),
         :ok <- validate_action(action) do
      case action do
        :delete -> {:ok, current}
        {:replace, replacement} -> {:ok, preserve_identity(current, replacement)}
      end
    end
  end

  defp mutable?(job, current) do
    internal? = Map.has_key?(current, @internal) or Map.has_key?(current, :_workflow_recovery)
    recovery = Map.get(current, @internal, Map.get(current, :_workflow_recovery))

    workflow? =
      internal? or is_binary(Map.get(current, "workflow_id", Map.get(current, :workflow_id)))

    terminal? = is_map(recovery) and Map.get(recovery, "state") in @terminal_recovery

    if workflow? and (job.status not in @terminal_jobs or (internal? and not terminal?)),
      do: {:error, :active_workflow_result_is_read_only},
      else: :ok
  end

  defp validate_action(:delete), do: :ok

  defp validate_action({:replace, replacement}) do
    if Map.has_key?(replacement, @internal) or Map.has_key?(replacement, :_workflow_recovery),
      do: {:error, :workflow_recovery_metadata_is_read_only},
      else: :ok
  end

  defp preserve_identity(current, replacement) do
    keys = [@internal, :_workflow_recovery, "workflow_id", :workflow_id]
    identity = Map.take(current, keys)

    # Ordinary solver results may also contain public recovery data. Only an
    # actual workflow identity makes that field a derived coordinator summary.
    workflow? =
      Enum.any?([@internal, :_workflow_recovery], &Map.has_key?(identity, &1)) or
        is_binary(Map.get(identity, "workflow_id", Map.get(identity, :workflow_id)))

    if workflow?,
      do: replacement |> Map.drop(keys ++ ["recovery", :recovery]) |> Map.merge(identity),
      else: replacement
  end
end
