defmodule KyuubikiWeb.WorkflowSummaryRuntime do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def merge_summary_pair(%{"left" => left, "right" => right} = payload, config)
      when is_map(left) and is_map(right) and is_map(config) do
    with {:ok, _} <-
           admit(payload, "transform.merge_summary_pair", [{"left", left}, {"right", right}]) do
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
  end

  def merge_summary_pair(_payload, _config), do: {:error, :invalid_summary_pair}

  def compare_summary_pair(%{"left" => left, "right" => right} = payload, config)
      when is_map(left) and is_map(right) and is_map(config) do
    with {:ok, _} <-
           admit(payload, "transform.compare_summary_pair", [{"left", left}, {"right", right}]),
         {:ok, fields} <- resolve_summary_compare_fields(left, right, config) do
      left_prefix = normalize_prefix(Map.get(config, "left_prefix", "left"))
      right_prefix = normalize_prefix(Map.get(config, "right_prefix", "right"))
      delta_prefix = normalize_prefix(Map.get(config, "delta_prefix", "delta"))
      ratio_prefix = normalize_prefix(Map.get(config, "ratio_prefix", "ratio"))
      percent_prefix = normalize_prefix(Map.get(config, "percent_prefix", "percent_change"))

      Check.run("transform.compare_summary_pair", fn ->
        {compared, count} =
          Enum.reduce(fields, {%{}, 0}, fn field, {acc, count} ->
            case {Map.get(left, field), Map.get(right, field)} do
              {left_value, right_value} when is_number(left_value) and is_number(right_value) ->
                left_value = Check.number(left_value, "payload.left.#{field}")
                right_value = Check.number(right_value, "payload.right.#{field}")

                next =
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
                  |> put_comparison(
                    Map.get(config, "include_delta", true),
                    "#{delta_prefix}_#{field}",
                    fn -> right_value - left_value end
                  )
                  |> put_comparison(
                    Map.get(config, "include_ratio", true) and left_value != 0.0,
                    "#{ratio_prefix}_#{field}",
                    fn -> right_value / left_value end
                  )
                  |> put_comparison(
                    Map.get(config, "include_percent_change", true) and left_value != 0.0,
                    "#{percent_prefix}_#{field}",
                    fn -> comparison_percent_change(left_value, right_value) end
                  )

                {next, count + 1}

              _ ->
                {acc, count}
            end
          end)

        if map_size(compared) == 0,
          do: Check.fail("did not find any shared numeric summary fields")

        compared
        |> maybe_put(
          Map.get(config, "include_shared_field_count", true),
          "summary_shared_numeric_field_count",
          count
        )
        |> Map.put("summary_left_prefix", left_prefix)
        |> Map.put("summary_right_prefix", right_prefix)
      end)
    end
  end

  def compare_summary_pair(_payload, _config), do: {:error, :invalid_summary_pair}

  def aggregate_summary_collection(payload, config) when is_map(payload) and is_map(config) do
    source_entries = summary_sources(payload)

    with {:ok, _} <- admit(payload, "transform.aggregate_summary_collection", source_entries) do
      Check.run("transform.aggregate_summary_collection", fn ->
        if source_entries == [], do: Check.fail("expects named object summaries")
        include_values = Map.get(config, "include_values", false)
        include_sources = Map.get(config, "include_sources", true)
        output_prefix = normalize_prefix(Map.get(config, "output_prefix", "aggregate"))
        fields = aggregate_fields(source_entries, config)

        aggregated =
          Enum.reduce(fields, %{}, fn field, acc ->
            field_values =
              Enum.flat_map(source_entries, fn {source_id, summary} ->
                case Map.get(summary, field) do
                  value when is_number(value) ->
                    [{source_id, Check.number(value, "payload.#{source_id}.#{field}")}]

                  _ ->
                    []
                end
              end)

            if field_values == [] do
              acc
            else
              values = Enum.map(field_values, &elem(&1, 1))
              prefix = "#{output_prefix}_#{field}"
              minimum = Enum.min(values)
              maximum = Enum.max(values)
              mean = Check.calculate("#{prefix}_mean", fn -> summary_mean(values) end)
              span = Check.calculate("#{prefix}_span", fn -> maximum - minimum end)

              acc
              |> Map.put("#{prefix}_count", length(values))
              |> Map.put("#{prefix}_min", minimum)
              |> Map.put("#{prefix}_max", maximum)
              |> Map.put("#{prefix}_mean", mean)
              |> Map.put("#{prefix}_span", span)
              |> maybe_put(include_values, "#{prefix}_values", values)
              |> maybe_put(
                include_sources,
                "#{prefix}_sources",
                Enum.map(field_values, &elem(&1, 0))
              )
            end
          end)

        if map_size(aggregated) == 0, do: Check.fail("did not find any numeric summary fields")
        width = 5 + if(include_values, do: 1, else: 0) + if(include_sources, do: 1, else: 0)

        aggregated
        |> Map.put("summary_input_count", length(source_entries))
        |> Map.put("summary_aggregated_field_count", div(map_size(aggregated), width))
      end)
    end
  end

  def normalize_summary_fields(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, _} <- admit(payload, "transform.normalize_summary_fields", []) do
      Check.run("transform.normalize_summary_fields", fn ->
        rules = Check.list(Map.get(config, "rules"), "config.rules")
        if rules == [], do: Check.fail("config.rules must not be empty")
        copy_unmapped = Check.boolean(config, "copy_unmapped", false)

        normalized =
          rules
          |> Enum.with_index()
          |> Enum.reduce(if(copy_unmapped, do: payload, else: %{}), fn {rule, index}, acc ->
            path = "config.rules[#{index}]"
            rule = Check.object(rule, path)
            source = Check.text(rule, "source", path) || Check.fail("#{path}.source is required")
            target = Check.text(rule, "target", path, source)
            scale = Check.optional_number(rule, "scale", path) || 1.0
            offset = Check.optional_number(rule, "offset", path) || 0.0
            clamp_min = Check.optional_number(rule, "clamp_min", path)
            clamp_max = Check.optional_number(rule, "clamp_max", path)

            if is_number(clamp_min) and is_number(clamp_max) and clamp_min > clamp_max,
              do: Check.fail("#{path}.clamp_max must be at least clamp_min")

            case Map.fetch(payload, source) do
              {:ok, value} ->
                next_value =
                  if is_number(value) do
                    number = Check.number(value, "payload.#{source}")

                    Check.calculate("normalized.#{target}", fn -> number * scale + offset end)
                    |> clamp_numeric(clamp_min, clamp_max)
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

        if map_size(normalized) == 0, do: Check.fail("did not emit any normalized fields")
        normalized
      end)
    end
  end

  def select_best_summary(payload, config) when is_map(payload) and is_map(config) do
    source_entries = summary_sources(payload)

    with {:ok, _} <- admit(payload, "transform.select_best_summary", source_entries),
         criteria when is_list(criteria) and criteria != [] <- Map.get(config, "criteria"),
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
      {:error, reason} when is_binary(reason) -> {:error, reason}
      true -> {:error, :invalid_summary_collection}
      _ -> {:error, :invalid_selection_criteria}
    end
  end

  defdelegate compose_diagnostics_bundle(payload, config),
    to: KyuubikiWeb.WorkflowDiagnosticBundleRuntime,
    as: :compose

  defdelegate evaluate_diagnostics_bundle_guard(payload, config),
    to: KyuubikiWeb.WorkflowDiagnosticBundleRuntime,
    as: :guard

  defp admit(payload, operator, sources) do
    Check.run(operator, fn ->
      Check.converged(payload, "payload")
      Enum.each(sources, fn {id, summary} -> Check.converged(summary, "payload.#{id}") end)
    end)
  end

  defp summary_sources(payload) do
    Enum.filter(payload, fn {id, summary} ->
      id not in ["converged", "stability_result"] and is_map(summary)
    end)
    |> Enum.sort_by(&elem(&1, 0))
  end

  defp summary_mean(values) do
    compensated_total(values) / length(values)
  rescue
    ArithmeticError ->
      # Scale only when summation overflows; ordinary/subnormal tails keep their original units.
      scale = values |> Enum.map(&abs/1) |> Enum.max()
      compensated_total(Enum.map(values, &(&1 / scale))) / length(values) * scale
  end

  defp compensated_total(values) do
    {sum, correction} =
      Enum.reduce(values, {0.0, 0.0}, fn value, {sum, correction} ->
        next = sum + value
        tail = if abs(sum) >= abs(value), do: sum - next + value, else: value - next + sum
        {next, correction + tail}
      end)

    sum + correction
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

  defp put_comparison(map, true, key, work),
    do: Map.put(map, key, Check.calculate(key, work))

  defp put_comparison(map, _enabled, _key, _work), do: map

  defp comparison_percent_change(left, right) do
    difference =
      try do
        {:ok, right - left}
      rescue
        ArithmeticError -> :overflow
      end

    # Preserve nearby-value subtraction; divide first only if the difference overflows.
    case difference do
      {:ok, delta} -> delta / left * 100.0
      :overflow -> (right / left - 1.0) * 100.0
    end
  end

  defp score_summary_entries(source_entries, criteria) do
    Check.run("transform.select_best_summary", fn ->
      Enum.map(source_entries, fn {source_id, summary} ->
        {score, breakdown} = score_summary_entry(summary, criteria, source_id)
        {source_id, summary, score, breakdown}
      end)
    end)
  end

  defp score_summary_entry(summary, criteria, source_id) do
    {score, breakdown} =
      criteria
      |> Enum.with_index()
      |> Enum.reduce({0.0, []}, fn {criterion, index}, {total, breakdown} ->
        path = "config.criteria[#{index}]"
        criterion = Check.object(criterion, path)
        field = Check.text(criterion, "field", path) || Check.fail("#{path}.field is required")
        goal = Check.choice(criterion, "goal", "max", ["min", "max"], path)
        weight = Check.optional_number(criterion, "weight", path) || 1.0
        value = Check.number(Map.get(summary, field), "payload.#{source_id}.#{field}")

        score =
          Check.calculate("payload.#{source_id}.score[#{index}]", fn ->
            if goal == "min", do: -value * weight, else: value * weight
          end)

        next = Check.calculate("payload.#{source_id}.total[#{index}]", fn -> total + score end)

        entry = %{
          "field" => field,
          "goal" => goal,
          "weight" => weight,
          "value" => value,
          "score" => score
        }

        {next, [entry | breakdown]}
      end)

    {score, Enum.reverse(breakdown)}
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
