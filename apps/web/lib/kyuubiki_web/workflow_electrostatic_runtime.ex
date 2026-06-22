defmodule KyuubikiWeb.WorkflowElectrostaticRuntime do
  @moduledoc false

  def extract_electrostatic_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "electrostatic"))
      potential_field = Map.get(config, "potential_field", "potential")
      charge_density_field = Map.get(config, "charge_density_field", "charge_density")
      energy_density_field = Map.get(config, "energy_density_field", "energy_density")

      charge_density_entries =
        resolve_charge_density_entries(
          config,
          nodes,
          elements
        )

      potential_nodes = collect_numeric_entries(nodes, potential_field)

      charge_density_values =
        collect_numeric_entries(charge_density_entries, charge_density_field)

      energy_density_peak = peak_scalar_entry(elements, energy_density_field)

      field_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "field_x_field", "electric_field_x"),
          Map.get(config, "field_y_field", "electric_field_y"),
          Map.get(config, "field_z_field"),
          Map.get(config, "field_magnitude_field", "electric_field_magnitude")
        )

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> merge_diagnostic_contract("electrostatic", "electrostatic_result", prefix, [
          "potential",
          "charge_density",
          "energy_density",
          "field"
        ])
        |> maybe_merge_range_stats(prefix, "potential", potential_nodes)
        |> maybe_merge_distribution_stats(prefix, "charge_density", charge_density_values)
        |> maybe_merge_peak_scalar(prefix, "energy_density", energy_density_peak)
        |> maybe_merge_peak_vector(prefix, "field", field_peak)

      if map_size(diagnostics) <= 2 do
        {:error, :empty_electrostatic_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_electrostatic_result}
    end
  end

  def evaluate_electrostatic_guard(payload, config) when is_map(payload) and is_map(config) do
    rules =
      case Map.get(config, "rules") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if rules == [] do
      {:error, :invalid_electrostatic_guard_rules}
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

  def benchmark_electrostatic_pair(%{"left" => left, "right" => right}, config)
      when is_map(left) and is_map(right) and is_map(config) do
    criteria =
      case Map.get(config, "criteria") do
        entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
        _ -> []
      end

    if criteria == [] do
      {:error, :invalid_electrostatic_benchmark_criteria}
    else
      left_label = normalize_prefix(Map.get(config, "left_label", "left"))
      right_label = normalize_prefix(Map.get(config, "right_label", "right"))

      breakdown =
        Enum.flat_map(criteria, fn criterion ->
          benchmark_criterion(left, right, criterion, left_label, right_label)
        end)

      if breakdown == [] do
        {:error, :empty_electrostatic_benchmark}
      else
        left_score = Enum.reduce(breakdown, 0.0, &(&1["left_score"] + &2))
        right_score = Enum.reduce(breakdown, 0.0, &(&1["right_score"] + &2))

        {:ok,
         %{
           "#{left_label}_score" => left_score,
           "#{right_label}_score" => right_score,
           "benchmark_winner" =>
             benchmark_winner(left_score, right_score, left_label, right_label),
           "benchmark_margin" => abs(left_score - right_score),
           "benchmark_criteria_count" => length(breakdown),
           "benchmark_left_win_count" =>
             Enum.count(breakdown, &(&1["left_score"] > &1["right_score"])),
           "benchmark_right_win_count" =>
             Enum.count(breakdown, &(&1["right_score"] > &1["left_score"])),
           "benchmark_tie_count" =>
             Enum.count(breakdown, &(&1["right_score"] == &1["left_score"])),
           "benchmark_breakdown" => breakdown,
           "benchmark_recommendation" =>
             benchmark_recommendation(left_score, right_score, left_label, right_label),
           "benchmark_summary" =>
             benchmark_summary(left_score, right_score, left_label, right_label, breakdown)
         }}
      end
    end
  end

  def benchmark_electrostatic_pair(_payload, _config),
    do: {:error, :invalid_electrostatic_benchmark_pair}

  defp fetch_list(payload, key) when is_binary(key) do
    case Map.get(payload, key) do
      entries when is_list(entries) -> {:ok, entries}
      _ -> {:error, :invalid_result_collection}
    end
  end

  defp fetch_list(_payload, _key), do: {:error, :invalid_result_collection}

  defp resolve_charge_density_entries(config, nodes, elements) do
    case Map.get(config, "charge_density_source", "nodes") do
      "elements" -> elements
      "element" -> elements
      _ -> nodes
    end
  end

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
    |> Enum.filter(&is_map/1)
    |> Enum.reduce(nil, fn entry, best ->
      case fetch_numeric_field(entry, field) do
        {:ok, value} ->
          candidate = %{"entry" => entry, "value" => value}

          cond do
            is_nil(best) -> candidate
            value > best["value"] -> candidate
            true -> best
          end

        :error ->
          best
      end
    end)
  end

  defp peak_scalar_entry(_entries, _field), do: nil

  defp peak_vector_entry(entries, x_field, y_field, z_field, magnitude_field)
       when is_list(entries) and is_binary(magnitude_field) do
    entries
    |> Enum.filter(&is_map/1)
    |> Enum.reduce(nil, fn entry, best ->
      case resolve_vector_magnitude(entry, x_field, y_field, z_field, magnitude_field) do
        {:ok, magnitude, components} ->
          candidate = %{
            "entry" => entry,
            "magnitude" => magnitude,
            "components" => components
          }

          cond do
            is_nil(best) -> candidate
            magnitude > best["magnitude"] -> candidate
            true -> best
          end

        :error ->
          best
      end
    end)
  end

  defp peak_vector_entry(_entries, _x_field, _y_field, _z_field, _magnitude_field), do: nil

  defp resolve_vector_magnitude(entry, x_field, y_field, z_field, magnitude_field) do
    case fetch_numeric_field(entry, magnitude_field) do
      {:ok, magnitude} ->
        {:ok, magnitude, vector_components(entry, x_field, y_field, z_field)}

      :error ->
        with {:ok, x} <- fetch_numeric_field(entry, x_field),
             {:ok, y} <- fetch_numeric_field(entry, y_field) do
          components =
            vector_components(entry, x_field, y_field, z_field)
            |> Map.put_new("x", x)
            |> Map.put_new("y", y)

          magnitude =
            (x * x + y * y + square_optional_component(Map.get(components, "z")))
            |> :math.sqrt()

          {:ok, magnitude, components}
        else
          _ -> :error
        end
    end
  end

  defp vector_components(entry, x_field, y_field, z_field) do
    %{}
    |> maybe_put_component("x", entry, x_field)
    |> maybe_put_component("y", entry, y_field)
    |> maybe_put_component("z", entry, z_field)
  end

  defp maybe_put_component(components, _axis, _entry, field) when not is_binary(field),
    do: components

  defp maybe_put_component(components, axis, entry, field) do
    case fetch_numeric_field(entry, field) do
      {:ok, value} -> Map.put(components, axis, value)
      :error -> components
    end
  end

  defp square_optional_component(nil), do: 0.0
  defp square_optional_component(value), do: value * value

  defp maybe_merge_range_stats(diagnostics, _prefix, _label, []), do: diagnostics

  defp maybe_merge_range_stats(diagnostics, prefix, label, entries) do
    values = Enum.map(entries, &elem(&1, 1))
    mean = Enum.sum(values) / length(values)

    diagnostics
    |> Map.put("#{prefix}_#{label}_min", Enum.min(values))
    |> Map.put("#{prefix}_#{label}_max", Enum.max(values))
    |> Map.put("#{prefix}_#{label}_mean", mean)
    |> Map.put("#{prefix}_#{label}_span", Enum.max(values) - Enum.min(values))
  end

  defp maybe_merge_distribution_stats(diagnostics, _prefix, _label, []), do: diagnostics

  defp maybe_merge_distribution_stats(diagnostics, prefix, label, entries) do
    values = Enum.map(entries, &elem(&1, 1))

    diagnostics
    |> Map.put("#{prefix}_#{label}_count", length(values))
    |> Map.put("#{prefix}_#{label}_min", Enum.min(values))
    |> Map.put("#{prefix}_#{label}_max", Enum.max(values))
    |> Map.put("#{prefix}_#{label}_mean", Enum.sum(values) / length(values))
    |> Map.put("#{prefix}_#{label}_sum", Enum.sum(values))
  end

  defp maybe_merge_peak_scalar(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_scalar(diagnostics, prefix, label, peak) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak", peak["value"])
    |> Map.put("#{prefix}_#{label}_peak_element_id", get_in(peak, ["entry", "id"]))
  end

  defp maybe_merge_peak_vector(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_vector(diagnostics, prefix, label, peak) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak_magnitude", peak["magnitude"])
    |> Map.put("#{prefix}_#{label}_peak_element_id", get_in(peak, ["entry", "id"]))
    |> maybe_put_vector_components(prefix, label, peak["components"])
  end

  defp maybe_put_vector_components(diagnostics, prefix, label, components) do
    Enum.reduce(components, diagnostics, fn {axis, value}, acc ->
      Map.put(acc, "#{prefix}_#{label}_peak_#{axis}", value)
    end)
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

  defp guard_triggered?(value, rule) when is_number(value) and is_map(rule) do
    threshold = rule_threshold(rule)

    cond do
      threshold == nil ->
        false

      rule_comparison(rule) in ["lt", "lte"] ->
        compare_guard_values(value, threshold, rule_comparison(rule))

      true ->
        compare_guard_values(value, threshold, rule_comparison(rule))
    end
  end

  defp guard_triggered?(_value, _rule), do: false

  defp compare_guard_values(value, threshold, "gt"), do: value > threshold
  defp compare_guard_values(value, threshold, "gte"), do: value >= threshold
  defp compare_guard_values(value, threshold, "lt"), do: value < threshold
  defp compare_guard_values(value, threshold, "lte"), do: value <= threshold
  defp compare_guard_values(value, threshold, _comparison), do: value > threshold

  defp rule_threshold(rule) when is_map(rule) do
    case Map.get(rule, "threshold") do
      value when is_number(value) -> value * 1.0
      _ -> nil
    end
  end

  defp rule_threshold(_rule), do: nil

  defp rule_comparison(rule) when is_map(rule) do
    case Map.get(rule, "comparison", "gt") do
      value when value in ["gt", "gte", "lt", "lte"] -> value
      _ -> "gt"
    end
  end

  defp rule_comparison(_rule), do: "gt"

  defp normalize_severity("block"), do: "block"
  defp normalize_severity(_severity), do: "warn"

  defp guard_recommendation("block"), do: "hold_and_review"
  defp guard_recommendation("warn"), do: "review_before_coupling"
  defp guard_recommendation(_status), do: "ready_for_next_stage"

  defp guard_summary("block", triggers),
    do:
      "BLOCK: #{length(triggers)} electrostatic guard trigger(s) exceeded configured thresholds."

  defp guard_summary("warn", triggers),
    do: "WARN: #{length(triggers)} electrostatic guard trigger(s) exceeded configured thresholds."

  defp guard_summary(_status, _triggers),
    do: "PASS: no electrostatic guard triggers exceeded configured thresholds."

  defp benchmark_criterion(left, right, criterion, left_label, right_label) do
    with field when is_binary(field) <- Map.get(criterion, "field"),
         left_field when is_binary(left_field) <- Map.get(criterion, "left_field"),
         right_field when is_binary(right_field) <- Map.get(criterion, "right_field"),
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

  defp benchmark_goal(criterion) when is_map(criterion) do
    case Map.get(criterion, "goal", "min") do
      value when value in ["min", "max"] -> value
      _ -> "min"
    end
  end

  defp benchmark_goal(_criterion), do: "min"

  defp benchmark_weight(criterion) when is_map(criterion) do
    case Map.get(criterion, "weight", 1.0) do
      value when is_number(value) and value > 0 -> value * 1.0
      _ -> 1.0
    end
  end

  defp benchmark_weight(_criterion), do: 1.0

  defp benchmark_scores(left_value, right_value, goal, weight) do
    cond do
      left_value == right_value ->
        {0.0, 0.0, "tie"}

      goal == "max" and left_value > right_value ->
        {weight, 0.0, "left"}

      goal == "max" ->
        {0.0, weight, "right"}

      left_value < right_value ->
        {weight, 0.0, "left"}

      true ->
        {0.0, weight, "right"}
    end
  end

  defp benchmark_winner(left_score, right_score, left_label, right_label) do
    cond do
      left_score > right_score -> left_label
      right_score > left_score -> right_label
      true -> "tie"
    end
  end

  defp benchmark_recommendation(left_score, right_score, left_label, right_label) do
    cond do
      left_score > right_score -> "prefer_#{left_label}"
      right_score > left_score -> "prefer_#{right_label}"
      true -> "keep_both_under_review"
    end
  end

  defp benchmark_summary(left_score, right_score, left_label, right_label, breakdown) do
    winner = benchmark_winner(left_score, right_score, left_label, right_label)

    case winner do
      "tie" -> "tie across #{length(breakdown)} criteria"
      ^left_label -> "#{left_label} leads across #{length(breakdown)} criteria"
      ^right_label -> "#{right_label} leads across #{length(breakdown)} criteria"
    end
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

  defp normalize_prefix(prefix) when is_binary(prefix) and prefix != "" do
    prefix
    |> String.trim()
    |> String.replace(~r/[^a-zA-Z0-9]+/u, "_")
    |> String.trim("_")
    |> case do
      "" -> "electrostatic"
      normalized -> normalized
    end
  end

  defp normalize_prefix(_prefix), do: "electrostatic"
end
