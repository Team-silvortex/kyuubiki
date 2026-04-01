defmodule KyuubikiWeb.Jobs.MemoryBackend do
  @moduledoc false

  use Agent

  alias KyuubikiWeb.Persistence
  alias KyuubikiWeb.Jobs.{Job, ProgressEvent}

  def start_link(_opts) do
    Agent.start_link(fn -> load_jobs() end, name: __MODULE__)
  end

  def create(attrs) do
    with {:ok, job} <- Job.new(attrs) do
      Agent.update(__MODULE__, fn jobs ->
        updated = Map.put(jobs, job.job_id, job)
        persist_jobs(updated)
        updated
      end)

      {:ok, job}
    end
  end

  def get(job_id) do
    Agent.get(__MODULE__, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, job} -> {:ok, job}
        :error -> :error
      end
    end)
  end

  def list do
    Agent.get(__MODULE__, fn jobs ->
      jobs
      |> Map.values()
      |> Enum.sort_by(& &1.updated_at, {:desc, DateTime})
    end)
  end

  def update_metadata(job_id, attrs) when is_binary(job_id) and is_map(attrs) do
    Agent.get_and_update(__MODULE__, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, %Job{} = job} ->
          updated_job = %Job{
            job
            | project_id: Map.get(attrs, "project_id", job.project_id),
              model_version_id: Map.get(attrs, "model_version_id", job.model_version_id),
              simulation_case_id: Map.get(attrs, "simulation_case_id", job.simulation_case_id),
              message: Map.get(attrs, "message", job.message),
              updated_at: DateTime.utc_now(:second)
          }

          updated_jobs = Map.put(jobs, job_id, updated_job)
          persist_jobs(updated_jobs)
          {{:ok, updated_job}, updated_jobs}

        :error ->
          {{:error, {:job_not_found, job_id}}, jobs}
      end
    end)
  end

  def delete(job_id) when is_binary(job_id) do
    Agent.get_and_update(__MODULE__, fn jobs ->
      case Map.pop(jobs, job_id) do
        {nil, current} ->
          {{:error, {:job_not_found, job_id}}, current}

        {job, current} ->
          persist_jobs(current)
          {{:ok, job}, current}
      end
    end)
  end

  def reset do
    Agent.update(__MODULE__, fn _ ->
      persist_jobs(%{})
      %{}
    end)
  end

  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         {:ok, updated_job} <- update_job(event) do
      {:ok, updated_job}
    end
  end

  def assign_worker(job_id, worker_id) when is_binary(worker_id) and byte_size(worker_id) > 0 do
    Agent.get_and_update(__MODULE__, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, job} ->
          updated_job = %{job | worker_id: worker_id, updated_at: DateTime.utc_now(:second)}
          updated_jobs = Map.put(jobs, job_id, updated_job)
          persist_jobs(updated_jobs)
          {{:ok, updated_job}, updated_jobs}

        :error ->
          {{:error, {:job_not_found, job_id}}, jobs}
      end
    end)
  end

  defp update_job(%ProgressEvent{} = event) do
    Agent.get_and_update(__MODULE__, fn jobs ->
      case Map.fetch(jobs, event.job_id) do
        {:ok, job} ->
          updated_job = Job.apply_progress(job, event)
          updated_jobs = Map.put(jobs, event.job_id, updated_job)
          persist_jobs(updated_jobs)
          {{:ok, updated_job}, updated_jobs}

        :error ->
          {{:error, {:job_not_found, event.job_id}}, jobs}
      end
    end)
  end

  defp load_jobs do
    Persistence.read_json(Persistence.jobs_path(), %{})
    |> Enum.reduce(%{}, fn
      {job_id, attrs}, acc ->
        case Job.from_persisted_map(attrs) do
          {:ok, job} -> Map.put(acc, job_id, job)
          {:error, _reason} -> acc
        end
    end)
  end

  defp persist_jobs(jobs) do
    payload =
      jobs
      |> Enum.into(%{}, fn {job_id, job} -> {job_id, Job.to_persisted_map(job)} end)

    Persistence.write_json!(Persistence.jobs_path(), payload)
  end
end
