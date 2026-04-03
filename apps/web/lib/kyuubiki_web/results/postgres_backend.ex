defmodule KyuubikiWeb.AnalysisResultPostgresBackend do
  @moduledoc false

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.ResultRecord

  def put(job_id, result) when is_binary(job_id) and is_map(result) do
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
        |> repo_update!()

      nil ->
        %ResultRecord{}
        |> Ecto.Changeset.change(attrs)
        |> repo_insert!()
    end

    :ok
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
  defp repo_insert!(changeset), do: apply(repo(), :insert!, [changeset])
  defp repo_update!(changeset), do: apply(repo(), :update!, [changeset])
  defp repo_delete!(struct), do: apply(repo(), :delete!, [struct])
  defp repo_delete_all(queryable), do: apply(repo(), :delete_all, [queryable])
end
