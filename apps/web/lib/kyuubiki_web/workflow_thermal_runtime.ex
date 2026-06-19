defmodule KyuubikiWeb.WorkflowThermalRuntime do
  @moduledoc false

  def extract_thermal_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "thermal"))
      temperature_field = Map.get(config, "temperature_field", "temperature")
      heat_load_field = Map.get(config, "heat_load_field", "heat_load")

      temperature_nodes = collect_numeric_entries(nodes, temperature_field)
      heat_load_nodes = collect_numeric_entries(nodes, heat_load_field)

      gradient_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "gradient_x_field", "temperature_gradient_x"),
          Map.get(config, "gradient_y_field", "temperature_gradient_y"),
          Map.get(config, "gradient_z_field"),
          Map.get(config, "gradient_magnitude_field")
        )

      flux_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "flux_x_field", "heat_flux_x"),
          Map.get(config, "flux_y_field", "heat_flux_y"),
          Map.get(config, "flux_z_field"),
          Map.get(config, "flux_magnitude_field", "heat_flux_magnitude")
        )

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> merge_diagnostic_contract("thermal", "thermal_result", prefix, [
          "temperature",
          "heat_load",
          "gradient",
          "flux"
        ])
        |> maybe_merge_temperature_stats(prefix, temperature_nodes)
        |> maybe_merge_load_stats(prefix, heat_load_nodes)
        |> maybe_merge_peak_vector(prefix, "gradient", gradient_peak)
        |> maybe_merge_peak_vector(prefix, "flux", flux_peak)

      if map_size(diagnostics) <= 2 do
        {:error, :empty_thermal_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_thermal_result}
    end
  end

  def extract_thermo_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "thermo"))
      delta_field = Map.get(config, "temperature_delta_field") || Map.get(config, "temperature_field", "temperature_delta")
      stress_field = Map.get(config, "stress_field")
      delta_nodes = collect_numeric_entries(nodes, delta_field)
      displacement_peak = peak_displacement_entry(nodes, config)
      stress_peak = resolve_thermo_stress_peak(payload, elements, stress_field)
      thermal_strain_peak = resolve_thermo_strain_peak(elements, "thermal_strain")
      mechanical_strain_peak = resolve_thermo_strain_peak(elements, "mechanical_strain")
      total_strain_peak = resolve_thermo_strain_peak(elements, "total_strain")

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> merge_diagnostic_contract("thermo_mechanical", "thermo_result", prefix, [
          "temperature_delta",
          "displacement",
          "stress"
        ])
        |> maybe_merge_temperature_delta_stats(prefix, delta_nodes)
        |> maybe_merge_peak_magnitude(prefix, "displacement", displacement_peak)
        |> maybe_merge_peak_scalar(prefix, "stress", stress_peak)
        |> maybe_merge_peak_scalar(prefix, "thermal_strain", thermal_strain_peak)
        |> maybe_merge_peak_scalar(prefix, "mechanical_strain", mechanical_strain_peak)
        |> maybe_merge_peak_scalar(prefix, "total_strain", total_strain_peak)
        |> maybe_merge_thermo_contract_aliases(prefix)

      if map_size(diagnostics) <= 2 do
        {:error, :empty_thermo_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_thermo_result}
    end
  end

  def evaluate_thermal_guard(payload, config) when is_map(payload) and is_map(config) do
    rules =
      case Map.get(config, "rules") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if rules == [] do
      {:error, :invalid_thermal_guard_rules}
    else
      triggers =
        Enum.flat_map(rules, fn rule ->
          evaluate_guard_rule(payload, rule)
        end)

      status =
        cond do
          Enum.any?(triggers, &(&1["severity"] == "block")) -> "block"
          Enum.any?(triggers, &(&1["severity"] == "warn")) -> "warn"
          true -> "pass"
        end

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

  def benchmark_coupled_heat_pair(%{"left" => left, "right" => right}, config)
      when is_map(left) and is_map(right) and is_map(config) do
    criteria =
      case Map.get(config, "criteria") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if criteria == [] do
      {:error, :invalid_coupled_heat_benchmark_criteria}
    else
      left_label = normalize_prefix(Map.get(config, "left_label", "left"))
      right_label = normalize_prefix(Map.get(config, "right_label", "right"))

      breakdown =
        Enum.flat_map(criteria, fn criterion ->
          benchmark_criterion(left, right, criterion, left_label, right_label)
        end)

      if breakdown == [] do
        {:error, :empty_coupled_heat_benchmark}
      else
        left_score = Enum.reduce(breakdown, 0.0, &(&1["left_score"] + &2))
        right_score = Enum.reduce(breakdown, 0.0, &(&1["right_score"] + &2))

        {:ok,
         %{
           "#{left_label}_score" => left_score,
           "#{right_label}_score" => right_score,
           "benchmark_winner" => benchmark_winner(left_score, right_score, left_label, right_label),
           "benchmark_margin" => abs(left_score - right_score),
           "benchmark_criteria_count" => length(breakdown),
           "benchmark_left_win_count" => Enum.count(breakdown, &(&1["left_score"] > &1["right_score"])),
           "benchmark_right_win_count" => Enum.count(breakdown, &(&1["right_score"] > &1["left_score"])),
           "benchmark_tie_count" => Enum.count(breakdown, &(&1["right_score"] == &1["left_score"])),
           "benchmark_breakdown" => breakdown,
           "benchmark_recommendation" => benchmark_recommendation(left_score, right_score, left_label, right_label),
           "benchmark_summary" => benchmark_summary(left_score, right_score, left_label, right_label, breakdown)
         }}
      end
    end
  end

  def benchmark_coupled_heat_pair(_payload, _config),
    do: {:error, :invalid_coupled_heat_benchmark_pair}

  defp resolve_thermo_stress_peak(payload, elements, stress_field) when is_binary(stress_field) do
    peak_scalar_entry(elements, stress_field) || payload_scalar_peak(payload, stress_field)
  end

  defp resolve_thermo_stress_peak(payload, elements, _stress_field) do
    peak_scalar_entry(elements, "von_mises_stress") ||
      peak_scalar_entry(elements, "von_mises") ||
      payload_scalar_peak(payload, "max_stress") ||
      peak_scalar_entry_by_abs(elements, "stress_x") ||
      peak_scalar_entry_by_abs(elements, "stress_y") ||
      peak_scalar_entry_by_abs(elements, "stress_z") ||
      peak_scalar_entry_by_abs(elements, "stress_xy")
  end

  defp resolve_thermo_strain_peak(elements, base_field) when is_list(elements) and is_binary(base_field) do
    peak_scalar_entry_by_abs(elements, base_field) ||
      peak_scalar_entry_by_abs(elements, "#{base_field}_x") ||
      peak_scalar_entry_by_abs(elements, "#{base_field}_y") ||
      peak_scalar_entry_by_abs(elements, "#{base_field}_z") ||
      peak_scalar_entry_by_abs(elements, "#{base_field}_xy")
  end

  defp payload_scalar_peak(payload, field) when is_map(payload) and is_binary(field) do
    case fetch_numeric_field(payload, field) do
      {:ok, value} -> {field, value}
      :error -> nil
    end
  end

  defp payload_scalar_peak(_payload, _field), do: nil

  defp maybe_merge_thermo_contract_aliases(summary, prefix) do
    summary
    |> maybe_alias_field(
      "#{prefix}_peak_displacement",
      "#{prefix}_displacement_peak_magnitude"
    )
    |> maybe_alias_field(
      "#{prefix}_peak_displacement_id",
      "#{prefix}_displacement_peak_element_id"
    )
    |> maybe_alias_field(
      "#{prefix}_peak_stress",
      "#{prefix}_stress_peak"
    )
    |> maybe_alias_field(
      "#{prefix}_peak_stress_id",
      "#{prefix}_stress_peak_element_id"
    )
  end

  defp maybe_alias_field(summary, source_key, target_key) do
    case Map.fetch(summary, source_key) do
      {:ok, value} -> Map.put(summary, target_key, value)
      :error -> summary
    end
  end

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
    left_field = criterion_field(criterion, "left_field")
    right_field = criterion_field(criterion, "right_field")
    label_field = Map.get(criterion, "field") || "#{left_field}|#{right_field}"

    with true <- is_binary(left_field),
         true <- is_binary(right_field),
         {:ok, left_value} <- fetch_numeric_field(left, left_field),
         {:ok, right_value} <- fetch_numeric_field(right, right_field) do
      weight = normalize_weight(Map.get(criterion, "weight", 1.0))
      goal = normalize_goal(Map.get(criterion, "goal", "min"))
      {left_score, right_score} = score_benchmark_pair(left_value, right_value, goal, weight)

      [
        %{
          "field" => label_field,
          "left_field" => left_field,
          "right_field" => right_field,
          "goal" => goal,
          "weight" => weight,
          "#{left_label}_value" => left_value,
          "#{right_label}_value" => right_value,
          "delta" => right_value - left_value,
          "left_score" => left_score,
          "right_score" => right_score
        }
      ]
    else
      _ -> []
    end
  end

  defp score_benchmark_pair(left_value, right_value, "max", weight) do
    cond do
      left_value > right_value -> {weight, 0.0}
      right_value > left_value -> {0.0, weight}
      true -> {weight * 0.5, weight * 0.5}
    end
  end

  defp score_benchmark_pair(left_value, right_value, _goal, weight) do
    cond do
      left_value < right_value -> {weight, 0.0}
      right_value < left_value -> {0.0, weight}
      true -> {weight * 0.5, weight * 0.5}
    end
  end

  defp benchmark_winner(left_score, right_score, left_label, right_label) do
    cond do
      left_score > right_score -> left_label
      right_score > left_score -> right_label
      true -> "tie"
    end
  end

  defp guard_triggered?(value, rule) do
    comparison = rule_comparison(rule)
    threshold = rule_threshold(rule)

    case {comparison, threshold} do
      {"gt", threshold} when is_number(threshold) -> value > threshold
      {"gte", threshold} when is_number(threshold) -> value >= threshold
      {"lt", threshold} when is_number(threshold) -> value < threshold
      {"lte", threshold} when is_number(threshold) -> value <= threshold
      {"eq", threshold} when is_number(threshold) -> value == threshold
      _ -> false
    end
  end

  defp rule_comparison(rule) do
    case Map.get(rule, "comparison", "gte") do
      operator when operator in ["gt", "gte", "lt", "lte", "eq"] -> operator
      _ -> "gte"
    end
  end

  defp rule_threshold(rule) do
    cond do
      is_number(Map.get(rule, "threshold")) -> Map.get(rule, "threshold") * 1.0
      is_number(Map.get(rule, "value")) -> Map.get(rule, "value") * 1.0
      true -> nil
    end
  end

  defp normalize_severity("block"), do: "block"
  defp normalize_severity("warn"), do: "warn"
  defp normalize_severity(_severity), do: "warn"

  defp guard_recommendation("block"), do: "hold_and_review"
  defp guard_recommendation("warn"), do: "review_before_continue"
  defp guard_recommendation(_status), do: "continue"

  defp guard_summary("pass", _triggers), do: "All thermal guard rules passed."

  defp guard_summary(status, triggers) do
    lead =
      triggers
      |> Enum.take(2)
      |> Enum.map_join(", ", fn trigger -> "#{trigger["label"]}=#{trigger["value"]}" end)

    "#{String.upcase(status)}: #{length(triggers)} trigger(s)" <> if(lead == "", do: ".", else: " (#{lead}).")
  end

  defp normalize_goal("max"), do: "max"
  defp normalize_goal(_goal), do: "min"

  defp normalize_weight(weight) when is_number(weight) and weight > 0, do: weight * 1.0
  defp normalize_weight(_weight), do: 1.0

  defp criterion_field(criterion, key) do
    case Map.get(criterion, key) do
      field when is_binary(field) -> field
      _ -> Map.get(criterion, "field")
    end
  end

  defp benchmark_recommendation(left_score, right_score, left_label, right_label) do
    case benchmark_winner(left_score, right_score, left_label, right_label) do
      "tie" -> "keep_both_under_review"
      ^left_label -> "prefer_#{left_label}"
      ^right_label -> "prefer_#{right_label}"
    end
  end

  defp benchmark_summary(left_score, right_score, left_label, right_label, breakdown) do
    winner = benchmark_winner(left_score, right_score, left_label, right_label)
    "#{winner} across #{length(breakdown)} criteria (#{left_label}=#{left_score}, #{right_label}=#{right_score})."
  end

  defp fetch_list(map, key) when is_map(map) and is_binary(key) do
    case Map.get(map, key) do
      value when is_list(value) -> {:ok, value}
      _ -> :error
    end
  end

  defp fetch_numeric_field(map, field) when is_map(map) and is_binary(field) do
    case Map.get(map, field) do
      value when is_number(value) -> {:ok, value * 1.0}
      _ -> :error
    end
  end

  defp collect_numeric_entries(items, field) when is_list(items) and is_binary(field) do
    Enum.flat_map(items, fn
      %{} = item ->
        case fetch_numeric_field(item, field) do
          {:ok, value} -> [{Map.get(item, "id"), value}]
          :error -> []
        end

      _ ->
        []
    end)
  end

  defp peak_scalar_entry(items, field) when is_list(items) and is_binary(field) do
    items
    |> collect_numeric_entries(field)
    |> Enum.max_by(&elem(&1, 1), fn -> nil end)
  end

  defp peak_scalar_entry_by_abs(items, field) when is_list(items) and is_binary(field) do
    items
    |> collect_numeric_entries(field)
    |> Enum.max_by(&abs(elem(&1, 1)), fn -> nil end)
  end

  defp peak_vector_entry(items, x_field, y_field, z_field, magnitude_field)
       when is_list(items) and is_binary(x_field) do
    Enum.reduce(items, nil, fn
      %{} = item, best ->
        components =
          [{"x", x_field}, {"y", y_field}, {"z", z_field}]
          |> Enum.flat_map(fn
            {axis, field} when is_binary(field) ->
              case Map.get(item, field) do
                value when is_number(value) -> [{axis, value * 1.0}]
                _ -> []
              end

            _ ->
              []
          end)

        magnitude =
          case Map.get(item, magnitude_field) do
            value when is_number(value) -> value * 1.0
            _ when length(components) >= 2 ->
              components
              |> Enum.map(&elem(&1, 1))
              |> Enum.map(&(&1 * &1))
              |> Enum.sum()
              |> :math.sqrt()

            _ ->
              nil
          end

        if is_number(magnitude) do
          candidate = {Map.get(item, "id"), magnitude, components}

          case best do
            nil -> candidate
            {_id, best_value, _components} when magnitude > best_value -> candidate
            _ -> best
          end
        else
          best
        end

      _item, best ->
        best
    end)
  end

  defp peak_displacement_entry(nodes, config) do
    peak_vector_entry(
      nodes,
      Map.get(config, "displacement_x_field") || Map.get(config, "ux_field", "ux"),
      Map.get(config, "displacement_y_field") || Map.get(config, "uy_field", "uy"),
      Map.get(config, "displacement_z_field") || Map.get(config, "uz_field"),
      Map.get(config, "displacement_magnitude_field") || Map.get(config, "displacement_field", "displacement_magnitude")
    ) ||
      peak_vector_entry(
        nodes,
        Map.get(config, "displacement_x_field", "displacement_x"),
        Map.get(config, "displacement_y_field", "displacement_y"),
        Map.get(config, "displacement_z_field"),
        Map.get(config, "displacement_magnitude_field") || Map.get(config, "displacement_field", "displacement_magnitude")
      ) ||
      peak_scalar_entry(nodes, Map.get(config, "displacement_magnitude_field") || Map.get(config, "displacement_field", "displacement_magnitude"))
  end

  defp maybe_merge_temperature_stats(summary, _prefix, []), do: summary

  defp maybe_merge_temperature_stats(summary, prefix, temperature_nodes) do
    values = Enum.map(temperature_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(temperature_nodes, &elem(&1, 1))
    {min_id, min_value} = Enum.min_by(temperature_nodes, &elem(&1, 1))

    summary
    |> Map.put("#{prefix}_temperature_min", min_value)
    |> Map.put("#{prefix}_temperature_min_node_id", min_id)
    |> Map.put("#{prefix}_temperature_max", peak_value)
    |> Map.put("#{prefix}_temperature_max_node_id", peak_id)
    |> Map.put("#{prefix}_temperature_span", peak_value - min_value)
    |> Map.put("#{prefix}_temperature_mean", Enum.sum(values) / length(values))
  end

  defp maybe_merge_load_stats(summary, _prefix, []), do: summary

  defp maybe_merge_load_stats(summary, prefix, heat_load_nodes) do
    values = Enum.map(heat_load_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(heat_load_nodes, &abs(elem(&1, 1)))
    loaded_count = Enum.count(values, &(&1 != 0.0))

    summary
    |> Map.put("#{prefix}_heat_load_count", length(values))
    |> Map.put("#{prefix}_loaded_node_count", loaded_count)
    |> Map.put("#{prefix}_total_heat_load", Enum.sum(values))
    |> Map.put("#{prefix}_heat_load_mean", Enum.sum(values) / length(values))
    |> Map.put("#{prefix}_peak_heat_load", peak_value)
    |> Map.put("#{prefix}_peak_heat_load_node_id", peak_id)
  end

  defp maybe_merge_temperature_delta_stats(summary, _prefix, []), do: summary

  defp maybe_merge_temperature_delta_stats(summary, prefix, delta_nodes) do
    values = Enum.map(delta_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(delta_nodes, &elem(&1, 1))
    {min_id, min_value} = Enum.min_by(delta_nodes, &elem(&1, 1))

    summary
    |> Map.put("#{prefix}_temperature_delta_min", min_value)
    |> Map.put("#{prefix}_temperature_delta_min_node_id", min_id)
    |> Map.put("#{prefix}_temperature_delta_max", peak_value)
    |> Map.put("#{prefix}_temperature_delta_max_node_id", peak_id)
    |> Map.put("#{prefix}_temperature_delta_span", peak_value - min_value)
    |> Map.put("#{prefix}_temperature_delta_mean", Enum.sum(values) / length(values))
    |> Map.put("#{prefix}_heated_node_count", Enum.count(values, &(&1 != 0.0)))
  end

  defp maybe_merge_peak_vector(summary, _prefix, _name, nil), do: summary

  defp maybe_merge_peak_vector(summary, prefix, name, {id, magnitude, components}) do
    Enum.reduce(components, summary, fn {axis, value}, acc ->
      Map.put(acc, "#{prefix}_peak_#{name}_#{axis}", value)
    end)
    |> Map.put("#{prefix}_peak_#{name}_magnitude", magnitude)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
    |> Map.put("#{prefix}_peak_#{name}_element_id", id)
  end

  defp maybe_merge_peak_scalar(summary, _prefix, _name, nil), do: summary

  defp maybe_merge_peak_scalar(summary, prefix, name, {id, value}) do
    summary
    |> Map.put("#{prefix}_peak_#{name}", value)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
    |> Map.put("#{prefix}_peak_#{name}_element_id", id)
  end

  defp maybe_merge_peak_magnitude(summary, _prefix, _name, nil), do: summary

  defp maybe_merge_peak_magnitude(summary, prefix, name, {id, value, components}) do
    Enum.reduce(components, summary, fn {axis, component}, acc ->
      Map.put(acc, "#{prefix}_peak_#{name}_#{axis}", component)
    end)
    |> Map.put("#{prefix}_peak_#{name}", value)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
    |> Map.put("#{prefix}_peak_#{name}_element_id", id)
  end

  defp maybe_merge_peak_magnitude(summary, prefix, name, {id, value}) do
    summary
    |> Map.put("#{prefix}_peak_#{name}", value)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
    |> Map.put("#{prefix}_peak_#{name}_element_id", id)
  end

  defp merge_diagnostic_contract(summary, domain, subject, prefix, metric_groups) do
    summary
    |> Map.put("diagnostic_contract", "kyuubiki.workflow_diagnostics/v1")
    |> Map.put("diagnostic_domain", domain)
    |> Map.put("diagnostic_subject", subject)
    |> Map.put("diagnostic_prefix", prefix)
    |> Map.put("diagnostic_node_count", Map.get(summary, "#{prefix}_node_count", 0))
    |> Map.put("diagnostic_element_count", Map.get(summary, "#{prefix}_element_count", 0))
    |> Map.put("diagnostic_metric_groups", metric_groups)
  end

  defp normalize_prefix(prefix) when is_binary(prefix) do
    trimmed = String.trim(prefix)
    if trimmed == "", do: "summary", else: trimmed
  end
end
