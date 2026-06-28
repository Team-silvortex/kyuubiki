defmodule KyuubikiWeb.WorkflowCfdRuntime do
  @moduledoc false

  def extract_stokes_flow_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    nodes = Map.get(payload, "nodes", [])
    elements = Map.get(payload, "elements", [])

    with true <- is_list(nodes) and is_list(elements),
         false <- nodes == [] and elements == [] do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "cfd"))
      velocity_values = numeric_values(nodes, "velocity_magnitude")
      pressure_values = numeric_values(nodes, "pressure")
      divergence_values = numeric_values(elements, "divergence_error")
      reynolds_values = numeric_values(elements, "reynolds_number")
      dissipation_values = numeric_values(elements, "viscous_dissipation")

      summary =
        %{
          "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
          "diagnostic_domain" => "fluid",
          "diagnostic_subject" => "stokes_flow_result",
          "diagnostic_prefix" => prefix,
          "diagnostic_node_count" => length(nodes),
          "diagnostic_element_count" => length(elements)
        }
        |> merge_min_max("#{prefix}_velocity", velocity_values)
        |> merge_min_max("#{prefix}_pressure", pressure_values)
        |> merge_peak("#{prefix}_divergence_error", elements, "divergence_error")
        |> merge_peak("#{prefix}_reynolds_number", elements, "reynolds_number")
        |> merge_peak("#{prefix}_viscous_dissipation", elements, "viscous_dissipation")
        |> Map.put("#{prefix}_velocity_mean", mean_or_zero(velocity_values))
        |> Map.put("#{prefix}_pressure_mean", mean_or_zero(pressure_values))
        |> Map.put("#{prefix}_divergence_error_mean", mean_or_zero(divergence_values))
        |> Map.put("#{prefix}_reynolds_number_mean", mean_or_zero(reynolds_values))
        |> Map.put("#{prefix}_viscous_dissipation_total", Enum.sum(dissipation_values))

      {:ok, summary}
    else
      _ -> {:error, :invalid_stokes_flow_result}
    end
  end

  def extract_stokes_flow_result_diagnostics(_payload, _config),
    do: {:error, :invalid_stokes_flow_result}

  def evaluate_cfd_guard(payload, config) when is_map(payload) and is_map(config) do
    rules = Map.get(config, "rules", [])

    if is_list(rules) and rules != [] do
      triggers =
        rules
        |> Enum.filter(&is_map/1)
        |> Enum.flat_map(fn rule ->
          if guard_triggered?(payload, rule), do: [guard_trigger(payload, rule)], else: []
        end)

      block_count = Enum.count(triggers, &(Map.get(&1, "severity") == "block"))
      warn_count = Enum.count(triggers, &(Map.get(&1, "severity") == "warn"))
      status = guard_status(block_count, warn_count)

      {:ok,
       %{
         "guard_status" => status,
         "guard_passed" => status == "pass",
         "guard_trigger_count" => length(triggers),
         "guard_checked_rule_count" => length(rules),
         "guard_warn_count" => warn_count,
         "guard_block_count" => block_count,
         "guard_triggers" => triggers,
         "guard_recommendation" => guard_recommendation(status),
         "guard_summary" => guard_summary(status, triggers)
       }}
    else
      {:error, :invalid_cfd_guard_rules}
    end
  end

  def evaluate_cfd_guard(_payload, _config), do: {:error, :invalid_cfd_guard}

  def benchmark_cfd_pair(payload, config) when is_map(payload) and is_map(config) do
    with %{} = left <- Map.get(payload, "left"),
         %{} = right <- Map.get(payload, "right"),
         criteria when is_list(criteria) and criteria != [] <- Map.get(config, "criteria") do
      left_label = normalize_label(Map.get(config, "left_label"), "left")
      right_label = normalize_label(Map.get(config, "right_label"), "right")

      breakdown =
        criteria
        |> Enum.filter(&is_map/1)
        |> Enum.flat_map(fn criterion ->
          if benchmarkable?(left, right, criterion) do
            [benchmark_criterion(left, right, criterion, left_label, right_label)]
          else
            []
          end
        end)

      if breakdown == [] do
        {:error, :empty_cfd_benchmark}
      else
        {:ok, benchmark_summary(breakdown, left_label, right_label)}
      end
    else
      _ -> {:error, :invalid_cfd_benchmark_criteria}
    end
  end

  def benchmark_cfd_pair(_payload, _config), do: {:error, :invalid_cfd_benchmark_pair}

  defp merge_min_max(summary, key, []),
    do: summary |> Map.put("#{key}_min", 0.0) |> Map.put("#{key}_max", 0.0)

  defp merge_min_max(summary, key, values),
    do:
      summary
      |> Map.put("#{key}_min", Enum.min(values))
      |> Map.put("#{key}_max", Enum.max(values))

  defp merge_peak(summary, key, elements, field) do
    case peak_element(elements, field) do
      nil ->
        summary
        |> Map.put("#{key}_peak", 0.0)
        |> Map.put("#{key}_peak_element_id", nil)

      {element, value} ->
        summary
        |> Map.put("#{key}_peak", value)
        |> Map.put("#{key}_peak_element_id", Map.get(element, "id"))
    end
  end

  defp peak_element(elements, field) do
    elements
    |> Enum.filter(&is_map/1)
    |> Enum.filter(&number?(Map.get(&1, field)))
    |> Enum.max_by(fn element -> abs(Map.fetch!(element, field)) end, fn -> nil end)
    |> case do
      nil -> nil
      element -> {element, Map.fetch!(element, field)}
    end
  end

  defp numeric_values(values, field) do
    values
    |> Enum.filter(&is_map/1)
    |> Enum.map(&Map.get(&1, field))
    |> Enum.filter(&number?/1)
  end

  defp mean_or_zero([]), do: 0.0
  defp mean_or_zero(values), do: Enum.sum(values) / length(values)

  defp number?(value), do: is_integer(value) or is_float(value)

  defp guard_triggered?(payload, rule) do
    field = Map.get(rule, "field")
    threshold = Map.get(rule, "threshold", Map.get(rule, "value"))
    value = Map.get(payload, field)

    number?(value) and number?(threshold) and
      compare_guard(value, threshold, Map.get(rule, "comparison", "gte"))
  end

  defp guard_trigger(payload, rule) do
    field = Map.fetch!(rule, "field")

    %{
      "field" => field,
      "value" => Map.fetch!(payload, field),
      "threshold" => Map.get(rule, "threshold", Map.get(rule, "value")),
      "comparison" => Map.get(rule, "comparison", "gte"),
      "severity" => normalize_severity(Map.get(rule, "severity")),
      "label" => Map.get(rule, "label", field)
    }
  end

  defp compare_guard(value, threshold, "gt"), do: value > threshold
  defp compare_guard(value, threshold, "lt"), do: value < threshold
  defp compare_guard(value, threshold, "lte"), do: value <= threshold
  defp compare_guard(value, threshold, "eq"), do: value == threshold
  defp compare_guard(value, threshold, _comparison), do: value >= threshold

  defp normalize_severity("block"), do: "block"
  defp normalize_severity(_severity), do: "warn"

  defp guard_status(block_count, _warn_count) when block_count > 0, do: "block"
  defp guard_status(_block_count, warn_count) when warn_count > 0, do: "warn"
  defp guard_status(_block_count, _warn_count), do: "pass"

  defp guard_recommendation("block"), do: "hold_and_review"
  defp guard_recommendation("warn"), do: "review_before_continue"
  defp guard_recommendation(_status), do: "continue"

  defp guard_summary("pass", _triggers), do: "All CFD guard rules passed."

  defp guard_summary(status, triggers),
    do: "#{String.upcase(status)}: #{length(triggers)} CFD guard trigger(s)."

  defp benchmarkable?(left, right, criterion) do
    number?(Map.get(left, criterion_field(criterion, "left_field"))) and
      number?(Map.get(right, criterion_field(criterion, "right_field")))
  end

  defp benchmark_criterion(left, right, criterion, left_label, right_label) do
    left_field = criterion_field(criterion, "left_field")
    right_field = criterion_field(criterion, "right_field")
    left_value = Map.fetch!(left, left_field)
    right_value = Map.fetch!(right, right_field)
    weight = normalize_weight(Map.get(criterion, "weight"))
    goal = Map.get(criterion, "goal", "min")
    {left_score, right_score} = score_pair(left_value, right_value, goal, weight)

    %{
      "field" => Map.get(criterion, "field", left_field),
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
  end

  defp criterion_field(criterion, key), do: Map.get(criterion, key, Map.get(criterion, "field"))

  defp normalize_weight(value) when (is_integer(value) or is_float(value)) and value > 0.0,
    do: value

  defp normalize_weight(_value), do: 1.0

  defp score_pair(left, right, "max", weight) when left > right, do: {weight, 0.0}
  defp score_pair(left, right, "max", weight) when right > left, do: {0.0, weight}
  defp score_pair(left, right, _goal, weight) when left < right, do: {weight, 0.0}
  defp score_pair(left, right, _goal, weight) when right < left, do: {0.0, weight}
  defp score_pair(_left, _right, _goal, weight), do: {weight * 0.5, weight * 0.5}

  defp benchmark_summary(breakdown, left_label, right_label) do
    left_score = Enum.sum(Enum.map(breakdown, &Map.fetch!(&1, "left_score")))
    right_score = Enum.sum(Enum.map(breakdown, &Map.fetch!(&1, "right_score")))
    winner = benchmark_winner(left_score, right_score, left_label, right_label)

    %{
      "#{left_label}_score" => left_score,
      "#{right_label}_score" => right_score,
      "benchmark_winner" => winner,
      "benchmark_margin" => abs(left_score - right_score),
      "benchmark_criteria_count" => length(breakdown),
      "benchmark_breakdown" => breakdown,
      "benchmark_recommendation" => benchmark_recommendation(winner, left_label, right_label),
      "benchmark_summary" => "#{winner} across #{length(breakdown)} CFD criteria."
    }
  end

  defp benchmark_winner(left_score, right_score, left_label, _right_label)
       when left_score > right_score, do: left_label

  defp benchmark_winner(left_score, right_score, _left_label, right_label)
       when right_score > left_score, do: right_label

  defp benchmark_winner(_left_score, _right_score, _left_label, _right_label), do: "tie"

  defp benchmark_recommendation("tie", _left_label, _right_label), do: "keep_both_under_review"
  defp benchmark_recommendation(winner, _left_label, _right_label), do: "prefer_#{winner}"

  defp normalize_label(value, default_value) when is_binary(value) do
    case String.trim(value) do
      "" -> default_value
      label -> label
    end
  end

  defp normalize_label(_value, default_value), do: default_value

  defp normalize_prefix(prefix) when is_binary(prefix) do
    case String.trim(prefix) do
      "" -> "cfd"
      value -> value
    end
  end

  defp normalize_prefix(_prefix), do: "cfd"
end
