defmodule KyuubikiWeb.Storage.DispatchRetentionStore do
  @moduledoc false
  import Ecto.Query

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.{AnalysisMemoryState, FailureBoundary, MemorySnapshot}
  alias KyuubikiWeb.Storage.{JobRecord, ResultRecord}

  @key "_workflow_recovery"
  @schema "kyuubiki.workflow-recovery/v1"
  @terminal ~w(completed failed cancelled)
  @empty %{job_status: nil, recovery_present: false, released_state: nil}

  # Only compact retention metadata crosses the store boundary, not scientific results.
  def snapshot(ids) when is_list(ids) and length(ids) <= 512 do
    if Enum.all?(ids, &(is_binary(&1) and byte_size(&1) in 1..1024)) do
      FailureBoundary.run(fn -> read(Enum.uniq(ids)) end)
    else
      {:error, :invalid_dispatch_retention_query}
    end
  end

  def snapshot(_ids), do: {:error, :invalid_dispatch_retention_query}

  defp read([]), do: {:ok, %{}}

  defp read(ids) do
    if Storage.sql?() do
      read_sql(Storage.repo_module!(), ids)
    else
      {:ok,
       MemorySnapshot.get(AnalysisMemoryState, fn state ->
         Map.new(ids, fn id ->
           status =
             case state.jobs[id] do
               nil -> nil
               job -> to_string(job.status)
             end

           {id, Map.put(project(state.results[id]), :job_status, status)}
         end)
       end)}
    end
  end

  defp read_sql(repo, ids) when not is_nil(repo) do
    repo.transaction(fn ->
      jobs =
        Map.new(
          repo.all(from(j in JobRecord, where: j.job_id in ^ids, select: {j.job_id, j.status}))
        )

      results =
        Map.new(repo.all(result_query(ids, Storage.backend())), fn row ->
          {row.job_id, Map.delete(row, :job_id)}
        end)

      Map.new(ids, fn id ->
        {id, Map.put(Map.get(results, id, @empty), :job_status, jobs[id])}
      end)
    end)
  end

  defp project(%{@key => recovery}) do
    released =
      if is_map(recovery) and recovery["schema_version"] == @schema and
           recovery["state"] in @terminal and not Map.has_key?(recovery, "envelope") and
           not Map.has_key?(recovery, "native_dispatch_evidence") and
           not Map.has_key?(recovery, "native_result_checkpoints"),
         do: recovery["state"]

    %{@empty | recovery_present: true, released_state: released}
  end

  defp project(_result), do: @empty

  defp result_query(ids, backend) do
    present = dynamic([r], not is_nil(^json_type(backend, [@key])))
    envelope = json_type(backend, [@key, "envelope"])
    evidence = json_type(backend, [@key, "native_dispatch_evidence"])
    checkpoints = json_type(backend, [@key, "native_result_checkpoints"])

    released =
      dynamic(
        [r],
        fragment(
          "CASE WHEN ? AND ? AND ? AND ? AND ? THEN ? ELSE NULL END",
          json_extract_path(r.payload, ["_workflow_recovery", "schema_version"]) == @schema,
          json_extract_path(r.payload, ["_workflow_recovery", "state"]) in @terminal,
          is_nil(^envelope),
          is_nil(^evidence),
          is_nil(^checkpoints),
          json_extract_path(r.payload, ["_workflow_recovery", "state"])
        )
      )

    selection = %{
      job_id: dynamic([r], r.job_id),
      recovery_present: present,
      released_state: released
    }

    from(r in ResultRecord, where: r.job_id in ^ids, select: ^selection)
  end

  # JSON null is retained as a present (malformed) key, never treated as absence.
  defp json_type(:sqlite, path),
    do: dynamic([r], fragment("json_type(?, ?)", r.payload, ^("$." <> Enum.join(path, "."))))

  defp json_type(:postgres, path),
    do:
      dynamic(
        [r],
        fragment("jsonb_typeof(jsonb_extract_path(?, VARIADIC ?::text[]))", r.payload, ^path)
      )
end
