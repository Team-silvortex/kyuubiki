defmodule KyuubikiWeb.TestSupport.AnalysisCommitFault do
  @moduledoc false

  alias KyuubikiWeb.Storage

  def reject_result(job_id) do
    reject_write(job_id, "kyuubiki_analysis_results", "INSERT")
  end

  def reject_result_update(job_id),
    do: reject_write(job_id, "kyuubiki_analysis_results", "UPDATE")

  def reject_job_update(job_id), do: reject_write(job_id, "kyuubiki_jobs", "UPDATE")

  defp reject_write(job_id, table, operation) do
    repo = Storage.repo_module!()
    name = "reject_completion_#{System.unique_integer([:positive])}"
    id = String.replace(job_id, "'", "''")

    Ecto.Adapters.SQL.query!(repo, """
    CREATE TRIGGER #{name} BEFORE #{operation} ON #{table}
    WHEN NEW.job_id = '#{id}' BEGIN SELECT RAISE(ABORT, 'injected result write failure'); END
    """)

    drop = fn -> Ecto.Adapters.SQL.query!(repo, "DROP TRIGGER IF EXISTS #{name}") end
    ExUnit.Callbacks.on_exit(drop)
    drop
  end

  def pause_after_job_write(job_id) do
    repo = Storage.repo_module!()
    handler = {__MODULE__, make_ref()}
    event = Keyword.fetch!(apply(repo, :config, []), :telemetry_prefix) ++ [:query]

    :ok =
      :telemetry.attach(handler, event, &__MODULE__.pause/4, %{
        id: job_id,
        owner: self(),
        handler: handler
      })

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  def pause(_event, _measurements, metadata, config) do
    if String.starts_with?(metadata.query, "UPDATE") and
         String.contains?(metadata.query, "kyuubiki_jobs") and config.id in metadata.params do
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
