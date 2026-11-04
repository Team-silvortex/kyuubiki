defmodule KyuubikiWeb.TestSupport.ResultReadProbe do
  @moduledoc false

  def pause(job_id, opts \\ []) do
    repo = KyuubikiWeb.Storage.repo_module!()
    handler = {__MODULE__, make_ref()}
    event = Keyword.fetch!(apply(repo, :config, []), :telemetry_prefix) ++ [:query]
    read = Keyword.get(opts, :read, 1)
    true = is_integer(read) and read > 0

    :ok =
      :telemetry.attach(handler, event, &__MODULE__.intercept/4, %{
        id: job_id,
        repo: repo,
        owner: self(),
        handler: handler,
        read: read,
        after_commit: Keyword.get(opts, :after_commit, false),
        count: :atomics.new(1, signed: false)
      })

    ExUnit.Callbacks.on_exit(fn -> :telemetry.detach(handler) end)
    handler
  end

  def intercept(_event, _measurements, metadata, config) do
    query = String.upcase(String.trim(metadata.query))

    cond do
      query in ["COMMIT", "ROLLBACK"] and Process.delete(config.handler) == true ->
        if query == "COMMIT", do: pause_process(config, :result_read_committed)

      String.starts_with?(query, "SELECT") and
        String.contains?(metadata.query, "kyuubiki_analysis_results") and
        config.id in metadata.params and :atomics.add_get(config.count, 1, 1) == config.read ->
        if config.after_commit do
          # Wait for this reader's transaction to release its database lock.
          true = apply(config.repo, :in_transaction?, [])
          Process.put(config.handler, true)
        else
          pause_process(config, :result_read_paused)
        end

      true ->
        :ok
    end
  end

  defp pause_process(config, event) do
    :telemetry.detach(config.handler)
    protected? = apply(config.repo, :in_transaction?, [])
    send(config.owner, {event, config.handler, self(), protected?})

    receive do
      :resume_result_read -> :ok
    after
      5_000 -> raise "result read probe was not released"
    end
  end
end
