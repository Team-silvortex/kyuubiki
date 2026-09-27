defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator do
  @moduledoc """
  Owns durable workflow execution claims and recovers them after Orchestra restarts.

  All runtime writes pass through this process so a stale execution generation cannot
  overwrite a newer claim. The durable record remains in the configured result backend,
  so the same behavior is available with JSON, SQLite, and PostgreSQL storage.
  """

  use GenServer

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowAdmission
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.Orchestra.WorkflowNodeProgress
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryScan
  alias KyuubikiWeb.Storage.FailureBoundary

  @active_job_statuses [:queued, :preprocessing, :partitioning, :solving, :postprocessing]
  @call_timeout 30_000

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @spec admit(map(), map(), map(), map(), map()) ::
          {:ok, KyuubikiWeb.Jobs.Job.t()} | {:error, term()}
  def admit(attrs, graph, inputs, context, options)
      when is_map(attrs) and is_map(graph) and is_map(inputs) and is_map(context) and
             is_map(options) do
    GenServer.call(__MODULE__, {:admit, attrs, graph, inputs, context, options}, @call_timeout)
  end

  @spec initialize(String.t(), map(), map(), map(), map()) :: :ok | {:error, term()}
  def initialize(job_id, graph, input_artifacts, orchestration_context, response_options)
      when is_binary(job_id) and is_map(graph) and is_map(input_artifacts) and
             is_map(orchestration_context) and is_map(response_options) do
    GenServer.call(
      __MODULE__,
      {:initialize, job_id, graph, input_artifacts, orchestration_context, response_options},
      @call_timeout
    )
  end

  @spec dispatch(String.t()) :: {:ok, pid()} | {:error, term()}
  def dispatch(job_id) when is_binary(job_id) do
    GenServer.call(__MODULE__, {:dispatch, job_id}, @call_timeout)
  end

  @spec record_progress(String.t(), map(), map()) :: :ok | {:error, term()}
  def record_progress(job_id, claim, progress)
      when is_binary(job_id) and is_map(claim) and is_map(progress) do
    GenServer.call(__MODULE__, {:record_progress, job_id, claim, progress}, @call_timeout)
  end

  @spec commit_result(String.t(), map(), map()) :: :ok | {:error, term()}
  def commit_result(job_id, claim, result)
      when is_binary(job_id) and is_map(claim) and is_map(result) do
    GenServer.call(__MODULE__, {:commit_result, job_id, claim, result}, @call_timeout)
  end

  @spec fail(String.t(), map(), String.t()) :: :ok | {:error, term()}
  def fail(job_id, claim, message)
      when is_binary(job_id) and is_map(claim) and is_binary(message) do
    GenServer.call(__MODULE__, {:fail, job_id, claim, message}, @call_timeout)
  end

  @spec cancel(String.t()) :: :ok | {:error, term()}
  def cancel(job_id) when is_binary(job_id) do
    GenServer.call(__MODULE__, {:cancel, job_id}, @call_timeout)
  end

  @spec delete(String.t()) :: {:ok, KyuubikiWeb.Jobs.Job.t()} | {:error, term()}
  def delete(job_id) when is_binary(job_id) do
    GenServer.call(__MODULE__, {:delete, job_id}, @call_timeout)
  end

  @spec edit_result(String.t(), :delete | {:replace, map()}) :: {:ok, map()} | {:error, term()}
  def edit_result(job_id, action) when is_binary(job_id) do
    GenServer.call(__MODULE__, {:edit_result, job_id, action}, @call_timeout)
  end

  @spec recover_now() :: map()
  def recover_now do
    GenServer.call(__MODULE__, :recover_now, @call_timeout)
  end

  @spec snapshot() :: map()
  def snapshot do
    GenServer.call(__MODULE__, :snapshot)
  end

  @impl true
  def init(:ok) do
    Process.flag(:trap_exit, true)
    config = Ownership.config()
    lease_ttl_ms = Ownership.positive_interval(config, :lease_ttl_ms, 15_000)
    lease_heartbeat_ms = Ownership.positive_interval(config, :lease_heartbeat_ms, 5_000)

    state = %{
      session_id: session_id(),
      instance_id: LeaseStore.instance_id(),
      max_attempts: Keyword.get(config, :max_attempts, 3),
      lease_name: Keyword.get(config, :lease_name, "workflow-recovery"),
      lease_ttl_ms: lease_ttl_ms,
      lease_heartbeat_ms: min(lease_heartbeat_ms, max(div(lease_ttl_ms, 2), 1)),
      lease_retry_ms: Ownership.positive_interval(config, :lease_retry_ms, 1_000),
      lease: nil,
      lease_status: :acquiring,
      lease_holder: nil,
      lease_timer_ref: nil,
      last_lease_error: nil,
      refs: %{},
      jobs: %{},
      progress: %{},
      activity: %{},
      recovery_runs: 0,
      recovered_jobs: 0,
      blocked_jobs: 0
    }

    {:ok, Ownership.acquire(state)}
  end

  @impl true
  def handle_call({:admit, attrs, graph, inputs, context, options}, _from, state) do
    result =
      Ownership.guarded_write(state, fn ->
        WorkflowAdmission.admit(attrs, graph, inputs, context, options)
      end)

    {:reply, result, Ownership.after_write(state, result)}
  end

  def handle_call({:initialize, job_id, graph, inputs, context, options}, _from, state) do
    result =
      Ownership.guarded_write(state, fn ->
        WorkflowAdmission.initialize(job_id, graph, inputs, context, options)
      end)

    {:reply, result, Ownership.after_write(state, result)}
  end

  def handle_call({:dispatch, job_id}, _from, state) do
    {reply, next_state} =
      if Ownership.owner?(state),
        do: dispatch_job(job_id, :initial, state),
        else: {{:error, :orchestra_standby}, state}

    {:reply, reply, Ownership.after_write(next_state, reply)}
  end

  def handle_call(
        {:record_progress, job_id, claim,
         %{"event" => "operator_activity", "node_id" => node_id}},
        _from,
        state
      )
      when is_binary(node_id) do
    now = System.monotonic_time(:millisecond)
    previous = Map.get(state.activity, job_id)
    persist? = is_nil(previous) or now - previous >= 1_000

    result =
      if Ownership.owner?(state),
        do: touch_activity_if_owned(job_id, claim, node_id, state.lease, persist?),
        else: {:error, :orchestra_standby}

    next = if result == :ok and persist?, do: put_in(state, [:activity, job_id], now), else: state
    {:reply, result, Ownership.after_write(next, result)}
  end

  def handle_call({:record_progress, job_id, claim, progress}, _from, state) do
    if Ownership.owner?(state) and
         WorkflowNodeProgress.persist?(progress, Map.get(state.progress, job_id)) do
      result = record_progress_if_owned(job_id, claim, progress, state.lease)

      next_state =
        if result == :ok,
          do: WorkflowNodeProgress.remember(state, job_id, progress),
          else: state

      {:reply, result, Ownership.after_write(next_state, result)}
    else
      if Ownership.owner?(state),
        do: {:reply, :ok, state},
        else: {:reply, {:error, :orchestra_standby}, state}
    end
  end

  def handle_call({:commit_result, job_id, claim, result}, _from, state) do
    if Ownership.owner?(state) do
      reply = commit_result_if_owned(job_id, claim, result, state.lease)
      next_state = state |> forget_progress(job_id) |> Ownership.after_write(reply)
      {:reply, reply, next_state}
    else
      {:reply, {:error, :orchestra_standby}, state}
    end
  end

  def handle_call({:fail, job_id, claim, message}, _from, state) do
    if Ownership.owner?(state) do
      reply = fail_if_owned(job_id, claim, message, state.lease)
      next_state = state |> forget_progress(job_id) |> Ownership.after_write(reply)
      {:reply, reply, next_state}
    else
      {:reply, {:error, :orchestra_standby}, state}
    end
  end

  def handle_call({:cancel, job_id}, _from, state) do
    reply = Ownership.guarded_write(state, fn -> cancel_recovery(job_id) end)
    {:reply, reply, Ownership.after_write(state, reply)}
  end

  def handle_call({:delete, job_id}, _from, state) do
    reply = Ownership.guarded_write(state, fn -> Store.delete_with_result(job_id) end)

    if match?({:ok, _job}, reply) do
      case Map.get(state.jobs, job_id) do
        %{pid: pid} -> Process.exit(pid, :shutdown)
        nil -> :ok
      end
    end

    next = if match?({:ok, _job}, reply), do: forget_progress(state, job_id), else: state
    {:reply, reply, Ownership.after_write(next, reply)}
  end

  def handle_call({:edit_result, job_id, action}, _from, state) do
    reply = Ownership.guarded_write(state, fn -> Store.edit_result(job_id, action) end)
    {:reply, reply, Ownership.after_write(state, reply)}
  end

  def handle_call(:recover_now, _from, state) do
    if Ownership.owner?(state) do
      {summary, next_state} = recover_active_jobs(state)
      {:reply, summary, next_state}
    else
      {:reply, Ownership.standby_summary(state), state}
    end
  end

  def handle_call(:snapshot, _from, state) do
    {:reply,
     %{
       "session_id" => state.session_id,
       "max_attempts" => state.max_attempts,
       "lease" => Ownership.snapshot(state),
       "tracked_jobs" => state.jobs |> Map.keys() |> Enum.sort(),
       "recovery_runs" => state.recovery_runs,
       "recovered_jobs" => state.recovered_jobs,
       "blocked_jobs" => state.blocked_jobs
     }, state}
  end

  @impl true
  def handle_info(:recover, state) do
    if Ownership.owner?(state) do
      {_summary, next_state} = recover_active_jobs(state)
      {:noreply, next_state}
    else
      {:noreply, state}
    end
  end

  def handle_info(:acquire_lease, state) do
    next = if Ownership.owner?(state), do: state, else: Ownership.acquire(state)
    {:noreply, next}
  end

  def handle_info({:renew_lease, fencing_token, expires_at_ms}, state) do
    case state.lease do
      %{fencing_token: ^fencing_token, expires_at_ms: ^expires_at_ms} = lease ->
        case LeaseStore.renew(lease, state.lease_ttl_ms) do
          {:ok, renewed} ->
            next_state = %{
              state
              | lease: renewed,
                lease_status: :owner,
                lease_holder: nil,
                last_lease_error: nil
            }

            {:noreply, Ownership.schedule(next_state)}

          {:error, reason} ->
            {:noreply, Ownership.lose(state, reason)}
        end

      _ ->
        {:noreply, state}
    end
  end

  def handle_info({:recover_job, job_id, reason}, state) do
    if Ownership.owner?(state) do
      {_outcome, next_state} = recover_job(job_id, reason, state)
      {:noreply, next_state}
    else
      {:noreply, state}
    end
  end

  def handle_info({:DOWN, ref, :process, _pid, _reason}, state) do
    case Map.pop(state.refs, ref) do
      {nil, _refs} ->
        {:noreply, state}

      {job_id, refs} ->
        next_state = %{
          state
          | refs: refs,
            jobs: Map.delete(state.jobs, job_id),
            progress: Map.delete(state.progress, job_id),
            activity: Map.delete(state.activity, job_id)
        }

        if Ownership.owner?(next_state) do
          Process.send_after(self(), {:recover_job, job_id, :runner_loss}, 10)
        end

        {:noreply, next_state}
    end
  end

  @impl true
  def terminate(reason, state) do
    if Ownership.graceful_shutdown?(reason) and Ownership.owner?(state),
      do: LeaseStore.release(state.lease)

    :ok
  end

  defp recover_active_jobs(state), do: WorkflowRecoveryScan.run(state, &recover_job/3)

  defp recover_job(_job_id, _reason, %{lease_status: status} = state) when status != :owner,
    do: {:skipped, state}

  defp recover_job(job_id, reason, state) do
    case FailureBoundary.run(fn -> do_recover_job(job_id, reason, state) end) do
      {:error, :analysis_store_unavailable} ->
        {:skipped, Ownership.lose(state, :analysis_store_unavailable)}

      result ->
        result
    end
  end

  defp do_recover_job(job_id, reason, state) do
    case WorkflowJobRunner.running(job_id) do
      {:ok, pid} ->
        {:skipped, track_runner(job_id, pid, state)}

      :error ->
        case fetch_runtime(job_id) do
          {:ok, _runtime, %{"state" => terminal} = recovery}
          when terminal in ["completed", "failed", "cancelled", "recovery_blocked"] ->
            result =
              Ownership.guarded_write(state, fn ->
                reconcile_terminal_job(job_id, terminal, recovery)
              end)

            {:skipped, Ownership.after_write(state, result)}

          {:ok, _runtime, _recovery} ->
            case dispatch_job(job_id, reason, state) do
              {{:ok, _pid}, updated} ->
                {:recovered, updated}

              {{:error, {:workflow_replay_blocked, _}}, updated} ->
                {:blocked, updated}

              {{:error, {:workflow_recovery_blocked, _}}, updated} ->
                {:blocked, updated}

              {{:error, lease_error} = error, updated}
              when lease_error in [:orchestra_lease_lost, :orchestra_lease_store_unavailable] ->
                {:skipped, Ownership.after_write(updated, error)}

              {{:error, _reason}, updated} ->
                {:skipped, updated}
            end

          {:legacy_workflow, runtime} ->
            message =
              "workflow recovery blocked: legacy active job has no durable execution envelope"

            result =
              Ownership.guarded_write(state, fn -> mark_job_failed(job_id, runtime, message) end)

            outcome = if match?({:ok, _job}, result), do: :blocked, else: :skipped
            {outcome, Ownership.after_write(state, result)}

          :error ->
            {:skipped, state}
        end
    end
  end

  defp dispatch_job(job_id, reason, state) do
    case FailureBoundary.run(fn -> dispatch_available_job(job_id, reason, state) end) do
      {:error, :analysis_store_unavailable} = error ->
        {error, Ownership.after_write(state, error)}

      result ->
        result
    end
  end

  defp dispatch_available_job(job_id, reason, state) do
    case WorkflowJobRunner.running(job_id) do
      {:ok, pid} ->
        {{:ok, pid}, track_runner(job_id, pid, state)}

      :error ->
        do_dispatch_job(job_id, reason, state)
    end
  end

  defp do_dispatch_job(job_id, reason, state) do
    with {:ok, runtime, recovery} <- fetch_runtime(job_id),
         :ok <- validate_dispatch_reason(recovery, reason),
         :ok <- ensure_attempt_available(recovery, state.max_attempts),
         {:ok, claimed, claim} <-
           WorkflowRecoveryEnvelope.claim(recovery, state.session_id, reason),
         :ok <- put_runtime_recovery(job_id, runtime, claimed, state.lease) do
      case WorkflowJobRunner.start_claimed(job_id, claim, Map.fetch!(claimed, "envelope")) do
        {:ok, pid} ->
          {{:ok, pid}, track_runner(job_id, pid, state)}

        {:error, reason} ->
          message = "workflow runner start failed: #{format_reason(reason)}"
          failure_result = fail_if_owned(job_id, claim, message, state.lease)

          {{:error, {:workflow_runner_start_failed, reason}},
           Ownership.after_write(state, failure_result)}
      end
    else
      {:error, {:workflow_replay_blocked, _safety} = reason} ->
        {block_recovery(job_id, reason, state.lease), state}

      {:error, :workflow_recovery_attempts_exhausted = reason} ->
        {block_recovery(job_id, reason, state.lease), state}

      {:error, integrity_reason}
      when integrity_reason in [
             :invalid_workflow_recovery_digest,
             :workflow_recovery_digest_mismatch,
             :invalid_workflow_execution_envelope,
             :workflow_recovery_policy_mismatch,
             :workflow_recovery_identity_mismatch,
             :workflow_recovery_checkpoint_mismatch,
             :workflow_recovery_checkpoint_missing_or_invalid,
             :invalid_workflow_recovery_record
           ] ->
        {block_recovery(job_id, integrity_reason, state.lease), state}

      {:error, _reason} = error ->
        {error, state}

      :error ->
        {{:error, {:workflow_recovery_not_found, job_id}}, state}

      {:legacy_workflow, _runtime} ->
        {{:error, {:legacy_workflow_recovery_unavailable, job_id}}, state}
    end
  end

  defp touch_activity_if_owned(job_id, claim, node_id, lease, persist?) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, _runtime, recovery} <- fetch_runtime(job_id),
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim) do
        if persist? do
          # A live operator is not a completed graph node and cannot reset the execution deadline.
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

  defp record_progress_if_owned(job_id, claim, update, lease) do
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
             {:ok, _updated_job} <-
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
          {:error, _reason} = error -> error
          :error -> {:error, {:workflow_recovery_not_found, job_id}}
          {:legacy_workflow, _runtime} -> {:error, :legacy_workflow_recovery_unavailable}
        end
      end)
    end
  end

  defp commit_result_if_owned(job_id, claim, result, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, runtime, recovery} <- fetch_runtime(job_id),
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
           {:ok, failed_count} <- failed_node_count(result),
           completed <-
             WorkflowRecoveryEnvelope.transition(recovery, "completed", %{
               "committed_generation" => claim["generation"]
             }),
           final <-
             result
             |> Map.put("workflow_id", Map.get(runtime, "workflow_id"))
             |> Map.put("current_node", nil)
             |> Map.put("progress_events", Map.get(runtime, "progress_events", []))
             |> Map.put("response_options", Map.get(runtime, "response_options", %{}))
             |> Map.put(WorkflowRecoveryEnvelope.internal_key(), completed),
           {:ok, _job} <-
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
             ) do
        :ok
      else
        false -> {:error, :stale_workflow_execution_claim}
        {:error, _reason} = error -> error
        :error -> {:error, {:workflow_recovery_not_found, job_id}}
        {:legacy_workflow, _runtime} -> {:error, :legacy_workflow_recovery_unavailable}
      end
    end)
  end

  defp fail_if_owned(job_id, claim, message, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id),
           {:ok, runtime, recovery} <- fetch_runtime(job_id),
           true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
           failed <-
             WorkflowRecoveryEnvelope.transition(recovery, "failed", %{"message" => message}) do
        update_recovery(job, runtime, failed, %{stage: "failed", progress: 1.0, message: message})
      else
        false -> {:error, :stale_workflow_execution_claim}
        {:error, _reason} = error -> error
        :error -> {:error, {:workflow_recovery_not_found, job_id}}
        {:legacy_workflow, _runtime} -> {:error, :legacy_workflow_recovery_unavailable}
      end
    end)
  end

  defp cancel_recovery(job_id) do
    case active_job(job_id) do
      {:ok, job} ->
        attrs = %{
          stage: "cancelled",
          progress: job.progress,
          message: "job cancelled by operator"
        }

        case fetch_runtime(job_id) do
          {:ok, runtime, recovery} ->
            cancelled = WorkflowRecoveryEnvelope.transition(recovery, "cancelled")
            update_recovery(job, runtime, cancelled, attrs)

          _ ->
            Store.apply_progress_if_current(Map.put(attrs, :job_id, job_id), job)
            |> progress_reply()
        end

      {:error, {:workflow_job_terminal, _status}} ->
        :ok

      error ->
        error
    end
  end

  defp block_recovery(job_id, reason, lease) do
    message = "workflow recovery blocked: #{format_reason(reason)}"

    case fetch_runtime(job_id) do
      {:ok, runtime, recovery} ->
        blocked =
          WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", %{
            "reason" => format_reason(reason),
            "next_action" => "supply_verified_checkpoint_or_resubmit"
          })

        LeaseStore.with_lease(lease, fn ->
          with {:ok, job} <- active_job(job_id),
               :ok <-
                 update_recovery(job, runtime, blocked, %{
                   stage: "failed",
                   progress: 1.0,
                   message: message
                 }) do
            {:error, normalize_block_reason(reason)}
          end
        end)

      _ ->
        {:error, reason}
    end
  end

  defp fetch_runtime(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, runtime} when is_map(runtime) ->
        case Map.get(runtime, WorkflowRecoveryEnvelope.internal_key()) do
          recovery when is_map(recovery) ->
            {:ok, runtime, recovery}

          _ ->
            if is_binary(Map.get(runtime, "workflow_id")),
              do: {:legacy_workflow, runtime},
              else: :error
        end

      _ ->
        :error
    end
  end

  defp put_runtime_recovery(job_id, runtime, recovery, lease) do
    LeaseStore.with_lease(lease, fn ->
      with {:ok, job} <- active_job(job_id), do: update_recovery(job, runtime, recovery)
    end)
  end

  defp update_recovery(job, runtime, recovery, changes \\ %{}) do
    attrs = Map.merge(%{job_id: job.job_id, stage: job.status, progress: job.progress}, changes)
    replacement = Map.put(runtime, WorkflowRecoveryEnvelope.internal_key(), recovery)
    Store.apply_progress_with_result(attrs, job, runtime, replacement) |> progress_reply()
  end

  defp failed_node_count(result) do
    case Map.get(result, "failed_nodes", []) do
      nodes when is_list(nodes) -> {:ok, length(nodes)}
      _ -> {:error, {:invalid_workflow_result, :failed_nodes}}
    end
  end

  defp progress_reply({:ok, _job}), do: :ok
  defp progress_reply(error), do: error

  defp active_job(job_id) do
    case Store.get(job_id) do
      {:ok, %{status: status} = job} when status in @active_job_statuses -> {:ok, job}
      {:ok, job} -> {:error, {:workflow_job_terminal, job.status}}
      :error -> {:error, {:job_not_found, job_id}}
    end
  end

  defp validate_dispatch_reason(%{"state" => "pending"}, :initial), do: :ok
  defp validate_dispatch_reason(_recovery, :initial), do: {:error, :workflow_already_dispatched}
  defp validate_dispatch_reason(_recovery, _reason), do: :ok

  defp ensure_attempt_available(%{"attempt" => attempt}, max_attempts)
       when is_integer(attempt) and attempt < max_attempts,
       do: :ok

  defp ensure_attempt_available(_recovery, _max_attempts),
    do: {:error, :workflow_recovery_attempts_exhausted}

  defp reconcile_terminal_job(job_id, "completed", _recovery) do
    Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0}) |> progress_reply()
  end

  defp reconcile_terminal_job(job_id, "cancelled", _recovery) do
    Store.apply_progress(%{job_id: job_id, stage: "cancelled", progress: 1.0}) |> progress_reply()
  end

  defp reconcile_terminal_job(job_id, terminal, recovery) do
    message =
      recovery
      |> Map.get("history", [])
      |> List.last()
      |> case do
        %{"message" => value} when is_binary(value) -> value
        %{"reason" => value} when is_binary(value) -> "workflow recovery blocked: #{value}"
        _ -> "workflow execution ended in #{terminal} state"
      end

    Store.apply_progress(%{job_id: job_id, stage: "failed", progress: 1.0, message: message})
    |> progress_reply()
  end

  defp mark_job_failed(job_id, _runtime, message) do
    Store.apply_progress(%{job_id: job_id, stage: "failed", progress: 1.0, message: message})
  end

  defp track_runner(job_id, pid, state) when is_pid(pid) do
    case Map.get(state.jobs, job_id) do
      %{ref: ref} when is_reference(ref) ->
        state

      nil ->
        ref = Process.monitor(pid)

        runner = %{pid: pid, ref: ref}

        %{
          state
          | refs: Map.put(state.refs, ref, job_id),
            jobs: Map.put(state.jobs, job_id, runner)
        }
    end
  end

  defp forget_progress(state, job_id),
    do: %{
      state
      | progress: Map.delete(state.progress, job_id),
        activity: Map.delete(state.activity, job_id)
    }

  defp normalize_block_reason({:workflow_replay_blocked, safety}),
    do: {:workflow_replay_blocked, safety}

  defp normalize_block_reason(reason), do: {:workflow_recovery_blocked, reason}
  defp format_reason(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp format_reason(reason), do: inspect(reason)

  defp session_id do
    "orch-session:" <> (:crypto.strong_rand_bytes(16) |> Base.url_encode64(padding: false))
  end
end
