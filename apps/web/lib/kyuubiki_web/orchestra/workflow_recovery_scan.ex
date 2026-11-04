defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryScan do
  @moduledoc false

  alias KyuubikiWeb.Jobs.{ProgressEvent, Store}
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership
  alias KyuubikiWeb.Storage.FailureBoundary

  @active [:queued, :preprocessing, :partitioning, :solving, :postprocessing]

  def defer_registry(state, job_id \\ nil, reason \\ nil) do
    state =
      if reason == :completion_retry,
        do: schedule_completion_retry(state, job_id),
        else: state

    if is_nil(state.registry_retry) do
      token = make_ref()
      timer = Process.send_after(self(), {:retry_runner_registry, token}, state.lease_retry_ms)
      %{state | registry_retry: %{token: token, timer: timer}}
    else
      state
    end
  end

  def retry_registry(state, token, recover) do
    case state.registry_retry do
      %{token: ^token} ->
        next = clear_registry_retry(state)
        if Ownership.owner?(next), do: elem(recover.(next), 1), else: next

      _ ->
        state
    end
  end

  def clear_registry_retry(state) do
    if state.registry_retry, do: Process.cancel_timer(state.registry_retry.timer)
    %{state | registry_retry: nil}
  end

  def schedule_completion_retry(state, job_id) do
    if Map.has_key?(state.completion_retries, job_id) do
      state
    else
      token = make_ref()
      message = {:retry_completion, job_id, token}
      timer = Process.send_after(self(), message, state.lease_retry_ms)
      put_in(state, [:completion_retries, job_id], %{token: token, timer: timer})
    end
  end

  def retry_completion(state, job_id, token, recover_job) do
    case Map.get(state.completion_retries, job_id) do
      %{token: ^token} ->
        # Re-read durable evidence and ownership; never retain a stale result to publish.
        {_, next} =
          recover_job.(job_id, :completion_retry, forget_completion_retry(state, job_id))

        next

      _ ->
        state
    end
  end

  def forget_completion_retry(state, job_id) do
    {retry, remaining} = Map.pop(state.completion_retries, job_id)
    if retry, do: Process.cancel_timer(retry.timer)
    %{state | completion_retries: remaining}
  end

  def clear_completion_retries(state) do
    Enum.each(state.completion_retries, fn {_, retry} -> Process.cancel_timer(retry.timer) end)
    %{state | completion_retries: %{}}
  end

  def missing_completion_runtime(state, job_id, :completion_retry) do
    case Store.get(job_id) do
      {:ok, %{status: status}} when status in @active -> schedule_completion_retry(state, job_id)
      _ -> state
    end
  end

  def missing_completion_runtime(state, _job_id, _reason), do: state

  def after_completion_block(state, job_id, reply) do
    case reply do
      {:error, {:workflow_recovery_blocked, :workflow_completion_checkpoint_invalid}} ->
        KyuubikiWeb.Orchestra.WorkflowRecoveryRunners.retire(state, job_id)

      {:error, {:completion_persistence_failed, {:invalid_payload, _}}} ->
        state

      {:error, {:completion_persistence_failed, _}} ->
        schedule_completion_retry(state, job_id)

      {:error, {:stale_job_snapshot, _}} ->
        schedule_completion_retry(state, job_id)

      {:error, :stale_analysis_result} ->
        schedule_completion_retry(state, job_id)

      {:error, {:result_not_found, ^job_id}} ->
        schedule_completion_retry(state, job_id)

      _ ->
        Ownership.after_write(state, reply)
    end
  end

  def reconcile_terminal(
        job_id,
        %{"_workflow_recovery" => %{"state" => terminal} = recovery} = runtime
      )
      when terminal in ["completed", "failed", "cancelled", "recovery_blocked"] do
    update = terminal_update(job_id, terminal, recovery)

    case commit_runtime_state(job_id, runtime, update) do
      {:ok, _job} -> :ok
      :already_terminal -> :ok
      error -> error
    end
  end

  def block_legacy(job_id, runtime) do
    update = %{
      job_id: job_id,
      stage: "failed",
      progress: 1.0,
      message: "workflow recovery blocked: legacy active job has no durable execution envelope"
    }

    commit_runtime_state(job_id, runtime, update)
  end

  defp commit_runtime_state(job_id, runtime, update) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active ->
        # The decision and the job must still describe the same stored pair at commit.
        Store.apply_progress_with_result(update, job, runtime, runtime)

      {:ok, _terminal_job} ->
        :already_terminal

      :error ->
        {:error, {:job_not_found, job_id}}
    end
  end

  defp terminal_update(job_id, terminal, _recovery) when terminal in ["completed", "cancelled"],
    do: %{job_id: job_id, stage: terminal, progress: 1.0}

  defp terminal_update(job_id, terminal, recovery),
    do: %{
      job_id: job_id,
      stage: "failed",
      progress: 1.0,
      message: terminal_message(terminal, recovery)
    }

  defp terminal_message(terminal, recovery) do
    event =
      case Map.get(recovery, "history") do
        history when is_list(history) -> List.last(history)
        _ -> nil
      end

    message =
      if is_map(event) do
        diagnostic_message(event["message"], "") ||
          diagnostic_message(event["reason"], "workflow recovery blocked: ")
      end

    message || "workflow execution ended in #{terminal} state"
  end

  # Diagnostics must not prevent terminal reconciliation; retain the raw history unchanged.
  defp diagnostic_message(value, prefix) when is_binary(value) do
    if byte_size(value) + byte_size(prefix) <= ProgressEvent.message_max_bytes() and
         String.valid?(value) and String.trim(value) != "",
       do: prefix <> value
  end

  defp diagnostic_message(_, _), do: nil

  def run(state, recover_job) do
    case WorkflowJobRunner.registry_status() do
      :ok -> run_available(clear_registry_retry(state), recover_job)
      {:error, reason} -> {unavailable_summary(reason), defer_registry(state)}
    end
  end

  defp run_available(state, recover_job) do
    case FailureBoundary.run(&Store.list/0) do
      {:error, :analysis_store_unavailable} ->
        {unavailable_summary(:analysis_store_unavailable),
         Ownership.lose(state, :analysis_store_unavailable)}

      jobs when is_list(jobs) ->
        if Ownership.owner?(state) do
          jobs
          |> Enum.filter(&(&1.status == :failed))
          |> Enum.map(& &1.job_id)
          |> KyuubikiWeb.Orchestra.WorkflowCancellationDelivery.kick(
            KyuubikiWeb.Orchestra.WorkflowCancellationStore.authority(state)
          )
        end

        recover(Enum.filter(jobs, &(&1.status in @active)), state, recover_job)
    end
  end

  defp recover(jobs, state, recover_job) do
    {counts, next, incomplete_reason} =
      Enum.reduce(
        jobs,
        {%{recovered: 0, blocked: 0, skipped: 0}, state, nil},
        fn job, {counts, acc, incomplete_reason} ->
          {outcome, updated} = recover_job.(job.job_id, :process_restart, acc)

          {key, reason} =
            case outcome do
              {:unreconciled, reason} -> {:skipped, incomplete_reason || reason}
              key -> {key, incomplete_reason}
            end

          {Map.update!(counts, key, &(&1 + 1)), updated, reason}
        end
      )

    complete? =
      Ownership.owner?(next) and is_nil(next.registry_retry) and is_nil(incomplete_reason)

    reason =
      cond do
        next.registry_retry -> "workflow_runner_registry_unavailable"
        not Ownership.owner?(next) -> Ownership.snapshot(next)["last_error"]
        true -> incomplete_reason
      end

    next = %{
      next
      | recovery_runs: next.recovery_runs + if(complete?, do: 1, else: 0),
        recovered_jobs: next.recovered_jobs + counts.recovered,
        blocked_jobs: next.blocked_jobs + counts.blocked
    }

    {%{
       "status" => if(complete?, do: "completed", else: "interrupted"),
       "reason" => reason,
       "active_jobs" => length(jobs),
       "recovered" => counts.recovered,
       "blocked" => counts.blocked,
       "skipped" => counts.skipped
     }, next}
  end

  defp unavailable_summary(reason) do
    %{
      "status" => "unavailable",
      "reason" => Atom.to_string(reason),
      "active_jobs" => nil,
      "recovered" => 0,
      "blocked" => 0,
      "skipped" => 0
    }
  end
end
