defmodule KyuubikiWeb.WorkflowMagnetostaticRuntime do
  @moduledoc false

  def extract_magnetostatic_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "magnetostatic"))

      vector_potential_nodes =
        collect_numeric_entries(
          nodes,
          Map.get(config, "vector_potential_field", "vector_potential")
        )

      current_density_nodes =
        collect_numeric_entries(
          nodes,
          Map.get(config, "current_density_field", "current_density")
        )

      energy_density_peak =
        peak_scalar_entry(
          elements,
          Map.get(config, "energy_density_field", "energy_area_density")
        ) ||
          peak_scalar_entry(elements, "stored_energy")

      field_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "field_x_field", "magnetic_field_strength_x"),
          Map.get(config, "field_y_field", "magnetic_field_strength_y"),
          Map.get(config, "field_z_field"),
          Map.get(config, "field_magnitude_field", "magnetic_field_strength_magnitude")
        )

      flux_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "flux_x_field", "magnetic_flux_density_x"),
          Map.get(config, "flux_y_field", "magnetic_flux_density_y"),
          Map.get(config, "flux_z_field"),
          Map.get(config, "flux_magnitude_field", "magnetic_flux_density_magnitude")
        )

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> merge_diagnostic_contract("magnetostatic", "magnetostatic_result", prefix, [
          "vector_potential",
          "current_density",
          "energy_density",
          "field",
          "flux"
        ])
        |> maybe_merge_distribution_stats(prefix, "vector_potential", vector_potential_nodes)
        |> maybe_merge_distribution_stats(prefix, "current_density", current_density_nodes)
        |> maybe_merge_peak_scalar(prefix, "energy_density", energy_density_peak)
        |> maybe_merge_peak_vector(prefix, "field", field_peak)
        |> maybe_merge_peak_vector(prefix, "flux", flux_peak)

      if map_size(diagnostics) <= 6 do
        {:error, :empty_magnetostatic_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_magnetostatic_result}
    end
  end

  def extract_magnetostatic_result_diagnostics(_payload, _config),
    do: {:error, :invalid_magnetostatic_result}

  def evaluate_magnetostatic_guard(payload, config) when is_map(payload) and is_map(config) do
    rules =
      case Map.get(config, "rules") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if rules == [] do
      {:error, :invalid_magnetostatic_guard_rules}
    else
      triggers = Enum.flat_map(rules, &evaluate_guard_rule(payload, &1))
      status = guard_status(triggers)

      {:ok,
       %{
         "guard_status" => status,
         "guard_passed" => status == "pass",
         "guard_trigger_count" => length(triggers),
         "guard_checked_rule_count" => length(rules),
         "guard_warn_count" => Enum.count(triggers, &(&1["severity"] == "warn")),
         "guard_block_count" => Enum.count(triggers, &(&1["severity"] == "block")),
         "guard_triggers" => triggers,
         "guard_recommendation" => guard_recommendation(status),
         "guard_summary" => guard_summary(status, triggers)
       }}
    end
  end

  def benchmark_magnetostatic_pair(%{"left" => left, "right" => right}, config)
      when is_map(left) and is_map(right) and is_map(config) do
    criteria =
      case Map.get(config, "criteria") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if criteria == [] do
      {:error, :invalid_magnetostatic_benchmark_criteria}
    else
      left_label = normalize_prefix(Map.get(config, "left_label", "left"))
      right_label = normalize_prefix(Map.get(config, "right_label", "right"))

      breakdown =
        Enum.flat_map(criteria, &benchmark_criterion(left, right, &1, left_label, right_label))

      if breakdown == [] do
        {:error, :empty_magnetostatic_benchmark}
      else
        emit_benchmark_summary(breakdown, left_label, right_label)
      end
    end
  end

  def benchmark_magnetostatic_pair(_payload, _config),
    do: {:error, :invalid_magnetostatic_benchmark_pair}

  defp fetch_list(payload, key) when is_binary(key) do
    case Map.get(payload, key) do
      entries when is_list(entries) -> {:ok, entries}
      _ -> {:error, :invalid_result_collection}
    end
  end

  defp fetch_list(_payload, _key), do: {:error, :invalid_result_collection}

  defp collect_numeric_entries(entries, field) when is_list(entries) and is_binary(field) do
    entries
    |> Enum.filter(&is_map/1)
    |> Enum.flat_map(fn entry ->
      case fetch_numeric_field(entry, field) do
        {:ok, value} -> [{entry, value}]
        :error -> []
      end
    end)
  end

  defp collect_numeric_entries(_entries, _field), do: []

  defp peak_scalar_entry(entries, field) when is_list(entries) and is_binary(field) do
    entries
    |> collect_numeric_entries(field)
    |> Enum.max_by(&elem(&1, 1), fn -> nil end)
  end

  defp peak_scalar_entry(_entries, _field), do: nil

  defp peak_vector_entry(entries, x_field, y_field, z_field, magnitude_field) do
    entries
    |> Enum.filter(&is_map/1)
    |> Enum.flat_map(fn entry ->
      components =
        %{}
        |> maybe_put_component("x", entry, x_field)
        |> maybe_put_component("y", entry, y_field)
        |> maybe_put_component("z", entry, z_field)

      magnitude =
        case fetch_numeric_field(entry, magnitude_field) do
          {:ok, value} -> value
          :error -> vector_magnitude(components)
        end

      if magnitude == nil,
        do: [],
        else: [%{"entry" => entry, "magnitude" => magnitude, "components" => components}]
    end)
    |> Enum.max_by(& &1["magnitude"], fn -> nil end)
  end

  defp maybe_put_component(components, _axis, _entry, field) when not is_binary(field),
    do: components

  defp maybe_put_component(components, axis, entry, field) do
    case fetch_numeric_field(entry, field) do
      {:ok, value} -> Map.put(components, axis, value)
      :error -> components
    end
  end

  defp vector_magnitude(components) when map_size(components) >= 2 do
    components
    |> Map.values()
    |> Enum.reduce(0.0, &(&1 * &1 + &2))
    |> :math.sqrt()
  end

  defp vector_magnitude(_components), do: nil

  defp maybe_merge_distribution_stats(diagnostics, _prefix, _label, []), do: diagnostics

  defp maybe_merge_distribution_stats(diagnostics, prefix, label, entries) do
    values = Enum.map(entries, &elem(&1, 1))
    sum = Enum.sum(values)

    diagnostics
    |> Map.put("#{prefix}_#{label}_count", length(values))
    |> Map.put("#{prefix}_#{label}_min", Enum.min(values))
    |> Map.put("#{prefix}_#{label}_max", Enum.max(values))
    |> Map.put("#{prefix}_#{label}_mean", sum / length(values))
    |> Map.put("#{prefix}_#{label}_sum", sum)
    |> Map.put("#{prefix}_#{label}_span", Enum.max(values) - Enum.min(values))
  end

  defp maybe_merge_peak_scalar(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_scalar(diagnostics, prefix, label, {entry, value}) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak", value)
    |> Map.put("#{prefix}_peak_#{label}", value)
    |> Map.put("#{prefix}_peak_#{label}_id", Map.get(entry, "id"))
    |> Map.put("#{prefix}_#{label}_peak_element_id", Map.get(entry, "id"))
  end

  defp maybe_merge_peak_vector(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_vector(diagnostics, prefix, label, peak) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak_magnitude", peak["magnitude"])
    |> Map.put("#{prefix}_peak_#{label}", peak["magnitude"])
    |> Map.put("#{prefix}_peak_#{label}_id", get_in(peak, ["entry", "id"]))
    |> Map.put("#{prefix}_#{label}_peak_element_id", get_in(peak, ["entry", "id"]))
    |> maybe_put_vector_components(prefix, label, peak["components"])
  end

  defp maybe_put_vector_components(diagnostics, prefix, label, components) do
    Enum.reduce(components, diagnostics, fn {axis, value}, acc ->
      Map.put(acc, "#{prefix}_#{label}_peak_#{axis}", value)
    end)
  end

  defp merge_diagnostic_contract(diagnostics, domain, subject, prefix, metric_groups) do
    diagnostics
    |> Map.put("diagnostic_contract", "kyuubiki.workflow_diagnostics/v1")
    |> Map.put("diagnostic_domain", domain)
    |> Map.put("diagnostic_subject", subject)
    |> Map.put("diagnostic_prefix", prefix)
    |> Map.put("diagnostic_node_count", Map.get(diagnostics, "#{prefix}_node_count", 0))
    |> Map.put("diagnostic_element_count", Map.get(diagnostics, "#{prefix}_element_count", 0))
    |> Map.put("diagnostic_metric_groups", metric_groups)
  end

  defp fetch_numeric_field(entry, field) when is_map(entry) and is_binary(field) do
    case Map.get(entry, field) do
      value when is_number(value) -> {:ok, value * 1.0}
      _ -> :error
    end
  end

  defp fetch_numeric_field(_entry, _field), do: :error

  defp evaluate_guard_rule(payload, rule) do
    with field when is_binary(field) <- Map.get(rule, "field"),
         {:ok, value} <- fetch_numeric_field(payload, field),
         true <- guard_triggered?(value, rule) do
      [
        %{
          "field" => field,
          "value" => value,
          "threshold" => rule_threshold(rule),
          "comparison" => rule_comparison(rule),
          "severity" => normalize_severity(Map.get(rule, "severity", "warn")),
          "label" => Map.get(rule, "label", field)
        }
      ]
    else
      _ -> []
    end
  end

  defp benchmark_criterion(left, right, criterion, left_label, right_label) do
    with field when is_binary(field) <- Map.get(criterion, "field"),
         left_field when is_binary(left_field) <- Map.get(criterion, "left_field", field),
         right_field when is_binary(right_field) <- Map.get(criterion, "right_field", field),
         {:ok, left_value} <- fetch_numeric_field(left, left_field),
         {:ok, right_value} <- fetch_numeric_field(right, right_field) do
      goal = benchmark_goal(criterion)
      weight = benchmark_weight(criterion)
      {left_score, right_score, winner} = benchmark_scores(left_value, right_value, goal, weight)

      [
        %{
          "field" => field,
          "goal" => goal,
          "weight" => weight,
          "#{left_label}_field" => left_field,
          "#{left_label}_value" => left_value,
          "#{right_label}_field" => right_field,
          "#{right_label}_value" => right_value,
          "left_score" => left_score,
          "right_score" => right_score,
          "winner" => winner
        }
      ]
    else
      _ -> []
    end
  end

  defp guard_triggered?(value, rule) do
    threshold = rule_threshold(rule)
    threshold != nil and compare_guard_values(value, threshold, rule_comparison(rule))
  end

  defp compare_guard_values(value, threshold, "gt"), do: value > threshold
  defp compare_guard_values(value, threshold, "gte"), do: value >= threshold
  defp compare_guard_values(value, threshold, "lt"), do: value < threshold
  defp compare_guard_values(value, threshold, "lte"), do: value <= threshold
  defp compare_guard_values(value, threshold, _comparison), do: value > threshold

  defp rule_threshold(rule), do: number_or_nil(Map.get(rule, "threshold"))

  defp rule_comparison(%{"comparison" => value}) when value in ["gt", "gte", "lt", "lte"],
    do: value

  defp rule_comparison(_rule), do: "gt"
  defp normalize_severity("block"), do: "block"
  defp normalize_severity(_severity), do: "warn"

  defp guard_status(triggers) do
    cond do
      Enum.any?(triggers, &(&1["severity"] == "block")) -> "block"
      Enum.any?(triggers, &(&1["severity"] == "warn")) -> "warn"
      true -> "pass"
    end
  end

  defp guard_recommendation("block"), do: "hold_and_review"
  defp guard_recommendation("warn"), do: "review_before_coupling"
  defp guard_recommendation(_status), do: "ready_for_next_stage"

  defp guard_summary("pass", _triggers),
    do: "PASS: no magnetostatic guard triggers exceeded configured thresholds."

  defp guard_summary(status, triggers),
    do:
      "#{String.upcase(status)}: #{length(triggers)} magnetostatic guard trigger(s) exceeded configured thresholds."

  defp benchmark_goal(%{"goal" => "max"}), do: "max"
  defp benchmark_goal(_criterion), do: "min"
  defp benchmark_weight(%{"weight" => value}) when is_number(value) and value > 0, do: value * 1.0
  defp benchmark_weight(_criterion), do: 1.0

  defp benchmark_scores(left_value, right_value, "max", weight) do
    cond do
      left_value > right_value -> {weight, 0.0, "left"}
      right_value > left_value -> {0.0, weight, "right"}
      true -> {0.0, 0.0, "tie"}
    end
  end

  defp benchmark_scores(left_value, right_value, _goal, weight) do
    cond do
      left_value < right_value -> {weight, 0.0, "left"}
      right_value < left_value -> {0.0, weight, "right"}
      true -> {0.0, 0.0, "tie"}
    end
  end

  defp emit_benchmark_summary(breakdown, left_label, right_label) do
    left_score = Enum.reduce(breakdown, 0.0, &(&1["left_score"] + &2))
    right_score = Enum.reduce(breakdown, 0.0, &(&1["right_score"] + &2))
    winner = benchmark_winner(left_score, right_score, left_label, right_label)

    {:ok,
     %{
       "#{left_label}_score" => left_score,
       "#{right_label}_score" => right_score,
       "benchmark_winner" => winner,
       "benchmark_margin" => abs(left_score - right_score),
       "benchmark_criteria_count" => length(breakdown),
       "benchmark_breakdown" => breakdown,
       "benchmark_recommendation" => benchmark_recommendation(winner, left_label, right_label),
       "benchmark_summary" => "#{winner} leads across #{length(breakdown)} magnetostatic criteria"
     }}
  end

  defp benchmark_winner(left_score, right_score, left_label, right_label) do
    cond do
      left_score > right_score -> left_label
      right_score > left_score -> right_label
      true -> "tie"
    end
  end

  defp benchmark_recommendation("tie", _left_label, _right_label), do: "keep_both_under_review"
  defp benchmark_recommendation(winner, _left_label, _right_label), do: "prefer_#{winner}"
  defp number_or_nil(value) when is_number(value), do: value * 1.0
  defp number_or_nil(_value), do: nil

  defp normalize_prefix(prefix) when is_binary(prefix) and prefix != "" do
    prefix
    |> String.trim()
    |> String.replace(~r/[^a-zA-Z0-9]+/u, "_")
    |> String.trim("_")
    |> case do
      "" -> "magnetostatic"
      normalized -> normalized
    end
  end

  defp normalize_prefix(_prefix), do: "magnetostatic"
end
