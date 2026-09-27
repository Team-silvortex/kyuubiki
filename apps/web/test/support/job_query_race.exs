defmodule KyuubikiWeb.TestSupport.JobQueryRace do
  @moduledoc false

  def after_read(selector, callback, opts \\ []) do
    repo = KyuubikiWeb.Storage.repo_module!()
    handler = {__MODULE__, make_ref()}
    event = Keyword.fetch!(apply(repo, :config, []), :telemetry_prefix) ++ [:query]

    :ok =
      :telemetry.attach(handler, event, &__MODULE__.interleave/4, %{
        handler: handler,
        selector: selector,
        callback: callback,
        repeat: Keyword.get(opts, :repeat, false),
        owner: self()
      })

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  # Telemetry runs after SELECT but before its caller can validate or write the
  # returned snapshot. One-shot hooks detach before the intervening writer's SQL;
  # repeat hooks must issue only writes to avoid recursively observing themselves.
  def interleave(_event, _measurements, metadata, config) do
    if String.starts_with?(metadata.query, "SELECT") and
         String.contains?(metadata.query, "kyuubiki_jobs") and
         matches?(metadata.params, config.selector) do
      unless config.repeat, do: :telemetry.detach(config.handler)
      result = config.callback.()
      send(config.owner, {:interleaved, config.handler, result})
    end
  end

  defp matches?([id], {:job, id}), do: true
  defp matches?([], :list), do: true
  defp matches?(_params, _selector), do: false
end
