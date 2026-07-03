defmodule KyuubikiWeb.AnalysisResultPostgresBackend do
  @moduledoc false

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.ResultRecord

  def put(job_id, result) when is_binary(job_id) and is_map(result) do
    put(job_id, result, 3)
  end

  defp put(job_id, result, attempts_left) do
    attrs = %{
      job_id: job_id,
      payload: result,
      inserted_at: DateTime.utc_now(),
      updated_at: DateTime.utc_now()
    }

    case repo_get(ResultRecord, job_id) do
      %ResultRecord{} = record ->
        record
        |> Ecto.Changeset.change(%{payload: result, updated_at: DateTime.utc_now()})
        |> safe_repo_update()
        |> case do
          {:ok, _record} -> :ok
          {:constraint_error, error} -> retry_or_error(job_id, result, attempts_left, error)
          {:error, changeset} -> {:error, changeset}
        end

      nil ->
        %ResultRecord{}
        |> Ecto.Changeset.change(attrs)
        |> safe_repo_insert()
        |> case do
          {:ok, _record} ->
            :ok

          {:constraint_error, error} ->
            retry_or_error(job_id, result, attempts_left, error)

          {:error, changeset} ->
            retry_or_error(job_id, result, attempts_left, changeset)
        end
    end
  end

  def get(job_id) when is_binary(job_id) do
    case repo_get(ResultRecord, job_id) do
      %ResultRecord{payload: payload} -> {:ok, payload}
      nil -> :error
    end
  end

  def list do
    repo_all(ResultRecord)
    |> Enum.sort_by(& &1.job_id)
    |> Enum.map(fn record ->
      %{
        "job_id" => record.job_id,
        "result" => record.payload,
        "inserted_at" => DateTime.to_iso8601(record.inserted_at),
        "updated_at" => DateTime.to_iso8601(record.updated_at)
      }
    end)
  end

  def update(job_id, result) when is_binary(job_id) and is_map(result), do: put(job_id, result)

  def delete(job_id) when is_binary(job_id) do
    case repo_get(ResultRecord, job_id) do
      %ResultRecord{} = record ->
        payload = record.payload
        repo_delete!(record)
        {:ok, payload}

      nil ->
        {:error, {:result_not_found, job_id}}
    end
  end

  def reset do
    repo_delete_all(ResultRecord)
    :ok
  end

  defp repo do
    Storage.repo_module!()
  end

  defp repo_get(schema, id), do: apply(repo(), :get, [schema, id])
  defp repo_all(queryable), do: apply(repo(), :all, [queryable])
  defp safe_repo_insert(changeset), do: safe_repo_write(:insert, changeset)
  defp safe_repo_update(changeset), do: safe_repo_write(:update, changeset)
  defp repo_delete!(struct), do: apply(repo(), :delete!, [struct])
  defp repo_delete_all(queryable), do: apply(repo(), :delete_all, [queryable])

  defp safe_repo_write(action, changeset) do
    apply(repo(), action, [changeset])
  rescue
    error in Ecto.ConstraintError -> {:constraint_error, error}
  end

  defp retry_or_error(job_id, result, attempts_left, reason) when attempts_left > 0 do
    Process.sleep(10)
    put(job_id, result, attempts_left - 1)
  rescue
    Ecto.ConstraintError -> retry_or_error(job_id, result, attempts_left - 1, reason)
  end

  defp retry_or_error(_job_id, _result, _attempts_left, reason), do: {:error, reason}
end
