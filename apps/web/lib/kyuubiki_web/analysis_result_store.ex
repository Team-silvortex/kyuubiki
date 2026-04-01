defmodule KyuubikiWeb.AnalysisResultStore do
  @moduledoc """
  Public result store facade. Uses PostgreSQL when configured, otherwise falls back to the
  local durable memory/json backend used in tests and lightweight development.
  """

  alias KyuubikiWeb.Storage

  def put(job_id, result), do: backend().put(job_id, result)
  def get(job_id), do: backend().get(job_id)
  def list, do: backend().list()
  def update(job_id, result), do: backend().update(job_id, result)
  def delete(job_id), do: backend().delete(job_id)
  def reset, do: backend().reset()

  defp backend do
    if Storage.postgres?() do
      KyuubikiWeb.AnalysisResultPostgresBackend
    else
      KyuubikiWeb.AnalysisResultMemoryBackend
    end
  end
end
