defmodule KyuubikiWeb.Storage.AnalysisMemoryState do
  @moduledoc false

  use Agent

  alias KyuubikiWeb.Jobs.Job
  alias KyuubikiWeb.Persistence

  @schema "kyuubiki.analysis-memory-state/v1"

  def start_link(_opts) do
    Agent.start_link(&load!/0, name: __MODULE__)
  end

  def get(collection, reader) do
    Agent.get(__MODULE__, fn state -> reader.(Map.fetch!(state, collection)) end)
  end

  def update(collection, change) do
    transaction(fn state ->
      {reply, updated} = change.(Map.fetch!(state, collection))
      {reply, Map.put(state, collection, updated)}
    end)
  end

  def transaction(change) do
    Agent.get_and_update(__MODULE__, fn state ->
      {reply, updated} = change.(state)

      if updated === state do
        {reply, state}
      else
        commit(updated, reply, state)
      end
    end)
  end

  defp commit(updated, reply, current) do
    persist!(updated)
    {reply, updated}
  rescue
    error -> {{:error, {:completion_persistence_failed, Exception.message(error)}}, current}
  end

  defp load! do
    path = Persistence.analysis_state_path()

    # Once migrated, a damaged shared snapshot must not revive stale legacy
    # files. Persistence can recover a previous *whole* generation instead.
    if generation_exists?(path) do
      case Persistence.read_json(path, :unrecoverable) do
        %{"schema_version" => @schema, "jobs" => jobs, "results" => results}
        when is_map(jobs) and is_map(results) ->
          decode_state!(jobs, results)

        _ ->
          raise "analysis memory snapshot is invalid or unrecoverable"
      end
    else
      jobs = read_legacy(Persistence.jobs_path())
      results = read_legacy(Persistence.results_path())
      state = decode_state!(jobs, results)
      persist!(state)
      state
    end
  end

  defp generation_exists?(path),
    do: Enum.any?(["", ".previous", ".corrupt", ".recovery.json"], &File.exists?(path <> &1))

  defp read_legacy(path) do
    if generation_exists?(path), do: Persistence.read_json(path, :unrecoverable), else: %{}
  end

  defp decode_state!(jobs, results) when is_map(jobs) and is_map(results) do
    decoded =
      Map.new(jobs, fn {id, attrs} ->
        case Job.from_persisted_map(attrs) do
          {:ok, %Job{job_id: ^id} = job} -> {id, job}
          _ -> raise "invalid job in analysis memory snapshot"
        end
      end)

    unless Enum.all?(results, fn {id, result} -> is_binary(id) and is_map(result) end),
      do: raise("invalid result in analysis memory snapshot")

    %{jobs: decoded, results: results}
  end

  defp decode_state!(_jobs, _results), do: raise("invalid analysis memory snapshot")

  defp persist!(state) do
    Persistence.write_json!(Persistence.analysis_state_path(), %{
      "schema_version" => @schema,
      "jobs" => Map.new(state.jobs, fn {id, job} -> {id, Job.to_persisted_map(job)} end),
      "results" => state.results
    })
  end
end
