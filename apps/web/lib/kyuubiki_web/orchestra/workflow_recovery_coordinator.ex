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
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope

  @active_job_statuses [:queued, :preprocessing, :partitioning, :solving, :postprocessing]
  @call_timeout 30_000

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
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
    send(self(), :recover)

    {:ok,
     %{
       session_id: session_id(),
       max_attempts: max_attempts(),
       refs: %{},
       jobs: %{},
       progress: %{},
       recovery_runs: 0,
       recovered_jobs: 0,
       blocked_jobs: 0
     }}
  end

  @impl true
  def handle_call(
        {:initialize, job_id, graph, input_artifacts, orchestration_context, response_options},
        _from,
        state
      ) do
    result =
      with {:ok, recovery} <-
             WorkflowRecoveryEnvelope.new(
               graph,
               input_artifacts,
               Map.put(orchestration_context, "job_id", job_id),
               response_options
             ) do
        AnalysisResultStore.put(job_id, %{
          "workflow_id" => Map.get(graph, "id"),
          "current_node" => nil,
          "progress_events" => [],
          "completed_nodes" => [],
          "artifacts" => %{},
          "response_options" => response_options,
          "orchestration_context" => orchestration_context,
          WorkflowRecoveryEnvelope.internal_key() => recovery
        })
      end

    {:reply, result, state}
  end

  def handle_call({:dispatch, job_id}, _from, state) do
    {reply, next_state} = dispatch_job(job_id, :initial, state)
    {:reply, reply, next_state}
  end

  def handle_call({:record_progress, job_id, claim, progress}, _from, state) do
    if persist_progress?(progress, Map.get(state.progress, job_id)) do
      result = record_progress_if_owned(job_id, claim, progress)
      next_state = if result == :ok, do: remember_progress(state, job_id, progress), else: state
      {:reply, result, next_state}
    else
      {:reply, :ok, state}
    end
  end

  def handle_call({:commit_result, job_id, claim, result}, _from, state) do
    reply = commit_result_if_owned(job_id, claim, result)
    {:reply, reply, forget_progress(state, job_id)}
  end

  def handle_call({:fail, job_id, claim, message}, _from, state) do
    reply = fail_if_owned(job_id, claim, message)
    {:reply, reply, forget_progress(state, job_id)}
  end

  def handle_call({:cancel, job_id}, _from, state) do
    {:reply, cancel_recovery(job_id), state}
  end

  def handle_call(:recover_now, _from, state) do
    {summary, next_state} = recover_active_jobs(state)
    {:reply, summary, next_state}
  end

  def handle_call(:snapshot, _from, state) do
    {:reply,
     %{
       "session_id" => state.session_id,
       "max_attempts" => state.max_attempts,
       "tracked_jobs" => state.jobs |> Map.keys() |> Enum.sort(),
       "recovery_runs" => state.recovery_runs,
       "recovered_jobs" => state.recovered_jobs,
       "blocked_jobs" => state.blocked_jobs
     }, state}
  end

  @impl true
  def handle_info(:recover, state) do
    {_summary, next_state} = recover_active_jobs(state)
    {:noreply, next_state}
  end

  def handle_info({:recover_job, job_id, reason}, state) do
    {_outcome, next_state} = recover_job(job_id, reason, state)
    {:noreply, next_state}
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
            progress: Map.delete(state.progress, job_id)
        }

        Process.send_after(self(), {:recover_job, job_id, :runner_loss}, 10)
        {:noreply, next_state}
    end
  end

  defp recover_active_jobs(state) do
    active_jobs = Enum.filter(Store.list(), &(&1.status in @active_job_statuses))

    {counts, next_state} =
      Enum.reduce(active_jobs, {%{recovered: 0, blocked: 0, skipped: 0}, state}, fn job,
                                                                                    {counts, acc} ->
        case recover_job(job.job_id, :process_restart, acc) do
          {:recovered, updated} -> {%{counts | recovered: counts.recovered + 1}, updated}
          {:blocked, updated} -> {%{counts | blocked: counts.blocked + 1}, updated}
          {:skipped, updated} -> {%{counts | skipped: counts.skipped + 1}, updated}
        end
      end)

    next_state = %{
      next_state
      | recovery_runs: next_state.recovery_runs + 1,
        recovered_jobs: next_state.recovered_jobs + counts.recovered,
        blocked_jobs: next_state.blocked_jobs + counts.blocked
    }

    {%{
       "active_jobs" => length(active_jobs),
       "recovered" => counts.recovered,
       "blocked" => counts.blocked,
       "skipped" => counts.skipped
     }, next_state}
  end

  defp recover_job(job_id, reason, state) do
    case WorkflowJobRunner.running(job_id) do
      {:ok, pid} ->
        {:skipped, track_runner(job_id, pid, state)}

      :error ->
        case fetch_runtime(job_id) do
          {:ok, _runtime, %{"state" => terminal} = recovery}
          when terminal in ["completed", "failed", "cancelled", "recovery_blocked"] ->
            reconcile_terminal_job(job_id, terminal, recovery)
            {:skipped, state}

          {:ok, _runtime, _recovery} ->
            case dispatch_job(job_id, reason, state) do
              {{:ok, _pid}, updated} -> {:recovered, updated}
              {{:error, {:workflow_replay_blocked, _}}, updated} -> {:blocked, updated}
              {{:error, _reason}, updated} -> {:blocked, updated}
            end

          {:legacy_workflow, runtime} ->
            message =
              "workflow recovery blocked: legacy active job has no durable execution envelope"

            _ = mark_job_failed(job_id, runtime, message)
            {:blocked, state}

          :error ->
            {:skipped, state}
        end
    end
  end

  defp dispatch_job(job_id, reason, state) do
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
         :ok <- put_runtime_recovery(job_id, runtime, claimed) do
      case WorkflowJobRunner.start_claimed(job_id, claim, Map.fetch!(claimed, "envelope")) do
        {:ok, pid} ->
          {{:ok, pid}, track_runner(job_id, pid, state)}

        {:error, reason} ->
          message = "workflow runner start failed: #{format_reason(reason)}"
          _ = fail_if_owned(job_id, claim, message)
          {{:error, {:workflow_runner_start_failed, reason}}, state}
      end
    else
      {:error, {:workflow_replay_blocked, _safety} = reason} ->
        {block_recovery(job_id, reason), state}

      {:error, :workflow_recovery_attempts_exhausted = reason} ->
        {block_recovery(job_id, reason), state}

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
        {block_recovery(job_id, integrity_reason), state}

      {:error, _reason} = error ->
        {error, state}

      :error ->
        {{:error, {:workflow_recovery_not_found, job_id}}, state}

      {:legacy_workflow, _runtime} ->
        {{:error, {:legacy_workflow_recovery_unavailable, job_id}}, state}
    end
  end

  defp record_progress_if_owned(job_id, claim, %{
         "node_id" => node_id,
         "completed_nodes" => completed_nodes,
         "total_nodes" => total_nodes
       })
       when is_binary(node_id) and is_integer(completed_nodes) and is_integer(total_nodes) and
              total_nodes > 0 do
    with {:ok, job} <- active_job(job_id),
         {:ok, runtime, recovery} <- fetch_runtime(job_id),
         true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
         progress <- min(completed_nodes / total_nodes, 0.98),
         progress_event <- progress_event(node_id, completed_nodes, total_nodes, progress),
         updated_runtime <-
           runtime
           |> Map.put("current_node", node_id)
           |> Map.update("progress_events", [progress_event], fn events ->
             (List.wrap(events) ++ [progress_event]) |> Enum.take(-25)
           end),
         :ok <- AnalysisResultStore.put(job_id, updated_runtime),
         {:ok, _updated_job} <-
           Store.apply_progress(%{
             job_id: job_id,
             stage: "solving",
             progress: progress,
             iteration: completed_nodes,
             message: "completed workflow node #{node_id}"
           }) do
      _ = job
      :ok
    else
      false -> {:error, :stale_workflow_execution_claim}
      {:error, _reason} = error -> error
      :error -> {:error, {:workflow_recovery_not_found, job_id}}
      {:legacy_workflow, _runtime} -> {:error, :legacy_workflow_recovery_unavailable}
    end
  end

  defp record_progress_if_owned(_job_id, _claim, _progress),
    do: {:error, :invalid_workflow_progress}

  defp commit_result_if_owned(job_id, claim, result) do
    with {:ok, _job} <- active_job(job_id),
         {:ok, runtime, recovery} <- fetch_runtime(job_id),
         true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
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
         :ok <- AnalysisResultStore.put(job_id, final),
         {:ok, _job} <- Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0}) do
      :ok
    else
      false -> {:error, :stale_workflow_execution_claim}
      {:error, _reason} = error -> error
      :error -> {:error, {:workflow_recovery_not_found, job_id}}
      {:legacy_workflow, _runtime} -> {:error, :legacy_workflow_recovery_unavailable}
    end
  end

  defp fail_if_owned(job_id, claim, message) do
    with {:ok, runtime, recovery} <- fetch_runtime(job_id),
         true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
         failed <-
           WorkflowRecoveryEnvelope.transition(recovery, "failed", %{"message" => message}),
         :ok <- put_runtime_recovery(job_id, runtime, failed) do
      _ =
        Store.apply_progress(%{job_id: job_id, stage: "failed", progress: 1.0, message: message})

      :ok
    else
      false -> {:error, :stale_workflow_execution_claim}
      {:error, _reason} = error -> error
      :error -> {:error, {:workflow_recovery_not_found, job_id}}
      {:legacy_workflow, _runtime} -> {:error, :legacy_workflow_recovery_unavailable}
    end
  end

  defp cancel_recovery(job_id) do
    case fetch_runtime(job_id) do
      {:ok, runtime, recovery} ->
        cancelled = WorkflowRecoveryEnvelope.transition(recovery, "cancelled")
        put_runtime_recovery(job_id, runtime, cancelled)

      _ ->
        :ok
    end
  end

  defp block_recovery(job_id, reason) do
    message = "workflow recovery blocked: #{format_reason(reason)}"

    case fetch_runtime(job_id) do
      {:ok, runtime, recovery} ->
        blocked =
          WorkflowRecoveryEnvelope.transition(recovery, "recovery_blocked", %{
            "reason" => format_reason(reason),
            "next_action" => "supply_verified_checkpoint_or_resubmit"
          })

        with :ok <- put_runtime_recovery(job_id, runtime, blocked) do
          _ = mark_job_failed(job_id, runtime, message)
          {:error, normalize_block_reason(reason)}
        end

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

  defp put_runtime_recovery(job_id, runtime, recovery),
    do:
      AnalysisResultStore.put(
        job_id,
        Map.put(runtime, WorkflowRecoveryEnvelope.internal_key(), recovery)
      )

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
    _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})
    :ok
  end

  defp reconcile_terminal_job(job_id, "cancelled", _recovery) do
    _ = Store.apply_progress(%{job_id: job_id, stage: "cancelled", progress: 1.0})
    :ok
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

    _ = Store.apply_progress(%{job_id: job_id, stage: "failed", progress: 1.0, message: message})
    :ok
  end

  defp mark_job_failed(job_id, _runtime, message) do
    Store.apply_progress(%{job_id: job_id, stage: "failed", progress: 1.0, message: message})
  end

  defp track_runner(job_id, pid, state) when is_pid(pid) do
    case Map.get(state.jobs, job_id) do
      ref when is_reference(ref) ->
        state

      nil ->
        ref = Process.monitor(pid)
        %{state | refs: Map.put(state.refs, ref, job_id), jobs: Map.put(state.jobs, job_id, ref)}
    end
  end

  defp progress_event(node_id, completed_nodes, total_nodes, progress) do
    %{
      "node_id" => node_id,
      "completed_nodes" => completed_nodes,
      "total_nodes" => total_nodes,
      "progress" => progress,
      "emitted_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
    }
  end

  defp persist_progress?(%{"completed_nodes" => completed, "total_nodes" => total}, previous)
       when is_integer(completed) and is_integer(total) and total > 0 do
    now = System.monotonic_time(:millisecond)
    stride = max(div(total, 100), 1)

    is_nil(previous) or completed == total or completed - previous.completed >= stride or
      now - previous.persisted_at_ms >= 250
  end

  defp persist_progress?(_progress, _previous), do: true

  defp remember_progress(state, job_id, %{"completed_nodes" => completed}) do
    put_in(state, [:progress, job_id], %{
      completed: completed,
      persisted_at_ms: System.monotonic_time(:millisecond)
    })
  end

  defp remember_progress(state, _job_id, _progress), do: state
  defp forget_progress(state, job_id), do: %{state | progress: Map.delete(state.progress, job_id)}

  defp normalize_block_reason({:workflow_replay_blocked, safety}),
    do: {:workflow_replay_blocked, safety}

  defp normalize_block_reason(reason), do: {:workflow_recovery_blocked, reason}
  defp format_reason(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp format_reason(reason), do: inspect(reason)

  defp max_attempts do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:max_attempts, 3)
  end

  defp session_id do
    "orch-session:" <> (:crypto.strong_rand_bytes(16) |> Base.url_encode64(padding: false))
  end
end
