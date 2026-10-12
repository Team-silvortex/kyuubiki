defmodule KyuubikiWeb.WorkflowSweepJoin do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowSweepResultChecks, as: Result

  def run(payload, config) do
    Check.run("transform.join_parameter_sweep_results", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      cases = Sweep.values(payload["cases"], "payload.cases")
      ids = Sweep.case_ids(cases)
      Sweep.require_count(payload, "case_count", length(cases))
      results = Map.get(payload, "summaries", payload["results"]) |> Check.list("payload.results")

      result_ids =
        Enum.with_index(results)
        |> Enum.map(fn {entry, index} ->
          Sweep.object(entry, "payload.results[#{index}]")
          Sweep.optional_case_id(entry, "payload.results[#{index}]")
        end)

      keyed = Enum.count(result_ids, &(!is_nil(&1)))

      if keyed != 0 && keyed != length(results),
        do: Check.fail("payload.results cannot mix keyed and positional identities")

      positional = results != [] && keyed == 0

      if positional && length(results) != length(cases),
        do: Check.fail("positional results require exactly one result per case")

      index =
        Enum.zip(result_ids, Enum.with_index(results))
        |> Enum.reduce(%{}, fn
          {nil, _}, acc ->
            acc

          {id, entry}, acc ->
            if Map.has_key?(acc, id),
              do: Check.fail("payload.results contains duplicate case id #{id}")

            Map.put(acc, id, entry)
        end)

      positions = if positional, do: List.to_tuple(results)
      known = MapSet.new(ids)
      unmatched = Enum.reject(result_ids, &(is_nil(&1) || MapSet.member?(known, &1)))

      field =
        Sweep.text(config, "summary_field", "summary", "config")
        |> Result.metric_field("config.summary_field")

      output =
        Sweep.text(config, "output_field", "summary", "config")
        |> Result.metric_field("config.output_field")

      strict = Sweep.flag(config, "strict", false, "config")

      old_field =
        Sweep.text(payload, "joined_summary_field", "summary", "payload")
        |> Result.metric_field("payload.joined_summary_field")

      {joined, {missing, rejected, count}} =
        Enum.zip(cases, ids)
        |> Enum.with_index()
        |> Enum.map_reduce({[], [], 0}, fn {{entry, id}, position}, {missing, rejected, count} ->
          # A fresh join must not inherit usable-looking summaries from the preceding attempt.
          entry =
            entry
            |> Map.drop(["summary", "result", "result_error", output, old_field])
            |> Map.put("id", id)

          matched =
            if positional, do: {elem(positions, position), position}, else: Map.get(index, id)

          case matched do
            nil ->
              {Map.put(entry, "result_status", "missing"), {[id | missing], rejected, count}}

            {result, result_index} ->
              case extract(
                     result,
                     field,
                     Map.has_key?(config, "summary_field"),
                     positional,
                     result_index
                   ) do
                {:ok, summary} ->
                  next =
                    entry
                    |> Map.put(output, summary)
                    |> Map.put("result_status", Map.get(result, "status", "joined"))

                  {next, {missing, rejected, count + 1}}

                {:error, reason} ->
                  next =
                    entry
                    |> Map.put("result_status", "rejected")
                    |> Map.put("result_error", reason)

                  rejection = %{
                    "case_id" => id,
                    "result_index" => result_index,
                    "reason" => reason,
                    "status" => result["status"],
                    "error" => Sweep.json(result["error"], "result.error")
                  }

                  {next, {[id | missing], [rejection | rejected], count}}
              end
          end
        end)

      missing = Enum.reverse(missing)
      rejected = Enum.reverse(rejected)

      if strict && missing != [],
        do:
          Check.fail(
            "missing usable results for #{length(missing)} case(s), first #{hd(missing)}; #{inspect(List.first(rejected))}"
          )

      if strict && unmatched != [],
        do: Check.fail("unmatched result ids: #{length(unmatched)}, first #{hd(unmatched)}")

      %{
        "cases" => joined,
        "case_count" => length(cases),
        "joined_summary_count" => count,
        "missing_summary_count" => length(missing),
        "missing_case_ids" => missing,
        "rejected_result_count" => length(rejected),
        "rejected_results" => rejected,
        "unmatched_result_count" => length(unmatched),
        "unmatched_result_ids" => unmatched,
        "matching_mode" => if(positional, do: "position", else: "case_id"),
        "joined_summary_field" => output,
        "join_complete" => missing == [] && unmatched == []
      }
    end)
  end

  defp extract(result, field, explicit, positional, index) do
    Check.run("payload.results[#{index}]", fn ->
      Result.success(result, "status", "error", "result")

      selected =
        cond do
          explicit ->
            result[field]

          Map.has_key?(result, "summary") ->
            result["summary"]

          Map.has_key?(result, "result") ->
            result["result"]

          positional && !Map.has_key?(result, "status") && !Map.has_key?(result, "error") ->
            result

          true ->
            nil
        end

      summary = Sweep.object(selected, "result.#{field}")
      Check.converged(summary, "result.#{field}")
      summary
    end)
  end
end
