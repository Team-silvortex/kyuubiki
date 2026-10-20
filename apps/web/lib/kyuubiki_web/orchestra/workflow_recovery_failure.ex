defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryFailure do
  @moduledoc false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowNativeResultRecovery, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint
  alias KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryDispatchGuard, as: DispatchGuard
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryRunners, as: Runners
  alias KyuubikiWeb.Orchestra.{WorkflowJobRunner, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.Orchestra.{WorkflowCancellationDelivery, WorkflowCancellationIntent}
  alias KyuubikiWeb.Playground.AgentExecutionGate

  @active ~w(queued preprocessing partitioning solving postprocessing)a

  def expire(job_id, claim, message)
      when is_binary(job_id) and is_map(claim) and is_binary(message) do
    GenServer.call(WorkflowRecoveryCoordinator, {:expire, job_id, claim, message}, 30_000)
  catch
    :exit, {reason, {GenServer, :call, [WorkflowRecoveryCoordinator | _]}}
    when reason in [:noproc, :normal, :shutdown, :killed, :timeout] ->
      {:error, :workflow_coordinator_unavailable}
  end

  def handle_expiry(job_id, claim, message, state) do
    reply =
      if Ownership.owner?(state),
        do: finish(job_id, claim, message, state.lease, true),
        else: {:error, :orchestra_standby}

    next =
      if reply == :ok do
        WorkflowCancellationDelivery.kick([job_id])

        Runners.retire(state, job_id)
      else
        state
      end

    # The calling runner stops its own linked task only after this commit returns.
    {:reply, reply, Ownership.after_write(next, reply)}
  end

  def fail_observed(%KyuubikiWeb.Jobs.Job{} = job, message) when is_binary(message) do
    GenServer.call(WorkflowRecoveryCoordinator, {:fail_observed, job, message}, 30_000)
  catch
    :exit, {reason, {GenServer, :call, [WorkflowRecoveryCoordinator | _]}}
    when reason in [:noproc, :normal, :shutdown, :killed, :timeout] ->
      {:error, :workflow_coordinator_unavailable}
  end

  def handle_observed(job, message, state) do
    result = Ownership.guarded_write(state, fn -> finish_observed(job, message) end)

    {reply, durable?} =
      case result do
        {:ok, retired, :workflow} -> {{:ok, retired}, true}
        other -> {other, false}
      end

    next =
      if match?({:ok, _}, reply) do
        # Stop only after both snapshots committed; a rejected scan must not cancel work.
        WorkflowJobRunner.stop_failed(job.job_id, not durable?)
        if durable?, do: WorkflowCancellationDelivery.kick([job.job_id])

        Runners.retire(state, job.job_id)
      else
        state
      end

    {:reply, reply, Ownership.after_write(next, reply)}
  end

  defp finish_observed(%{status: status} = expected, message) when status in @active do
    with {:ok, ^expected} <- Store.get(expected.job_id) do
      attrs = %{
        job_id: expected.job_id,
        stage: "failed",
        progress: expected.progress,
        message: message
      }

      case AnalysisResultStore.get(expected.job_id) do
        {:ok, %{"_workflow_recovery" => %{"state" => terminal}}}
        when terminal in ["completed", "failed", "cancelled", "recovery_blocked"] ->
          {:error, :workflow_recovery_not_active}

        {:ok, %{"_workflow_recovery" => %{} = recovery} = runtime} ->
          with {:ok, ended, _} <- transition(expected.job_id, recovery, message),
               {:ok, ended} <- capture_intent(ended, expected, true) do
            replacement = Map.put(runtime, WorkflowRecoveryEnvelope.internal_key(), ended)

            with {:ok, retired} <-
                   Store.apply_progress_with_result(attrs, expected, runtime, replacement),
                 do: {:ok, retired, :workflow}
          end

        {:ok, %{"_workflow_recovery" => _}} ->
          {:error, :invalid_workflow_recovery_record}

        {:ok, %{} = runtime} ->
          # Legacy and non-workflow results still participate in the atomic snapshot.
          Store.apply_progress_with_result(attrs, expected, runtime, runtime)

        :error ->
          Store.apply_progress_if_current(attrs, expected)

        {:error, _} = error ->
          error
      end
    else
      {:ok, _} -> {:error, {:stale_job_snapshot, expected.job_id}}
      :error -> {:error, {:job_not_found, expected.job_id}}
      {:error, _} = error -> error
    end
  end

  defp finish_observed(expected, _message),
    do: {:error, {:workflow_job_terminal, expected.status}}

  defp observe_targets(job_id) do
    AgentExecutionGate.observe_job_targets(job_id)
  catch
    :exit, _ -> {:error, :execution_gate_unavailable}
  end

  def finish(job_id, claim, message, lease, capture_targets? \\ false) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, runtime, recovery} <- fetch_runtime(job_id),
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
           {:ok, ended, text} <- transition(job_id, recovery, message),
           {:ok, ended} <- capture_intent(ended, job, capture_targets?),
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

  defp capture_intent(recovery, _job, false), do: {:ok, recovery}

  defp capture_intent(recovery, job, true) do
    with {:ok, targets} <- observe_targets(job.job_id),
         do: WorkflowCancellationIntent.attach(recovery, job, targets)
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

  defp transition(job_id, recovery, message) when is_binary(message) do
    with :ok <- WorkflowRecoveryEnvelope.verify(recovery),
         :ok <- DispatchGuard.verify_native_dispatch(job_id, recovery) do
      {:ok, WorkflowRecoveryEnvelope.transition(recovery, "failed", %{"message" => message}),
       message}
    else
      {:error, reason} ->
        details = %{
          "message" => message,
          "reason" => failure_fence_reason(reason),
          "next_action" => DispatchGuard.next_action(reason)
        }

        {:ok, WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", details), message}
    end
  end

  defp transition(_job_id, recovery, message), do: transition(recovery, message)

  defp failure_fence_reason({:workflow_dispatch_journal_unavailable, _reason}),
    do: "workflow_dispatch_journal_unavailable"

  defp failure_fence_reason(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp failure_fence_reason(_reason), do: "invalid_workflow_recovery_record"

  defp transition(recovery, {:native_workflow_recovery_blocked, reason}) do
    if WorkflowNativeResultRecovery.blocking_reason?(reason) do
      text = "workflow recovery blocked: native_workflow_recovery_#{reason}"

      details = %{
        "reason" => "native_workflow_recovery_#{reason}",
        "next_action" => native_next_action(reason)
      }

      {:ok, WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", details), text}
    else
      {:error, :invalid_native_workflow_recovery_reason}
    end
  end

  defp transition(recovery, {:workflow_solve_checkpoint_blocked, reason}) do
    if WorkflowSolveCheckpoint.blocking_reason?(reason) do
      text = "workflow recovery blocked: workflow_solve_checkpoint_#{reason}"

      details = %{
        "reason" => "workflow_solve_checkpoint_#{reason}",
        "next_action" => "inspect_solve_checkpoint_before_explicit_resubmit"
      }

      {:ok, WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", details), text}
    else
      {:error, :invalid_workflow_solve_checkpoint_reason}
    end
  end

  defp transition(recovery, {:workflow_branch_checkpoint_blocked, reason}) do
    if WorkflowBranchCheckpoint.blocking_reason?(reason) do
      text = "workflow recovery blocked: workflow_branch_checkpoint_#{reason}"

      details = %{
        "reason" => "workflow_branch_checkpoint_#{reason}",
        "next_action" => "inspect_branch_checkpoint_before_explicit_resubmit"
      }

      {:ok, WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", details), text}
    else
      {:error, :invalid_workflow_branch_checkpoint_reason}
    end
  end

  defp native_next_action(reason)
       when reason in [
              :result_checkpoint_invalid,
              :result_checkpoint_unavailable,
              :result_checkpoint_commit_failed
            ],
       do: "inspect_native_result_checkpoint_before_explicit_resubmit"

  defp native_next_action(_), do: "restore_original_native_receipt_or_explicit_resubmit"

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end
end
