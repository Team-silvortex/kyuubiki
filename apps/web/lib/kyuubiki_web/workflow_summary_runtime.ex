defmodule KyuubikiWeb.WorkflowSummaryRuntime do
  @moduledoc false

  def merge_summary_pair(%{"left" => left, "right" => right}, config)
      when is_map(left) and is_map(right) and is_map(config) do
    merged =
      namespaced_summary_fields(left, normalize_prefix(Map.get(config, "left_prefix", "left")))
      |> Map.merge(
        namespaced_summary_fields(
          right,
          normalize_prefix(Map.get(config, "right_prefix", "right"))
        )
      )
      |> maybe_put(Map.get(config, "include_source_count", true), "summary_source_count", 2)

    if map_size(merged) == 0, do: {:error, :empty_summary}, else: {:ok, merged}
  end

  def merge_summary_pair(_payload, _config), do: {:error, :invalid_summary_pair}

  def compare_summary_pair(%{"left" => left, "right" => right}, config)
      when is_map(left) and is_map(right) and is_map(config) do
    with {:ok, fields} <- resolve_summary_compare_fields(left, right, config) do
      left_prefix = normalize_prefix(Map.get(config, "left_prefix", "left"))
      right_prefix = normalize_prefix(Map.get(config, "right_prefix", "right"))
      delta_prefix = normalize_prefix(Map.get(config, "delta_prefix", "delta"))
      ratio_prefix = normalize_prefix(Map.get(config, "ratio_prefix", "ratio"))
      percent_prefix = normalize_prefix(Map.get(config, "percent_prefix", "percent_change"))

      compared =
        Enum.reduce(fields, %{}, fn field, acc ->
          with {:ok, left_value} <- fetch_numeric_field(left, field),
               {:ok, right_value} <- fetch_numeric_field(right, field) do
            acc
            |> maybe_put(
              Map.get(config, "include_originals", true),
              "#{left_prefix}_#{field}",
              left_value
            )
            |> maybe_put(
              Map.get(config, "include_originals", true),
              "#{right_prefix}_#{field}",
              right_value
            )
            |> maybe_put(
              Map.get(config, "include_delta", true),
              "#{delta_prefix}_#{field}",
              right_value - left_value
            )
            |> maybe_put(
              Map.get(config, "include_ratio", true) and left_value != 0.0,
              "#{ratio_prefix}_#{field}",
              right_value / left_value
            )
            |> maybe_put(
              Map.get(config, "include_percent_change", true) and left_value != 0.0,
              "#{percent_prefix}_#{field}",
              (right_value - left_value) / left_value * 100.0
            )
          else
            _ -> acc
          end
        end)

      if map_size(compared) == 0 do
        {:error, :empty_comparison}
      else
        {:ok,
         compared
         |> maybe_put(
           Map.get(config, "include_shared_field_count", true),
           "summary_shared_numeric_field_count",
           count_shared_numeric_fields(fields, left, right)
         )
         |> Map.put("summary_left_prefix", left_prefix)
         |> Map.put("summary_right_prefix", right_prefix)}
      end
    end
  end

  def compare_summary_pair(_payload, _config), do: {:error, :invalid_summary_pair}

  def aggregate_summary_collection(payload, config) when is_map(payload) and is_map(config) do
    source_entries = Enum.filter(payload, fn {_source_id, summary} -> is_map(summary) end)

    if source_entries == [] do
      {:error, :invalid_summary_collection}
    else
      include_values = Map.get(config, "include_values", false)
      include_sources = Map.get(config, "include_sources", true)
      output_prefix = normalize_prefix(Map.get(config, "output_prefix", "aggregate"))
      fields = aggregate_fields(source_entries, config)

      aggregated =
        Enum.reduce(fields, %{}, fn field, acc ->
          field_values =
            Enum.flat_map(source_entries, fn {source_id, summary} ->
              case fetch_numeric_field(summary, field) do
                {:ok, value} -> [{source_id, value}]
                :error -> []
              end
            end)

          if field_values == [] do
            acc
          else
            values = Enum.map(field_values, &elem(&1, 1))
            prefix = "#{output_prefix}_#{field}"

            acc
            |> Map.put("#{prefix}_count", length(values))
            |> Map.put("#{prefix}_min", Enum.min(values))
            |> Map.put("#{prefix}_max", Enum.max(values))
            |> Map.put("#{prefix}_mean", Enum.sum(values) / length(values))
            |> Map.put("#{prefix}_span", Enum.max(values) - Enum.min(values))
            |> maybe_put(include_values, "#{prefix}_values", values)
            |> maybe_put(
              include_sources,
              "#{prefix}_sources",
              Enum.map(field_values, &elem(&1, 0))
            )
          end
        end)

      if map_size(aggregated) == 0 do
        {:error, :empty_aggregate}
      else
        width = 5 + if(include_values, do: 1, else: 0) + if(include_sources, do: 1, else: 0)

        {:ok,
         aggregated
         |> Map.put("summary_input_count", length(source_entries))
         |> Map.put("summary_aggregated_field_count", div(map_size(aggregated), width))}
      end
    end
  end

  def normalize_summary_fields(payload, config) when is_map(payload) and is_map(config) do
    case Map.get(config, "rules") do
      rules when is_list(rules) and rules != [] ->
        copy_unmapped = Map.get(config, "copy_unmapped", false)

        normalized =
          Enum.reduce(rules, if(copy_unmapped, do: payload, else: %{}), fn rule, acc ->
            source = Map.get(rule, "source")
            target = Map.get(rule, "target", source)

            case {source, target, Map.fetch(payload, source)} do
              {source, target, {:ok, value}} when is_binary(source) and is_binary(target) ->
                next_value =
                  if is_number(value) do
                    value
                    |> Kernel.*(Map.get(rule, "scale", 1.0))
                    |> Kernel.+(Map.get(rule, "offset", 0.0))
                    |> clamp_numeric(Map.get(rule, "clamp_min"), Map.get(rule, "clamp_max"))
                  else
                    value
                  end

                acc
                |> Map.put(target, next_value)
                |> maybe_put(not copy_unmapped and target != source, "source_#{target}", source)

              _ ->
                acc
            end
          end)

        if map_size(normalized) == 0,
          do: {:error, :empty_normalized_summary},
          else: {:ok, normalized}

      _ ->
        {:error, :invalid_normalization_rules}
    end
  end

  def select_best_summary(payload, config) when is_map(payload) and is_map(config) do
    source_entries = Enum.filter(payload, fn {_source_id, summary} -> is_map(summary) end)

    with criteria when is_list(criteria) and criteria != [] <- Map.get(config, "criteria"),
         false <- source_entries == [],
         {:ok, scored} <- score_summary_entries(source_entries, criteria) do
      [{best_source, best_summary, best_score, best_breakdown} | _] =
        Enum.sort_by(scored, fn {source_id, _summary, score, _breakdown} ->
          {-score, source_id}
        end)

      {:ok,
       best_summary
       |> Map.put("selected_summary_source", best_source)
       |> Map.put("selected_summary_score", best_score)
       |> maybe_put(
         Map.get(config, "include_breakdown", true),
         "selected_summary_breakdown",
         best_breakdown
       )
       |> maybe_put(
         Map.get(config, "include_all_scores", true),
         "selected_summary_candidates",
         Enum.map(scored, fn {source_id, _summary, score, _breakdown} ->
           %{"source" => source_id, "score" => score}
         end)
       )}
    else
      true -> {:error, :invalid_summary_collection}
      _ -> {:error, :invalid_selection_criteria}
    end
  end

  def compose_diagnostics_bundle(payload, config) when is_map(payload) and is_map(config) do
    diagnostics =
      payload
      |> Enum.filter(fn {_source_id, entry} -> is_map(entry) end)
      |> Enum.filter(fn {_source_id, entry} -> diagnostic_entry?(entry, config) end)

    if diagnostics == [] do
      {:error, :invalid_diagnostics_bundle}
    else
      include_payloads = Map.get(config, "include_payloads", true)
      include_numeric_fields = Map.get(config, "include_numeric_fields", true)

      items =
        Enum.map(diagnostics, fn {source_id, entry} ->
          %{
            "source" => source_id,
            "domain" => Map.get(entry, "diagnostic_domain"),
            "subject" => Map.get(entry, "diagnostic_subject"),
            "prefix" => Map.get(entry, "diagnostic_prefix"),
            "node_count" => Map.get(entry, "diagnostic_node_count", 0),
            "element_count" => Map.get(entry, "diagnostic_element_count", 0),
            "metric_groups" => Map.get(entry, "diagnostic_metric_groups", [])
          }
        end)

      numeric_fields =
        diagnostics
        |> Enum.flat_map(fn {_source_id, entry} ->
          entry
          |> Enum.filter(fn {_field, value} -> is_number(value) end)
          |> Enum.map(&elem(&1, 0))
        end)
        |> Enum.uniq()
        |> Enum.sort()

      domains =
        items
        |> Enum.map(& &1["domain"])
        |> Enum.reject(&is_nil/1)
        |> Enum.uniq()
        |> Enum.sort()

      metric_groups =
        items
        |> Enum.flat_map(&List.wrap(&1["metric_groups"]))
        |> Enum.uniq()
        |> Enum.sort()

      subjects =
        items
        |> Enum.map(& &1["subject"])
        |> Enum.reject(&is_nil/1)
        |> Enum.uniq()
        |> Enum.sort()

      payloads =
        Map.new(diagnostics, fn {source_id, entry} ->
          {source_id, entry}
        end)

      domain_counts =
        Enum.reduce(items, %{}, fn item, acc ->
          Map.update(acc, item["domain"], 1, &(&1 + 1))
        end)

      {:ok,
       %{
         "bundle_contract" => "kyuubiki.workflow_diagnostics_bundle/v1",
         "bundle_kind" => "workflow_diagnostics_bundle",
         "bundle_source_count" => length(items),
         "bundle_sources" => Enum.map(items, & &1["source"]),
         "bundle_domains" => domains,
         "bundle_subjects" => subjects,
         "bundle_domain_counts" => domain_counts,
         "bundle_metric_groups" => metric_groups,
         "bundle_items" => items,
         "bundle_total_node_count" => Enum.sum(Enum.map(items, & &1["node_count"])),
         "bundle_total_element_count" => Enum.sum(Enum.map(items, & &1["element_count"])),
         "bundle_numeric_field_count" => length(numeric_fields)
       }
       |> maybe_put(include_payloads, "bundle_payloads", payloads)
       |> maybe_put(include_numeric_fields, "bundle_numeric_fields", numeric_fields)}
    end
  end

  def evaluate_diagnostics_bundle_guard(payload, config)
      when is_map(payload) and is_map(config) do
    rules =
      case Map.get(config, "rules") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if rules == [] do
      {:error, :invalid_diagnostics_bundle_guard_rules}
    else
      triggers =
        Enum.flat_map(rules, fn rule ->
          evaluate_bundle_guard_rule(payload, rule)
        end)

      status =
        cond do
          Enum.any?(triggers, &(&1["severity"] == "block")) -> "block"
          Enum.any?(triggers, &(&1["severity"] == "warn")) -> "warn"
          true -> "pass"
        end

      {:ok,
       %{
         "guard_contract" => "kyuubiki.workflow_guard_result/v1",
         "guard_scope" => "workflow_diagnostics_bundle",
         "guard_status" => status,
         "guard_passed" => status == "pass",
         "guard_trigger_count" => length(triggers),
         "guard_checked_rule_count" => length(rules),
         "guard_warn_count" => Enum.count(triggers, &(&1["severity"] == "warn")),
         "guard_block_count" => Enum.count(triggers, &(&1["severity"] == "block")),
         "guard_triggers" => triggers,
         "guard_recommendation" => bundle_guard_recommendation(status),
         "guard_summary" => bundle_guard_summary(status, triggers)
       }}
    end
  end

  defp aggregate_fields(source_entries, config) do
    case Map.get(config, "fields") do
      fields when is_list(fields) and fields != [] ->
        Enum.filter(fields, &is_binary/1)

      _ ->
        source_entries
        |> Enum.flat_map(fn {_source_id, summary} ->
          summary
          |> Enum.filter(fn {_field, value} -> is_number(value) end)
          |> Enum.map(&elem(&1, 0))
        end)
        |> Enum.uniq()
        |> Enum.sort()
    end
  end

  defp resolve_summary_compare_fields(left, right, config) do
    case Map.get(config, "fields") do
      fields when is_list(fields) ->
        requested = Enum.filter(fields, &is_binary/1)
        if requested == [], do: {:error, :invalid_compare_fields}, else: {:ok, requested}

      _ ->
        {:ok,
         left
         |> Enum.filter(fn {field, value} ->
           is_number(value) and is_number(Map.get(right, field))
         end)
         |> Enum.map(&elem(&1, 0))
         |> Enum.sort()}
    end
  end

  defp count_shared_numeric_fields(fields, left, right) do
    Enum.count(fields, fn field ->
      match?({:ok, _}, fetch_numeric_field(left, field)) and
        match?({:ok, _}, fetch_numeric_field(right, field))
    end)
  end

  defp score_summary_entries(source_entries, criteria) do
    Enum.reduce_while(source_entries, {:ok, []}, fn {source_id, summary}, {:ok, acc} ->
      case score_summary_entry(summary, criteria) do
        {:ok, {score, breakdown}} ->
          {:cont, {:ok, [{source_id, summary, score, breakdown} | acc]}}

        {:error, reason} ->
          {:halt, {:error, reason}}
      end
    end)
  end

  defp score_summary_entry(summary, criteria) do
    Enum.reduce_while(criteria, {:ok, {0.0, []}}, fn criterion, {:ok, {total, breakdown}} ->
      field = Map.get(criterion, "field")
      goal = Map.get(criterion, "goal", "max")
      weight = Map.get(criterion, "weight", 1.0)

      case {field, fetch_numeric_field(summary, field)} do
        {field, {:ok, value}} when is_binary(field) and goal in ["min", "max"] ->
          score = if goal == "min", do: -value * weight, else: value * weight

          {:cont,
           {:ok,
            {total + score,
             breakdown ++
               [
                 %{
                   "field" => field,
                   "goal" => goal,
                   "weight" => weight,
                   "value" => value,
                   "score" => score
                 }
               ]}}}

        _ ->
          {:halt, {:error, :invalid_selection_criterion}}
      end
    end)
  end

  defp evaluate_bundle_guard_rule(payload, rule) do
    with field when is_binary(field) <- Map.get(rule, "field"),
         {:ok, value, source_ref} <- fetch_bundle_guard_value(payload, rule, field),
         true <- bundle_guard_triggered?(value, rule) do
      [
        %{
          "field" => field,
          "source" => source_ref,
          "value" => value,
          "threshold" => bundle_guard_threshold(rule),
          "comparison" => bundle_guard_comparison(rule),
          "severity" => bundle_guard_severity(Map.get(rule, "severity", "warn")),
          "label" => Map.get(rule, "label", field)
        }
      ]
    else
      _ -> []
    end
  end

  defp fetch_bundle_guard_value(payload, rule, field) do
    case Map.get(rule, "source") do
      source when is_binary(source) ->
        with %{} = bundle_payloads <- Map.get(payload, "bundle_payloads"),
             %{} = source_payload <- Map.get(bundle_payloads, source),
             {:ok, value} <- fetch_numeric_field(source_payload, field) do
          {:ok, value, source}
        else
          _ -> :error
        end

      _ ->
        case fetch_numeric_field(payload, field) do
          {:ok, value} -> {:ok, value, "bundle"}
          :error -> :error
        end
    end
  end

  defp bundle_guard_triggered?(value, rule) do
    comparison = bundle_guard_comparison(rule)
    threshold = bundle_guard_threshold(rule)

    case {comparison, threshold} do
      {"gt", threshold} when is_number(threshold) -> value > threshold
      {"gte", threshold} when is_number(threshold) -> value >= threshold
      {"lt", threshold} when is_number(threshold) -> value < threshold
      {"lte", threshold} when is_number(threshold) -> value <= threshold
      {"eq", threshold} when is_number(threshold) -> value == threshold
      _ -> false
    end
  end

  defp bundle_guard_comparison(rule) do
    case Map.get(rule, "comparison", "gte") do
      operator when operator in ["gt", "gte", "lt", "lte", "eq"] -> operator
      _ -> "gte"
    end
  end

  defp bundle_guard_threshold(rule) do
    cond do
      is_number(Map.get(rule, "threshold")) -> Map.get(rule, "threshold") * 1.0
      is_number(Map.get(rule, "value")) -> Map.get(rule, "value") * 1.0
      true -> nil
    end
  end

  defp bundle_guard_severity("block"), do: "block"
  defp bundle_guard_severity("warn"), do: "warn"
  defp bundle_guard_severity(_severity), do: "warn"

  defp bundle_guard_recommendation("block"), do: "hold_and_review"
  defp bundle_guard_recommendation("warn"), do: "review_before_continue"
  defp bundle_guard_recommendation(_status), do: "continue"

  defp bundle_guard_summary("pass", _triggers), do: "All diagnostics bundle guard rules passed."

  defp bundle_guard_summary(status, triggers) do
    lead =
      triggers
      |> Enum.take(2)
      |> Enum.map_join(", ", fn trigger ->
        "#{trigger["source"]}.#{trigger["label"]}=#{trigger["value"]}"
      end)

    "#{String.upcase(status)}: #{length(triggers)} trigger(s)" <>
      if(lead == "", do: ".", else: " (#{lead}).")
  end

  defp diagnostic_entry?(entry, config) do
    include_non_diagnostics = Map.get(config, "include_non_diagnostics", false)

    include_non_diagnostics or
      Map.get(entry, "diagnostic_contract") == "kyuubiki.workflow_diagnostics/v1"
  end

  defp fetch_numeric_field(map, field) when is_map(map) and is_binary(field) do
    case Map.get(map, field) do
      value when is_number(value) -> {:ok, value * 1.0}
      _ -> :error
    end
  end

  defp namespaced_summary_fields(source, prefix),
    do: Map.new(source, fn {key, value} -> {"#{prefix}_#{key}", value} end)

  defp clamp_numeric(value, clamp_min, clamp_max) do
    value
    |> then(fn next -> if is_number(clamp_min), do: max(next, clamp_min), else: next end)
    |> then(fn next -> if is_number(clamp_max), do: min(next, clamp_max), else: next end)
  end

  defp normalize_prefix(prefix) when is_binary(prefix) do
    trimmed = String.trim(prefix)
    if trimmed == "", do: "summary", else: trimmed
  end

  defp maybe_put(map, true, key, value), do: Map.put(map, key, value)
  defp maybe_put(map, _condition, _key, _value), do: map
end
