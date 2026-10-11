defmodule KyuubikiWeb.TestSupport.ExportReadInterleave do
  @moduledoc false

  def after_read(table, writer) do
    repo = KyuubikiWeb.Storage.repo_module!()
    handler = {__MODULE__, make_ref()}
    event = Keyword.fetch!(apply(repo, :config, []), :telemetry_prefix) ++ [:query]

    :ok =
      :telemetry.attach(handler, event, &__MODULE__.interleave/4, %{
        table: table,
        writer: writer,
        owner: self(),
        handler: handler
      })

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  # Commit on a second real connection after one SELECT has returned, before
  # the export can read its next collection. No sleeps or production hooks.
  def interleave(_event, _measurements, metadata, config) do
    if self() == config.owner and String.starts_with?(metadata.query, "SELECT") and
         String.contains?(metadata.query, config.table) do
      :telemetry.detach(config.handler)
      outcome = config.writer |> Task.async() |> Task.await(10_000)
      send(config.owner, {:export_writer_committed, config.handler, outcome})
    end
  end
end
