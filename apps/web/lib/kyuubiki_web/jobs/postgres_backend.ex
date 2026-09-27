defmodule KyuubikiWeb.Jobs.PostgresBackend do
  @moduledoc false

  import Ecto.Query

  alias KyuubikiWeb.AnalysisResultMutation
  alias KyuubikiWeb.AnalysisResultPostgresBackend, as: Results
  alias KyuubikiWeb.Jobs.{Job, ProgressEvent}
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.{JobRecord, ResultRecord}

  @write_attempts 4

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

  def create_with_result(attrs, result) do
    with {:ok, proposed} <- Job.new(attrs),
         :ok <- Job.validate_initial_runtime(proposed) do
      atomic_change(fn ->
        if is_nil(repo_get(JobRecord, proposed.job_id)) do
          with {:ok, job} <- create(attrs),
               :ok <- insert_completion_result(job.job_id, result),
               do: {:ok, job}
        else
          {:error, {:job_already_exists, proposed.job_id}}
        end
      end)
    end
  end

  def initialize_result(job_id, result) do
    atomic_change(fn ->
      with {:ok, job} <- get(job_id),
           :ok <- Job.validate_initial_runtime(job) do
        # Even without changing job fields, hold its row lock until the initial
        # result is committed so cancellation/deletion cannot pass between them.
        case compare_and_swap(job_attrs(job), job, true) do
          :ok ->
            with :ok <- insert_completion_result(job_id, result), do: {:ok, job}

          :stale ->
            snapshot_miss(job_id)
        end
      else
        :error -> {:error, {:job_not_found, job_id}}
        {:error, _reason} = error -> error
      end
    end)
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
        {:error, _reason} -> []
      end
    end)
  end

  def update_metadata(job_id, attrs) when is_binary(job_id) and is_map(attrs) do
    mutate(job_id, &Job.update_metadata(&1, attrs))
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

  def delete_with_result(job_id) do
    atomic_change(fn ->
      with {:ok, job} <- get(job_id) do
        case compare_and_swap(job_attrs(job), job, true) do
          :ok ->
            # The FK cascade removes the result in this same transaction. Lock
            # the observed job first so concurrent publication cannot pass it.
            {1, _} = repo_delete_all(where(JobRecord, [record], record.job_id == ^job_id))
            {:ok, job}

          :stale ->
            snapshot_miss(job_id)
        end
      else
        :error -> {:error, {:job_not_found, job_id}}
        {:error, _reason} = error -> error
      end
    end)
  end

  def apply_progress(attrs) do
    with {:ok, event} <- ProgressEvent.new(attrs) do
      # Parse once: a retry must not refresh an old event's emitted_at.
      mutate(event.job_id, &Job.apply_progress(&1, event))
    end
  end

  def edit_result(job_id, action) do
    atomic_change(fn ->
      with {:job, {:ok, job}} <- {:job, get(job_id)},
           :ok <- compare_and_swap(job_attrs(job), job, true),
           {:result, {:ok, current}} <- {:result, Results.get(job_id)},
           {:ok, result} <- AnalysisResultMutation.prepare(job, current, action),
           :ok <- commit_result_edit(job_id, current, action, result) do
        {:ok, result}
      else
        {:job, :error} -> {:error, {:job_not_found, job_id}}
        {:job, {:error, _} = error} -> error
        {:result, :error} -> {:error, {:result_not_found, job_id}}
        :stale -> snapshot_miss(job_id)
        {:error, _reason} = error -> error
      end
    end)
  end

  defp commit_result_edit(job_id, current, :delete, _result),
    do: Results.delete_if_current(job_id, current)

  defp commit_result_edit(job_id, current, {:replace, _}, result),
    do: Results.compare_and_swap(job_id, current, result)

  def apply_progress_if_current(attrs, %Job{} = expected) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         {:ok, updated} <- Job.apply_progress(expected, event) do
      case compare_and_swap(job_attrs(expected), updated) do
        :ok -> {:ok, updated}
        :stale -> snapshot_miss(expected.job_id)
      end
    end
  end

  def assign_worker(job_id, worker_id) when is_binary(worker_id) and byte_size(worker_id) > 0 do
    mutate(job_id, &Job.assign_worker(&1, worker_id))
  end

  def complete_with_result(job_id, worker_id, result) do
    atomic_change(fn ->
      with {:ok, completed} <- mutate(job_id, &Job.complete(&1, worker_id)),
           :ok <- insert_completion_result(job_id, result),
           do: {:ok, completed}
    end)
  end

  def apply_progress_with_result(attrs, expected, result, replacement) do
    with {:ok, event} <- ProgressEvent.new(attrs),
         {:ok, updated} <- Job.apply_progress(expected, event) do
      atomic_change(fn ->
        case compare_and_swap(job_attrs(expected), updated, true) do
          :ok ->
            with :ok <-
                   KyuubikiWeb.AnalysisResultPostgresBackend.compare_and_swap(
                     expected.job_id,
                     result,
                     replacement
                   ),
                 do: {:ok, updated}

          :stale ->
            snapshot_miss(expected.job_id)
        end
      end)
    end
  end

  defp atomic_change(change) do
    # SQLite must reserve its writer before reading. PostgreSQL's conditional
    # job UPDATE serializes competing completions/cancellation on that row.
    options = if Storage.sqlite?(), do: [mode: :immediate], else: []

    commit = fn ->
      case change.() do
        {:ok, value} -> value
        {:error, reason} -> apply(repo(), :rollback, [reason])
      end
    end

    # A workflow lease already owns the SQL transaction. Roll back that boundary
    # directly instead of hiding an aborted nested transaction behind an error tuple.
    if apply(repo(), :in_transaction?, []),
      do: {:ok, commit.()},
      else: apply(repo(), :transaction, [commit, options])
  rescue
    error ->
      reason = {:completion_persistence_failed, Exception.message(error)}

      if apply(repo(), :in_transaction?, []),
        do: apply(repo(), :rollback, [reason]),
        else: {:error, reason}
  end

  defp insert_completion_result(job_id, result) do
    case repo_get(ResultRecord, job_id) do
      nil ->
        %ResultRecord{}
        |> Ecto.Changeset.change(%{job_id: job_id, payload: result})
        |> repo_insert()
        |> case do
          {:ok, _record} -> :ok
          {:error, reason} -> {:error, reason}
        end

      %ResultRecord{} ->
        {:error, {:result_already_exists, job_id}}
    end
  end

  defp mutate(job_id, change, attempts \\ @write_attempts) do
    case repo_get(JobRecord, job_id) do
      %JobRecord{} = record ->
        with {:ok, job} <- repo_record_to_job(record),
             {:ok, updated} <- change.(job) do
          expected = Map.take(record, JobRecord.__schema__(:fields))

          cond do
            updated == job -> {:ok, job}
            compare_and_swap(expected, updated) == :ok -> {:ok, updated}
            attempts > 1 -> mutate(job_id, change, attempts - 1)
            true -> {:error, {:job_write_conflict, job_id}}
          end
        end

      nil ->
        {:error, {:job_not_found, job_id}}
    end
  end

  defp compare_and_swap(expected, updated, lock_unchanged? \\ false) do
    # Match every persisted field, not just time: equal-time progress, ownership
    # and metadata writes must also invalidate the snapshot used for validation.
    query =
      Enum.reduce(expected, JobRecord, fn
        {key, nil}, query -> where(query, [job], is_nil(field(job, ^key)))
        {key, value}, query -> where(query, [job], field(job, ^key) == ^value)
      end)

    changes = Enum.reject(job_attrs(updated), fn {key, value} -> expected[key] == value end)
    # A result-only replacement still needs the job row locked until commit.
    changes =
      if changes == [] and lock_unchanged?, do: [progress: expected.progress], else: changes

    case changes do
      [] ->
        if apply(repo(), :exists?, [query]), do: :ok, else: :stale

      _ ->
        case apply(repo(), :update_all, [query, [set: changes]]) do
          {1, _} -> :ok
          {0, _} -> :stale
        end
    end
  end

  defp snapshot_miss(job_id) do
    case repo_get(JobRecord, job_id) do
      nil -> {:error, {:job_not_found, job_id}}
      %JobRecord{} -> {:error, {:stale_job_snapshot, job_id}}
    end
  end

  defp job_attrs(job), do: job |> Job.to_persisted_map() |> persisted_map_to_repo_attrs()

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
      queue_timeout_ms: Map.get(attrs, "queue_timeout_ms"),
      execution_timeout_ms: Map.get(attrs, "execution_timeout_ms"),
      execution_started_at: parse_optional_datetime(Map.get(attrs, "execution_started_at")),
      created_at: parse_datetime!(Map.fetch!(attrs, "created_at")),
      updated_at: parse_datetime!(Map.fetch!(attrs, "updated_at"))
    }
  end

  defp parse_datetime!(value) when is_binary(value) do
    {:ok, datetime, _offset} = DateTime.from_iso8601(value)
    # utc_datetime_usec requires six-digit precision, including whole seconds.
    %{datetime | microsecond: {elem(datetime.microsecond, 0), 6}}
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
      "queue_timeout_ms" => record.queue_timeout_ms,
      "execution_timeout_ms" => record.execution_timeout_ms,
      "execution_started_at" => format_datetime(record.execution_started_at),
      "created_at" => DateTime.to_iso8601(record.created_at),
      "updated_at" => DateTime.to_iso8601(record.updated_at)
    })
  end

  defp repo do
    Storage.repo_module!()
  end

  defp repo_get(schema, id), do: apply(repo(), :get, [schema, id])
  defp repo_all(queryable), do: apply(repo(), :all, [queryable])
  defp repo_insert(changeset), do: apply(repo(), :insert, [changeset])

  defp repo_delete!(struct), do: apply(repo(), :delete!, [struct])
  defp repo_delete_all(queryable), do: apply(repo(), :delete_all, [queryable])

  defp parse_optional_datetime(nil), do: nil
  defp parse_optional_datetime(value), do: parse_datetime!(value)

  defp format_datetime(%DateTime{} = value), do: DateTime.to_iso8601(value)
  defp format_datetime(_value), do: nil
end
