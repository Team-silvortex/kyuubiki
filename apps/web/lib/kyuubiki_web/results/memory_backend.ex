defmodule KyuubikiWeb.AnalysisResultMemoryBackend do
  @moduledoc false

  alias KyuubikiWeb.Storage.AnalysisMemoryState

  def put(job_id, result) when is_binary(job_id) and is_map(result) do
    AnalysisMemoryState.update(:results, fn results ->
      updated = Map.put(results, job_id, result)
      {:ok, updated}
    end)
  end

  def get(job_id) when is_binary(job_id) do
    AnalysisMemoryState.get(:results, fn results ->
      case Map.fetch(results, job_id) do
        {:ok, result} -> {:ok, result}
        :error -> :error
      end
    end)
  end

  def list do
    AnalysisMemoryState.get(:results, fn results ->
      results
      |> Enum.map(fn {job_id, payload} -> %{"job_id" => job_id, "result" => payload} end)
      |> Enum.sort_by(& &1["job_id"])
    end)
  end

  def update(job_id, result) when is_binary(job_id) and is_map(result), do: put(job_id, result)

  def compare_and_swap(job_id, expected, replacement)
      when is_binary(job_id) and is_map(expected) and is_map(replacement) do
    AnalysisMemoryState.update(:results, fn results ->
      case Map.fetch(results, job_id) do
        {:ok, ^expected} ->
          updated = Map.put(results, job_id, replacement)
          {:ok, updated}

        {:ok, _current} ->
          {{:error, :stale_analysis_result}, results}

        :error ->
          {{:error, {:result_not_found, job_id}}, results}
      end
    end)
  end

  def delete(job_id) when is_binary(job_id) do
    AnalysisMemoryState.update(:results, fn results ->
      case Map.pop(results, job_id) do
        {nil, current} ->
          {{:error, {:result_not_found, job_id}}, current}

        {result, current} ->
          {{:ok, result}, current}
      end
    end)
  end

  def reset do
    AnalysisMemoryState.update(:results, fn _ -> {:ok, %{}} end)
  end
end
