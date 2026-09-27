defmodule KyuubikiWeb.TestSupport.ResultReadProbe do
  @moduledoc false

  def pause(job_id) do
    repo = KyuubikiWeb.Storage.repo_module!()
    handler = {__MODULE__, make_ref()}
    event = Keyword.fetch!(apply(repo, :config, []), :telemetry_prefix) ++ [:query]

    :ok =
      :telemetry.attach(handler, event, &__MODULE__.intercept/4, %{
        id: job_id,
        repo: repo,
        owner: self(),
        handler: handler
      })

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  def intercept(_event, _measurements, metadata, config) do
    if String.starts_with?(metadata.query, "SELECT") and
         String.contains?(metadata.query, "kyuubiki_analysis_results") and
         config.id in metadata.params do
      :telemetry.detach(config.handler)
      protected? = apply(config.repo, :in_transaction?, [])
      send(config.owner, {:result_read_paused, config.handler, self(), protected?})

      receive do
        :resume_result_read -> :ok
      after
        5_000 -> raise "result read probe was not released"
      end
    end
  end
end
