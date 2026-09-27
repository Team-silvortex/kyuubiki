defmodule KyuubikiWeb.Jobs.Store do
  @moduledoc """
  Public job store facade. Uses SQLite or PostgreSQL when configured, otherwise falls back to the
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

  @doc "Applies progress only to the observed job snapshot; never rebases a stale decision."
  def apply_progress_if_current(attrs, expected),
    do: backend().apply_progress_if_current(attrs, expected)

  def assign_worker(job_id, worker_id), do: backend().assign_worker(job_id, worker_id)

  @doc "Publishes a solver result, worker and completion in one storage commit; terminal jobs win."
  def complete_with_result(job_id, worker_id, result)
      when is_binary(job_id) and byte_size(job_id) > 0 and is_binary(worker_id) and
             byte_size(worker_id) > 0 and is_map(result) and not is_struct(result),
      do: backend().complete_with_result(job_id, worker_id, result)

  def complete_with_result(_job_id, _worker_id, _result),
    do: {:error, :invalid_solver_completion}

  @doc "Atomically replaces an existing result and applies progress to their exact snapshots."
  def apply_progress_with_result(attrs, %KyuubikiWeb.Jobs.Job{} = expected, result, replacement)
      when is_map(attrs) and is_map(result) and is_map(replacement),
      do: backend().apply_progress_with_result(attrs, expected, result, replacement)

  def apply_progress_with_result(_attrs, _expected, _result, _replacement),
    do: {:error, :invalid_analysis_update}

  defp backend do
    if Storage.sql?() do
      KyuubikiWeb.Jobs.PostgresBackend
    else
      KyuubikiWeb.Jobs.MemoryBackend
    end
  end
end
