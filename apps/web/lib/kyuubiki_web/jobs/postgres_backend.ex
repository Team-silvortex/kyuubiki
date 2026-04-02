defmodule KyuubikiWeb.Jobs.PostgresBackend do
  @moduledoc false

  import Ecto.Query

  alias KyuubikiWeb.Jobs.{Job, ProgressEvent}
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.JobRecord

  def create(attrs) do
    with {:ok, job} <- Job.new(attrs) do
      record_attrs =
        job
        |> Job.to_persisted_map()
        |> persisted_map_to_repo_attrs()

      %JobRecord{}
      |> Ecto.Changeset.change(record_attrs)
      |> repo_insert()
      |> case do
        {:ok, _record} -> {:ok, job}
        {:error, changeset} -> {:error, changeset}
      end
    end
  end

  def get(job_id) do
    case repo_get(JobRecord, job_id) do
      %JobRecord{} = record -> repo_record_to_job(record)
      nil -> :error
    end
  end

  def list do
    query =
      JobRecord
      |> order_by([job], desc: job.updated_at)

    repo_all(query)
    |> Enum.flat_map(fn record ->
      case repo_record_to_job(record) do
        {:ok, job} -> [job]
        :error -> []
      end
    end)
  end

  def update_metadata(job_id, attrs) when is_binary(job_id) and is_map(attrs) do
    case repo_get(JobRecord, job_id) do
      %JobRecord{} = record ->
        changes =
          attrs
          |> Map.take(["project_id", "model_version_id", "simulation_case_id", "message"])
          |> Enum.into(%{}, fn {key, value} -> {String.to_existing_atom(key), value} end)
          |> Map.put(:updated_at, DateTime.utc_now())

        updated =
          record
          |> Ecto.Changeset.change(changes)
          |> repo_update!()

        repo_record_to_job(updated)

      nil ->
        {:error, {:job_not_found, job_id}}
    end
  end

  def delete(job_id) when is_binary(job_id) do
    case repo_get(JobRecord, job_id) do
      %JobRecord{} = record ->
        with {:ok, job} <- repo_record_to_job(record) do
          repo_delete!(record)
          {:ok, job}
        end

      nil ->
        {:error, {:job_not_found, job_id}}
    end
  end

  def reset do
    repo_delete_all(JobRecord)
    :ok
  end

  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         %JobRecord{} = record <- repo_get(JobRecord, event.job_id),
         {:ok, job} <- repo_record_to_job(record),
         updated_job <- Job.apply_progress(job, event),
         {:ok, _record} <- update_record(updated_job) do
      {:ok, updated_job}
    else
      nil -> {:error, {:job_not_found, attrs[:job_id] || attrs["job_id"]}}
      {:error, _reason} = error -> error
    end
  end

  def assign_worker(job_id, worker_id) when is_binary(worker_id) and byte_size(worker_id) > 0 do
    case repo_get(JobRecord, job_id) do
      %JobRecord{} = record ->
        with {:ok, job} <- repo_record_to_job(record) do
          updated_job = %{job | worker_id: worker_id, updated_at: DateTime.utc_now()}

          case update_record(updated_job) do
            {:ok, _record} -> {:ok, updated_job}
            {:error, changeset} -> {:error, changeset}
          end
        end

      nil ->
        {:error, {:job_not_found, job_id}}
    end
  end

  defp update_record(%Job{} = job) do
    JobRecord
    |> repo_get!(job.job_id)
    |> Ecto.Changeset.change(
      job
      |> Job.to_persisted_map()
      |> persisted_map_to_repo_attrs()
    )
    |> repo_update()
  end

  defp persisted_map_to_repo_attrs(attrs) do
    %{
      job_id: Map.fetch!(attrs, "job_id"),
      project_id: Map.fetch!(attrs, "project_id"),
      model_version_id: Map.get(attrs, "model_version_id"),
      simulation_case_id: Map.fetch!(attrs, "simulation_case_id"),
      worker_id: Map.get(attrs, "worker_id"),
      message: Map.get(attrs, "message"),
      status: Map.fetch!(attrs, "status"),
      progress: Map.fetch!(attrs, "progress"),
      residual: Map.get(attrs, "residual"),
      iteration: Map.get(attrs, "iteration"),
      created_at: parse_datetime!(Map.fetch!(attrs, "created_at")),
      updated_at: parse_datetime!(Map.fetch!(attrs, "updated_at"))
    }
  end

  defp parse_datetime!(value) when is_binary(value) do
    {:ok, datetime, _offset} = DateTime.from_iso8601(value)
    datetime
  end

  defp repo_record_to_job(%JobRecord{} = record) do
    Job.from_persisted_map(%{
      "job_id" => record.job_id,
      "project_id" => record.project_id,
      "model_version_id" => record.model_version_id,
      "simulation_case_id" => record.simulation_case_id,
      "worker_id" => record.worker_id,
      "message" => record.message,
      "status" => record.status,
      "progress" => record.progress,
      "residual" => record.residual,
      "iteration" => record.iteration,
      "created_at" => DateTime.to_iso8601(record.created_at),
      "updated_at" => DateTime.to_iso8601(record.updated_at)
    })
  end

  defp repo do
    Storage.repo_module!()
  end

  defp repo_get(schema, id), do: apply(repo(), :get, [schema, id])
  defp repo_get!(schema, id), do: apply(repo(), :get!, [schema, id])
  defp repo_all(queryable), do: apply(repo(), :all, [queryable])
  defp repo_insert(changeset), do: apply(repo(), :insert, [changeset])
  defp repo_update(changeset), do: apply(repo(), :update, [changeset])
  defp repo_update!(changeset), do: apply(repo(), :update!, [changeset])
  defp repo_delete!(struct), do: apply(repo(), :delete!, [struct])
  defp repo_delete_all(queryable), do: apply(repo(), :delete_all, [queryable])
end
