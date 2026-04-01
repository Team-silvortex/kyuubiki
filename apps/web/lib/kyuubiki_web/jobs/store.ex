defmodule KyuubikiWeb.Jobs.Store do
  @moduledoc """
  Public job store facade. Uses PostgreSQL when configured, otherwise falls back to the
  local durable memory/json backend used in tests and lightweight development.
  """

  alias KyuubikiWeb.Storage

  def create(attrs), do: backend().create(attrs)
  def get(job_id), do: backend().get(job_id)
  def list, do: backend().list()
  def update_metadata(job_id, attrs), do: backend().update_metadata(job_id, attrs)
  def delete(job_id), do: backend().delete(job_id)
  def reset, do: backend().reset()
  def apply_progress(attrs), do: backend().apply_progress(attrs)
  def assign_worker(job_id, worker_id), do: backend().assign_worker(job_id, worker_id)

  defp backend do
    if Storage.postgres?() do
      KyuubikiWeb.Jobs.PostgresBackend
    else
      KyuubikiWeb.Jobs.MemoryBackend
    end
  end
end
