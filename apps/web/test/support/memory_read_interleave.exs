defmodule KyuubikiWeb.TestSupport.MemoryReadInterleave do
  @moduledoc false

  def after_capture(store, writer, options \\ []) do
    handler = {__MODULE__, make_ref()}

    :ok =
      :telemetry.attach(
        handler,
        [:kyuubiki, :memory_snapshot, :capture],
        &__MODULE__.interleave/4,
        %{
          handler: handler,
          store: store,
          writer: writer,
          owner: self(),
          repeat: Keyword.get(options, :repeat, false)
        }
      )

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  def interleave(_event, _measurements, metadata, config) do
    if self() == config.owner and metadata.store == config.store do
      unless config.repeat, do: :telemetry.detach(config.handler)
      result = config.writer |> Task.async() |> Task.await(10_000)
      send(config.owner, {:memory_writer_committed, config.handler, metadata.attempt, result})
    end
  end
end
