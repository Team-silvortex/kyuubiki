defmodule KyuubikiWeb.AnalysisResultStore do
  @moduledoc """
  In-memory storage for completed analysis payloads keyed by job id.
  """

  use Agent

  alias KyuubikiWeb.Persistence

  def start_link(_opts) do
    Agent.start_link(fn -> load_results() end, name: __MODULE__)
  end

  @spec put(String.t(), map()) :: :ok
  def put(job_id, result) when is_binary(job_id) and is_map(result) do
    Agent.update(__MODULE__, fn results ->
      updated = Map.put(results, job_id, result)
      Persistence.write_json!(Persistence.results_path(), updated)
      updated
    end)
  end

  @spec get(String.t()) :: {:ok, map()} | :error
  def get(job_id) when is_binary(job_id) do
    Agent.get(__MODULE__, fn results ->
      case Map.fetch(results, job_id) do
        {:ok, result} -> {:ok, result}
        :error -> :error
      end
    end)
  end

  @spec reset() :: :ok
  def reset do
    Agent.update(__MODULE__, fn _ ->
      Persistence.write_json!(Persistence.results_path(), %{})
      %{}
    end)
  end

  defp load_results do
    Persistence.read_json(Persistence.results_path(), %{})
  end
end
