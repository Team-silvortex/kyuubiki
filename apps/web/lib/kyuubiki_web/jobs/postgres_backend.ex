defmodule KyuubikiWeb.Jobs.PostgresBackend do
  @moduledoc false

  import Ecto.Query

  alias KyuubikiWeb.Jobs.{Job, ProgressEvent}
  alias KyuubikiWeb.Repo
  alias KyuubikiWeb.Storage.JobRecord

  def create(attrs) do
    with {:ok, job} <- Job.new(attrs) do
      record_attrs =
        job
        |> Job.to_persisted_map()
        |> persisted_map_to_repo_attrs()

      %JobRecord{}
      |> Ecto.Changeset.change(record_attrs)
      |> Repo.insert()
      |> case do
        {:ok, _record} -> {:ok, job}
        {:error, changeset} -> {:error, changeset}
      end
    end
  end

  def get(job_id) do
    case Repo.get(JobRecord, job_id) do
      %JobRecord{} = record -> repo_record_to_job(record)
      nil -> :error
    end
  end

  def list do
    JobRecord
    |> order_by([job], desc: job.updated_at)
    |> Repo.all()
    |> Enum.flat_map(fn record ->
      case repo_record_to_job(record) do
        {:ok, job} -> [job]
        :error -> []
      end
    end)
  end

  def reset do
    Repo.delete_all(JobRecord)
    :ok
  end

  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         %JobRecord{} = record <- Repo.get(JobRecord, event.job_id),
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
    case Repo.get(JobRecord, job_id) do
      %JobRecord{} = record ->
        with {:ok, job} <- repo_record_to_job(record) do
          updated_job = %{job | worker_id: worker_id, updated_at: DateTime.utc_now(:second)}

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
    |> Repo.get!(job.job_id)
    |> Ecto.Changeset.change(
      job
      |> Job.to_persisted_map()
      |> persisted_map_to_repo_attrs()
    )
    |> Repo.update()
  end

  defp persisted_map_to_repo_attrs(attrs) do
    %{
      job_id: Map.fetch!(attrs, "job_id"),
      project_id: Map.fetch!(attrs, "project_id"),
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
end
