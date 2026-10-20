defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryRunners do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryScan, as: Scan

  def running(job_id, state) do
    case WorkflowJobRunner.running(job_id) do
      :error ->
        # Task startup returns before registration. A live tracked process still owns
        # that startup window; absence from Registry does not authorize a second runner.
        case state.jobs[job_id] do
          %{pid: pid} -> if Process.alive?(pid), do: {:ok, pid}, else: :error
          nil -> :error
        end

      reply ->
        reply
    end
  end

  def finalizing?(state, job_id),
    do: match?(%{purpose: :finalizing}, state.jobs[job_id])

  def staged(state, job_id) do
    case state.jobs[job_id] do
      nil -> state
      runner -> put_in(state, [:jobs, job_id], Map.put(runner, :purpose, :finalizing))
    end
  end

  def track(job_id, pid, state, purpose \\ :observed) when is_pid(pid) do
    previous = state.jobs[job_id]

    purpose =
      if purpose == :observed do
        if finalizing?(state, job_id) or Map.has_key?(state.completion_retries, job_id),
          do: :finalizing,
          else: Map.get(previous || %{}, :purpose, :executing)
      else
        purpose
      end

    state = Scan.forget_completion_retry(state, job_id)

    case previous do
      %{pid: ^pid, ref: ref} ->
        put_in(state, [:jobs, job_id], %{pid: pid, ref: ref, purpose: purpose})

      _ ->
        refs =
          if previous do
            Process.demonitor(previous.ref, [:flush])
            Map.delete(state.refs, previous.ref)
          else
            state.refs
          end

        ref = Process.monitor(pid)
        runner = %{pid: pid, ref: ref, purpose: purpose}
        %{state | refs: Map.put(refs, ref, job_id), jobs: Map.put(state.jobs, job_id, runner)}
    end
  end

  def retire(state, job_id) do
    state = Scan.forget_completion_retry(state, job_id)

    jobs =
      case state.jobs[job_id] do
        nil -> state.jobs
        runner -> Map.put(state.jobs, job_id, Map.put(runner, :purpose, :retired))
      end

    %{
      state
      | jobs: jobs,
        progress: Map.delete(state.progress, job_id),
        activity: Map.delete(state.activity, job_id)
    }
  end

  def down(ref, state) do
    case Map.pop(state.refs, ref) do
      {nil, _} ->
        state

      {job_id, refs} ->
        publication_only? = finalizing?(state, job_id)
        retired? = match?(%{purpose: :retired}, state.jobs[job_id])

        next = %{
          state
          | refs: refs,
            jobs: Map.delete(state.jobs, job_id),
            progress: Map.delete(state.progress, job_id),
            activity: Map.delete(state.activity, job_id)
        }

        cond do
          not Ownership.owner?(next) or retired? ->
            next

          publication_only? ->
            Scan.schedule_completion_retry(next, job_id)

          true ->
            Process.send_after(self(), {:recover_job, job_id, :runner_loss}, 10)
            next
        end
    end
  end
end
