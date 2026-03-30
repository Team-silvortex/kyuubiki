defmodule KyuubikiWeb.Jobs.Store do
  @moduledoc """
  In-memory job registry used until persistence is wired in.
  """

  use Agent

  alias KyuubikiWeb.Jobs.{Job, ProgressEvent}

  def start_link(_opts) do
    Agent.start_link(fn -> %{} end, name: __MODULE__)
  end

  @spec create(map()) :: {:ok, Job.t()} | {:error, term()}
  def create(attrs) do
    with {:ok, job} <- Job.new(attrs) do
      Agent.update(__MODULE__, &Map.put(&1, job.job_id, job))
      {:ok, job}
    end
  end

  @spec get(String.t()) :: {:ok, Job.t()} | :error
  def get(job_id) do
    Agent.get(__MODULE__, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, job} -> {:ok, job}
        :error -> :error
      end
    end)
  end

  @spec list() :: [Job.t()]
  def list do
    Agent.get(__MODULE__, &Map.values/1)
  end

  @spec reset() :: :ok
  def reset do
    Agent.update(__MODULE__, fn _ -> %{} end)
  end

  @spec apply_progress(map()) :: {:ok, Job.t()} | {:error, term()}
  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         {:ok, updated_job} <- update_job(event) do
      {:ok, updated_job}
    end
  end

  @spec assign_worker(String.t(), String.t()) :: {:ok, Job.t()} | {:error, term()}
  def assign_worker(job_id, worker_id) when is_binary(worker_id) and byte_size(worker_id) > 0 do
    Agent.get_and_update(__MODULE__, fn jobs ->
      case Map.fetch(jobs, job_id) do
        {:ok, job} ->
          updated_job = %{job | worker_id: worker_id, updated_at: DateTime.utc_now(:second)}
          {{:ok, updated_job}, Map.put(jobs, job_id, updated_job)}

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
          {{:ok, updated_job}, Map.put(jobs, event.job_id, updated_job)}

        :error ->
          {{:error, {:job_not_found, event.job_id}}, jobs}
      end
    end)
  end
end
