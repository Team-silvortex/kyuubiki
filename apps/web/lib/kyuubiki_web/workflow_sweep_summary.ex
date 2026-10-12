defmodule KyuubikiWeb.WorkflowSweepSummary do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowSweepResultChecks, as: Result

  def run(payload, config) do
    Check.run("transform.summarize_parameter_sweep", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      Result.complete(payload)
      cases = Sweep.values(payload["cases"], "payload.cases")
      ids = Sweep.case_ids(cases)

      for key <- ~w(case_count joined_summary_count),
          do: Sweep.require_count(payload, key, length(cases))

      joined_field =
        Sweep.text(payload, "joined_summary_field", "summary", "payload")
        |> Result.metric_field("payload.joined_summary_field")

      field =
        Sweep.text(config, "summary_field", joined_field, "config")
        |> Result.metric_field("config.summary_field")

      if Map.has_key?(payload, "joined_summary_field") && field != joined_field,
        do: Check.fail("config.summary_field disagrees with joined_summary_field")

      parameters = Sweep.flag(config, "include_parameters", true, "config")
      metadata = Sweep.flag(config, "include_metadata", true, "config")
      strict = Sweep.flag(config, "fail_on_missing", true, "config")

      fallback =
        !Map.has_key?(config, "summary_field") && !Map.has_key?(payload, "joined_summary_field")

      summaries =
        Enum.zip(cases, ids)
        |> Enum.map(fn {entry, id} ->
          Result.success(entry, "result_status", "result_error", "case #{id}")
          value = Map.get(entry, field, if(fallback, do: entry["result"]))
          summary = Sweep.object(value, "case #{id}.#{field}")
          Check.converged(summary, "case #{id}.#{field}")
          summary
        end)

      fields = selected_fields(summaries, config)

      numeric =
        for summary <- summaries,
            {key, value} <- summary,
            is_number(value),
            into: MapSet.new(),
            do: key

      {rows, {columns, missing}} =
        Enum.zip([cases, ids, summaries])
        |> Enum.map_reduce({%{}, []}, fn {entry, id, summary}, {columns, missing} ->
          row = %{"case_id" => id}
          row = if parameters, do: Map.put(row, "parameters", entry["parameters"]), else: row
          row = if metadata, do: Map.put(row, "metadata", entry["metadata"]), else: row

          {row, columns, missing} =
            Enum.reduce(fields, {row, columns, missing}, fn key, {row, columns, missing} ->
              value = summary[key]

              reason =
                cond do
                  !Map.has_key?(summary, key) -> "missing"
                  is_nil(value) -> "null"
                  MapSet.member?(numeric, key) && !is_number(value) -> "non_numeric"
                  true -> nil
                end

              if strict && reason, do: Check.fail("case #{id} field #{key} is #{reason}")

              missing =
                if reason,
                  do: [%{"case_id" => id, "field" => key, "reason" => reason} | missing],
                  else: missing

              row = if Map.has_key?(summary, key), do: Map.put(row, key, value), else: row

              columns =
                if is_number(value), do: add_column(columns, key, value, id), else: columns

              {row, columns, missing}
            end)

          {row, {columns, missing}}
        end)

      %{
        "rows" => rows,
        "row_count" => length(rows),
        "numeric_columns" => columns,
        "summary_complete" => missing == [],
        "missing_field_count" => length(missing),
        "missing_fields" => Enum.reverse(missing)
      }
    end)
  end

  defp selected_fields(summaries, config) do
    fields =
      if Map.has_key?(config, "fields"),
        do: Sweep.values(config["fields"], "config.fields"),
        else: summaries |> Enum.flat_map(&Map.keys/1) |> Enum.uniq() |> Enum.sort()

    if fields == [], do: Check.fail("summaries contain no selected fields")

    fields =
      Enum.with_index(fields)
      |> Enum.map(fn {field, index} ->
        Sweep.text(%{"field" => field}, "field", nil, "config.fields[#{index}]")
        |> Result.metric_field("summary field")
      end)

    if length(Enum.uniq(fields)) != length(fields),
      do: Check.fail("config.fields contains duplicate fields")

    fields
  end

  defp add_column(columns, key, value, id) do
    value = Check.number(value, "case #{id}.#{key}")
    old = Map.get(columns, key, %{"count" => 0, "sum" => 0.0, "min" => value, "max" => value})
    count = old["count"] + 1
    sum = Check.calculate("case #{id} numeric column #{key}", fn -> old["sum"] + value end)

    Map.put(columns, key, %{
      "count" => count,
      "sum" => sum,
      "min" => min(old["min"], value),
      "max" => max(old["max"], value),
      "mean" => sum / count
    })
  end
end
