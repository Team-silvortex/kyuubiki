defmodule KyuubikiWeb.TestSupport.AnalysisCommitFault do
  @moduledoc false

  alias KyuubikiWeb.Storage

  def reject_result(job_id) do
    reject_write(job_id, "kyuubiki_analysis_results", "INSERT")
  end

  def reject_result_update(job_id),
    do: reject_write(job_id, "kyuubiki_analysis_results", "UPDATE")

  def reject_job_update(job_id), do: reject_write(job_id, "kyuubiki_jobs", "UPDATE")

  def reject_delete(job_id, table) when table in ["kyuubiki_jobs", "kyuubiki_analysis_results"],
    do: reject_write(job_id, table, "DELETE")

  defp reject_write(job_id, table, operation) do
    repo = Storage.repo_module!()
    name = "reject_completion_#{System.unique_integer([:positive])}"
    id = String.replace(job_id, "'", "''")
    row = if operation == "DELETE", do: "OLD", else: "NEW"

    Ecto.Adapters.SQL.query!(repo, """
    CREATE TRIGGER #{name} BEFORE #{operation} ON #{table}
    WHEN #{row}.job_id = '#{id}' BEGIN SELECT RAISE(ABORT, 'injected result write failure'); END
    """)

    drop = fn -> Ecto.Adapters.SQL.query!(repo, "DROP TRIGGER IF EXISTS #{name}") end
    ExUnit.Callbacks.on_exit(drop)
    drop
  end

  def pause_after_job_write(job_id), do: pause_after_change(job_id, "kyuubiki_jobs", "UPDATE")
  def pause_after_job_insert(job_id), do: pause_after_change(job_id, "kyuubiki_jobs", "INSERT")
  def pause_after_job_delete(job_id), do: pause_after_change(job_id, "kyuubiki_jobs", "DELETE")

  def pause_after_result_change(job_id, operation) when operation in ["UPDATE", "DELETE"],
    do: pause_after_change(job_id, "kyuubiki_analysis_results", operation)

  defp pause_after_change(job_id, table, operation) do
    repo = Storage.repo_module!()
    handler = {__MODULE__, make_ref()}
    event = Keyword.fetch!(apply(repo, :config, []), :telemetry_prefix) ++ [:query]

    :ok =
      :telemetry.attach(handler, event, &__MODULE__.pause/4, %{
        id: job_id,
        owner: self(),
        handler: handler,
        operation: operation,
        table: table
      })

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  def pause(_event, _measurements, metadata, config) do
    if String.starts_with?(metadata.query, config.operation) and
         String.contains?(metadata.query, config.table) and config.id in metadata.params do
      :telemetry.detach(config.handler)
      send(config.owner, {:completion_paused, config.handler, self()})

      receive do
        :resume_completion -> :ok
      after
        5_000 -> raise "completion fault probe was not released"
      end
    end
  end
end
