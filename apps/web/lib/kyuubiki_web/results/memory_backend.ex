defmodule KyuubikiWeb.AnalysisResultMemoryBackend do
  @moduledoc false

  use Agent

  alias KyuubikiWeb.Persistence

  def start_link(_opts) do
    Agent.start_link(fn -> load_results() end, name: __MODULE__)
  end

  def put(job_id, result) when is_binary(job_id) and is_map(result) do
    Agent.update(__MODULE__, fn results ->
      updated = Map.put(results, job_id, result)
      Persistence.write_json!(Persistence.results_path(), updated)
      updated
    end)
  end

  def get(job_id) when is_binary(job_id) do
    Agent.get(__MODULE__, fn results ->
      case Map.fetch(results, job_id) do
        {:ok, result} -> {:ok, result}
        :error -> :error
      end
    end)
  end

  def list do
    Agent.get(__MODULE__, fn results ->
      results
      |> Enum.map(fn {job_id, payload} -> %{"job_id" => job_id, "result" => payload} end)
      |> Enum.sort_by(& &1["job_id"])
    end)
  end

  def update(job_id, result) when is_binary(job_id) and is_map(result), do: put(job_id, result)

  def delete(job_id) when is_binary(job_id) do
    Agent.get_and_update(__MODULE__, fn results ->
      case Map.pop(results, job_id) do
        {nil, current} ->
          {{:error, {:result_not_found, job_id}}, current}

        {result, current} ->
          Persistence.write_json!(Persistence.results_path(), current)
          {{:ok, result}, current}
      end
    end)
  end

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
