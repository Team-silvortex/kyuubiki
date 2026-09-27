defmodule KyuubikiWeb.Jobs.MemoryBackend do
  @moduledoc false

  alias KyuubikiWeb.AnalysisResultMutation
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

  def create_with_result(attrs, result) do
    with {:ok, job} <- Job.new(attrs),
         :ok <- Job.validate_initial_runtime(job) do
      AnalysisMemoryState.transaction(fn state ->
        cond do
          Map.has_key?(state.jobs, job.job_id) ->
            {{:error, {:job_already_exists, job.job_id}}, state}

          Map.has_key?(state.results, job.job_id) ->
            {{:error, {:result_already_exists, job.job_id}}, state}

          true ->
            {{:ok, job},
             %{
               state
               | jobs: Map.put(state.jobs, job.job_id, job),
                 results: Map.put(state.results, job.job_id, result)
             }}
        end
      end)
    end
  end

  def initialize_result(job_id, result) do
    AnalysisMemoryState.transaction(fn state ->
      with {:ok, job} <- Map.fetch(state.jobs, job_id),
           :ok <- Job.validate_initial_runtime(job),
           false <- Map.has_key?(state.results, job_id) do
        {{:ok, job}, %{state | results: Map.put(state.results, job_id, result)}}
      else
        :error -> {{:error, {:job_not_found, job_id}}, state}
        true -> {{:error, {:result_already_exists, job_id}}, state}
        {:error, _reason} = error -> {error, state}
      end
    end)
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

  def delete_with_result(job_id) do
    AnalysisMemoryState.transaction(fn state ->
      case Map.pop(state.jobs, job_id) do
        {nil, _jobs} ->
          {{:error, {:job_not_found, job_id}}, state}

        {job, jobs} ->
          {{:ok, job}, %{state | jobs: jobs, results: Map.delete(state.results, job_id)}}
      end
    end)
  end

  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs) do
      update_job(event.job_id, &Job.apply_progress(&1, event))
    end
  end

  def edit_result(job_id, action) do
    AnalysisMemoryState.transaction(fn state ->
      with {:job, {:ok, job}} <- {:job, Map.fetch(state.jobs, job_id)},
           {:result, {:ok, current}} <- {:result, Map.fetch(state.results, job_id)},
           {:ok, result} <- AnalysisResultMutation.prepare(job, current, action) do
        results =
          case action do
            :delete -> Map.delete(state.results, job_id)
            {:replace, _} -> Map.put(state.results, job_id, result)
          end

        {{:ok, result}, %{state | results: results}}
      else
        {:job, :error} -> {{:error, {:job_not_found, job_id}}, state}
        {:result, :error} -> {{:error, {:result_not_found, job_id}}, state}
        {:error, _reason} = error -> {error, state}
      end
    end)
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
