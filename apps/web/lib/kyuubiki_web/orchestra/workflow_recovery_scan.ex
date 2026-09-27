defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryScan do
  @moduledoc false

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership
  alias KyuubikiWeb.Storage.FailureBoundary

  @active [:queued, :preprocessing, :partitioning, :solving, :postprocessing]

  def run(state, recover_job) do
    case FailureBoundary.run(&Store.list/0) do
      {:error, :analysis_store_unavailable} ->
        {%{
           "status" => "unavailable",
           "reason" => "analysis_store_unavailable",
           "active_jobs" => nil,
           "recovered" => 0,
           "blocked" => 0,
           "skipped" => 0
         }, Ownership.lose(state, :analysis_store_unavailable)}

      jobs when is_list(jobs) ->
        recover(Enum.filter(jobs, &(&1.status in @active)), state, recover_job)
    end
  end

  defp recover(jobs, state, recover_job) do
    {counts, next} =
      Enum.reduce(jobs, {%{recovered: 0, blocked: 0, skipped: 0}, state}, fn job, {counts, acc} ->
        {outcome, updated} = recover_job.(job.job_id, :process_restart, acc)
        {Map.update!(counts, outcome, &(&1 + 1)), updated}
      end)

    complete? = Ownership.owner?(next)

    next = %{
      next
      | recovery_runs: next.recovery_runs + if(complete?, do: 1, else: 0),
        recovered_jobs: next.recovered_jobs + counts.recovered,
        blocked_jobs: next.blocked_jobs + counts.blocked
    }

    {%{
       "status" => if(complete?, do: "completed", else: "interrupted"),
       "reason" => Ownership.snapshot(next)["last_error"],
       "active_jobs" => length(jobs),
       "recovered" => counts.recovered,
       "blocked" => counts.blocked,
       "skipped" => counts.skipped
     }, next}
  end
end
