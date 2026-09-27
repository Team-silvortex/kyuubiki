defmodule KyuubikiWeb.Jobs.MemoryBackend do
  @moduledoc false

  alias KyuubikiWeb.Jobs.{Job, ProgressEvent}
  alias KyuubikiWeb.Storage.AnalysisMemoryState

  def create(attrs) do
    with {:ok, job} <- Job.new(attrs) do
      AnalysisMemoryState.update(:jobs, fn jobs ->
        updated = Map.put(jobs, job.job_id, job)
        {{:ok, job}, updated}
      end)
    end
  end

  def get(job_id) do
    AnalysisMemoryState.get(:jobs, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, job} -> {:ok, job}
        :error -> :error
      end
    end)
  end

  def list do
    AnalysisMemoryState.get(:jobs, fn jobs ->
      jobs
      |> Map.values()
      |> Enum.sort_by(& &1.updated_at, {:desc, DateTime})
    end)
  end

  def update_metadata(job_id, attrs) when is_binary(job_id) and is_map(attrs) do
    update_job(job_id, &Job.update_metadata(&1, attrs))
  end

  def delete(job_id) when is_binary(job_id) do
    AnalysisMemoryState.update(:jobs, fn jobs ->
      case Map.pop(jobs, job_id) do
        {nil, current} ->
          {{:error, {:job_not_found, job_id}}, current}

        {job, current} ->
          {{:ok, job}, current}
      end
    end)
  end

  def reset do
    AnalysisMemoryState.update(:jobs, fn _ -> {:ok, %{}} end)
  end

  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs) do
      update_job(event.job_id, &Job.apply_progress(&1, event))
    end
  end

  def apply_progress_if_current(attrs, %Job{} = expected) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         {:ok, updated} <- Job.apply_progress(expected, event) do
      update_job(expected.job_id, fn current ->
        if current == expected,
          do: {:ok, updated},
          else: {:error, {:stale_job_snapshot, expected.job_id}}
      end)
    end
  end

  def assign_worker(job_id, worker_id) when is_binary(worker_id) and byte_size(worker_id) > 0 do
    update_job(job_id, &Job.assign_worker(&1, worker_id))
  end

  def complete_with_result(job_id, worker_id, result) do
    AnalysisMemoryState.transaction(fn state ->
      with {:ok, job} <- Map.fetch(state.jobs, job_id),
           {:ok, completed} <- Job.complete(job, worker_id),
           false <- Map.has_key?(state.results, job_id) do
        updated = %{
          jobs: Map.put(state.jobs, job_id, completed),
          results: Map.put(state.results, job_id, result)
        }

        {{:ok, completed}, updated}
      else
        :error -> {{:error, {:job_not_found, job_id}}, state}
        true -> {{:error, {:result_already_exists, job_id}}, state}
        {:error, _reason} = error -> {error, state}
      end
    end)
  end

  def apply_progress_with_result(attrs, expected, result, replacement) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         {:ok, updated} <- Job.apply_progress(expected, event) do
      AnalysisMemoryState.transaction(fn state ->
        id = expected.job_id

        cond do
          not Map.has_key?(state.jobs, id) ->
            {{:error, {:job_not_found, id}}, state}

          state.jobs[id] !== expected ->
            {{:error, {:stale_job_snapshot, id}}, state}

          not Map.has_key?(state.results, id) ->
            {{:error, {:result_not_found, id}}, state}

          state.results[id] !== result ->
            {{:error, :stale_analysis_result}, state}

          true ->
            {{:ok, updated},
             %{
               state
               | jobs: Map.put(state.jobs, id, updated),
                 results: Map.put(state.results, id, replacement)
             }}
        end
      end)
    end
  end

  defp update_job(job_id, change) do
    AnalysisMemoryState.update(:jobs, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, job} ->
          case change.(job) do
            {:ok, ^job} ->
              {{:ok, job}, jobs}

            {:ok, updated_job} ->
              updated_jobs = Map.put(jobs, job_id, updated_job)
              {{:ok, updated_job}, updated_jobs}

            {:error, reason} ->
              {{:error, reason}, jobs}
          end

        :error ->
          {{:error, {:job_not_found, job_id}}, jobs}
      end
    end)
  end
end
