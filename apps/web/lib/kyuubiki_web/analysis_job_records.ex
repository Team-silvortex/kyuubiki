defmodule KyuubikiWeb.AnalysisJobRecords do
  @moduledoc """
  Job and result persistence helpers for analysis orchestration.
  """

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentClient

  @spec fetch_job(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_job(job_id) when is_binary(job_id) do
    case Store.get(job_id) do
      {:ok, job} ->
        payload = serialize_payload(job)

        case AnalysisResultStore.get(job_id) do
          {:ok, result} ->
            {:ok, payload |> put_has_result(true) |> Map.put("result", result)}

          :error ->
            {:ok, payload}
        end

      :error ->
        {:error, {:job_not_found, job_id}}
    end
  end

  @spec list_jobs() :: map()
  def list_jobs do
    jobs =
      Store.list()
      |> Enum.map(fn job ->
        has_result? = match?({:ok, _result}, AnalysisResultStore.get(job.job_id))

        job
        |> serialize_job()
        |> Map.put("has_result", has_result?)
      end)

    %{"jobs" => jobs}
  end

  @spec update_job(String.t(), map()) :: {:ok, map()} | {:error, term()}
  def update_job(job_id, attrs) when is_binary(job_id) and is_map(attrs) do
    case Store.update_metadata(job_id, attrs) do
      {:ok, job} -> {:ok, serialize_payload(job)}
      {:error, _reason} = error -> error
    end
  end

  @spec cancel_job(String.t()) :: {:ok, map()} | {:error, term()}
  def cancel_job(job_id) when is_binary(job_id) do
    case Store.get(job_id) do
      {:ok, job} ->
        if job.status in [:completed, :failed, :cancelled] do
          {:ok, serialize_payload(job)}
        else
          _ =
            Store.apply_progress(%{
              job_id: job_id,
              stage: "cancelled",
              progress: job.progress,
              message: "job cancelled by operator"
            })

          _ = AgentClient.cancel_job(job_id)
          fetch_job(job_id)
        end

      :error ->
        {:error, {:job_not_found, job_id}}
    end
  end

  @spec delete_job(String.t()) :: {:ok, map()} | {:error, term()}
  def delete_job(job_id) when is_binary(job_id) do
    _ = AnalysisResultStore.delete(job_id)

    case Store.delete(job_id) do
      {:ok, job} -> {:ok, %{"job" => serialize_job(job), "deleted" => true}}
      {:error, _reason} = error -> error
    end
  end

  @spec list_results() :: map()
  def list_results do
    %{"results" => AnalysisResultStore.list()}
  end

  @spec fetch_result(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_result(job_id) when is_binary(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, result} -> {:ok, %{"job_id" => job_id, "result" => result}}
      :error -> {:error, {:result_not_found, job_id}}
    end
  end

  @spec fetch_result_chunk(String.t(), String.t(), map()) :: {:ok, map()} | {:error, term()}
  def fetch_result_chunk(job_id, kind, params \\ %{})
      when is_binary(job_id) and is_binary(kind) and is_map(params) do
    with {:ok, result} <- fetch_raw_result(job_id),
         {:ok, items} <- fetch_chunk_source(result, kind),
         {:ok, offset} <- normalize_chunk_integer(params, "offset", 0),
         {:ok, limit} <- normalize_chunk_integer(params, "limit", 200) do
      safe_offset = min(offset, length(items))
      safe_limit = max(limit, 1)
      chunk = items |> Enum.drop(safe_offset) |> Enum.take(safe_limit)

      {:ok,
       %{
         "job_id" => job_id,
         "kind" => kind,
         "offset" => safe_offset,
         "limit" => safe_limit,
         "returned" => length(chunk),
         "total" => length(items),
         "items" => chunk
       }}
    end
  end

  @spec update_result(String.t(), map()) :: {:ok, map()} | {:error, term()}
  def update_result(job_id, result) when is_binary(job_id) and is_map(result) do
    :ok = AnalysisResultStore.update(job_id, result)
    fetch_result(job_id)
  end

  @spec delete_result(String.t()) :: {:ok, map()} | {:error, term()}
  def delete_result(job_id) when is_binary(job_id) do
    case AnalysisResultStore.delete(job_id) do
      {:ok, result} -> {:ok, %{"job_id" => job_id, "result" => result, "deleted" => true}}
      {:error, _reason} = error -> error
    end
  end

  defp serialize_payload(job) do
    %{"job" => serialize_job(job) |> Map.put("has_result", false)}
  end

  defp fetch_raw_result(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, result} -> {:ok, result}
      :error -> {:error, {:result_not_found, job_id}}
    end
  end

  defp fetch_chunk_source(result, "nodes") when is_map(result) do
    case Map.get(result, "nodes") do
      items when is_list(items) -> {:ok, items}
      _ -> {:error, {:unsupported_chunk_kind, "nodes"}}
    end
  end

  defp fetch_chunk_source(result, "elements") when is_map(result) do
    case Map.get(result, "elements") do
      items when is_list(items) -> {:ok, items}
      _ -> {:error, {:unsupported_chunk_kind, "elements"}}
    end
  end

  defp fetch_chunk_source(_result, kind), do: {:error, {:unsupported_chunk_kind, kind}}

  defp normalize_chunk_integer(params, key, default) do
    case Map.get(params, key, default) do
      value when is_integer(value) and value >= 0 ->
        {:ok, value}

      value when is_binary(value) ->
        case Integer.parse(value) do
          {parsed, ""} when parsed >= 0 -> {:ok, parsed}
          _ -> {:error, {:invalid_chunk_param, key}}
        end

      _ ->
        {:error, {:invalid_chunk_param, key}}
    end
  end

  defp serialize_job(job) do
    %{
      "job_id" => job.job_id,
      "project_id" => job.project_id,
      "model_version_id" => job.model_version_id,
      "simulation_case_id" => job.simulation_case_id,
      "worker_id" => job.worker_id,
      "message" => job.message,
      "status" => Atom.to_string(job.status),
      "progress" => job.progress,
      "residual" => job.residual,
      "iteration" => job.iteration,
      "created_at" => DateTime.to_iso8601(job.created_at),
      "updated_at" => DateTime.to_iso8601(job.updated_at)
    }
  end

  defp put_has_result(payload, value) do
    update_in(payload, ["job"], &Map.put(&1, "has_result", value))
  end
end
