defmodule KyuubikiWeb.AnalysisResultPostgresBackend do
  @moduledoc false

  alias KyuubikiWeb.Repo
  alias KyuubikiWeb.Storage.ResultRecord

  def put(job_id, result) when is_binary(job_id) and is_map(result) do
    attrs = %{
      job_id: job_id,
      payload: result,
      inserted_at: DateTime.utc_now(:second),
      updated_at: DateTime.utc_now(:second)
    }

    case Repo.get(ResultRecord, job_id) do
      %ResultRecord{} = record ->
        record
        |> Ecto.Changeset.change(%{payload: result, updated_at: DateTime.utc_now(:second)})
        |> Repo.update!()

      nil ->
        %ResultRecord{}
        |> Ecto.Changeset.change(attrs)
        |> Repo.insert!()
    end

    :ok
  end

  def get(job_id) when is_binary(job_id) do
    case Repo.get(ResultRecord, job_id) do
      %ResultRecord{payload: payload} -> {:ok, payload}
      nil -> :error
    end
  end

  def list do
    ResultRecord
    |> Repo.all()
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
    case Repo.get(ResultRecord, job_id) do
      %ResultRecord{} = record ->
        payload = record.payload
        Repo.delete!(record)
        {:ok, payload}

      nil ->
        {:error, {:result_not_found, job_id}}
    end
  end

  def reset do
    Repo.delete_all(ResultRecord)
    :ok
  end
end
