defmodule KyuubikiWeb.WorkflowDomainDecisionRuntime do
  @moduledoc false

  @guard_domains %{
    "transform.evaluate_structural_guard" => "structural",
    "transform.evaluate_acoustic_guard" => "acoustic",
    "transform.evaluate_modal_guard" => "modal",
    "transform.evaluate_transport_guard" => "transport"
  }

  @benchmark_domains %{
    "transform.benchmark_structural_pair" => "structural",
    "transform.benchmark_acoustic_pair" => "acoustic",
    "transform.benchmark_modal_pair" => "modal",
    "transform.benchmark_transport_pair" => "transport"
  }

  def supported_guard_operator_ids, do: Map.keys(@guard_domains)
  def supported_benchmark_operator_ids, do: Map.keys(@benchmark_domains)

  def evaluate_guard(operator_id, payload, config) when is_map(payload) and is_map(config) do
    with {:ok, domain} <- fetch_domain(@guard_domains, operator_id),
         rules when rules != [] <- config_list(config, "rules") do
      triggers = Enum.flat_map(rules, &evaluate_guard_rule(payload, &1))
      warn_count = Enum.count(triggers, &(&1["severity"] == "warn"))
      block_count = Enum.count(triggers, &(&1["severity"] == "block"))
      status = guard_status(warn_count, block_count)

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
         "guard_summary" => guard_summary(domain, status, triggers)
       }}
    else
      [] -> {:error, :invalid_domain_guard_rules}
      {:error, reason} -> {:error, reason}
      _ -> {:error, :invalid_domain_guard_payload}
    end
  end

  def evaluate_guard(operator_id, _payload, _config),
    do: {:error, {:unsupported_domain_guard_operator, operator_id}}

  def benchmark_pair(operator_id, %{"left" => left, "right" => right}, config)
      when is_map(left) and is_map(right) and is_map(config) do
    with {:ok, _domain} <- fetch_domain(@benchmark_domains, operator_id),
         criteria when criteria != [] <- config_list(config, "criteria") do
      left_label = normalize_label(Map.get(config, "left_label"), "left")
      right_label = normalize_label(Map.get(config, "right_label"), "right")

      breakdown =
        Enum.flat_map(criteria, &benchmark_criterion(left, right, &1, left_label, right_label))

      if breakdown == [] do
        {:error, :empty_domain_benchmark}
      else
        {:ok, benchmark_payload(breakdown, left_label, right_label)}
      end
    else
      [] -> {:error, :invalid_domain_benchmark_criteria}
      {:error, reason} -> {:error, reason}
      _ -> {:error, :invalid_domain_benchmark_pair}
    end
  end

  def benchmark_pair(operator_id, _payload, _config),
    do: {:error, {:unsupported_domain_benchmark_operator, operator_id}}

  defp fetch_domain(domains, operator_id) do
    case Map.fetch(domains, operator_id) do
      {:ok, domain} -> {:ok, domain}
      :error -> {:error, {:unsupported_domain_decision_operator, operator_id}}
    end
  end

  defp config_list(config, key) do
    case Map.get(config, key) do
      entries when is_list(entries) -> Enum.filter(entries, &is_map/1)
      _ -> []
    end
  end

  defp evaluate_guard_rule(payload, rule) do
    with field when is_binary(field) <- Map.get(rule, "field"),
         value when is_number(value) <- number(payload[field]),
         true <- guard_triggered?(value, rule) do
      [
        %{
          "field" => field,
          "value" => value,
          "threshold" => rule_threshold(rule),
          "comparison" => rule_comparison(rule),
          "severity" => normalize_severity(Map.get(rule, "severity")),
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

    with true <- is_binary(left_field),
         true <- is_binary(right_field),
         left_value when is_number(left_value) <- number(left[left_field]),
         right_value when is_number(right_value) <- number(right[right_field]) do
      weight = normalize_weight(Map.get(criterion, "weight"))
      goal = normalize_goal(Map.get(criterion, "goal"))
      {left_score, right_score} = score_benchmark_pair(left_value, right_value, goal, weight)

      [
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
      ]
    else
      _ -> []
    end
  end

  defp benchmark_payload(breakdown, left_label, right_label) do
    left_score = Enum.reduce(breakdown, 0.0, &(&2 + &1["left_score"]))
    right_score = Enum.reduce(breakdown, 0.0, &(&2 + &1["right_score"]))
    left_wins = Enum.count(breakdown, &(&1["left_score"] > &1["right_score"]))
    right_wins = Enum.count(breakdown, &(&1["right_score"] > &1["left_score"]))
    winner = benchmark_winner(left_score, right_score, left_label, right_label)

    %{
      "#{left_label}_score" => left_score,
      "#{right_label}_score" => right_score,
      "benchmark_winner" => winner,
      "benchmark_margin" => abs(left_score - right_score),
      "benchmark_criteria_count" => length(breakdown),
      "benchmark_left_win_count" => left_wins,
      "benchmark_right_win_count" => right_wins,
      "benchmark_tie_count" => length(breakdown) - left_wins - right_wins,
      "benchmark_breakdown" => breakdown,
      "benchmark_recommendation" => benchmark_recommendation(winner, left_label, right_label),
      "benchmark_summary" =>
        benchmark_summary(winner, left_score, right_score, left_label, right_label, breakdown)
    }
  end

  defp guard_status(_warn_count, block_count) when block_count > 0, do: "block"
  defp guard_status(warn_count, _block_count) when warn_count > 0, do: "warn"
  defp guard_status(_warn_count, _block_count), do: "pass"

  defp guard_triggered?(value, rule) do
    case {rule_comparison(rule), rule_threshold(rule)} do
      {"gt", threshold} when is_number(threshold) -> value > threshold
      {"gte", threshold} when is_number(threshold) -> value >= threshold
      {"lt", threshold} when is_number(threshold) -> value < threshold
      {"lte", threshold} when is_number(threshold) -> value <= threshold
      {"eq", threshold} when is_number(threshold) -> value == threshold
      _ -> false
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

  defp rule_comparison(%{"comparison" => comparison})
       when comparison in ["gt", "gte", "lt", "lte", "eq"],
       do: comparison

  defp rule_comparison(_rule), do: "gte"

  defp rule_threshold(rule), do: number(rule["threshold"]) || number(rule["value"])

  defp normalize_severity("block"), do: "block"
  defp normalize_severity(_severity), do: "warn"

  defp normalize_goal("max"), do: "max"
  defp normalize_goal(_goal), do: "min"

  defp normalize_weight(weight) when is_number(weight) and weight > 0, do: weight * 1.0
  defp normalize_weight(_weight), do: 1.0

  defp normalize_label(label, _fallback) when is_binary(label),
    do: label |> String.downcase() |> String.replace(~r/[^a-z0-9_]+/, "_")

  defp normalize_label(_label, fallback), do: fallback

  defp criterion_field(criterion, key) do
    case Map.get(criterion, key) do
      field when is_binary(field) -> field
      _ -> Map.get(criterion, "field")
    end
  end

  defp benchmark_winner(left_score, right_score, left_label, right_label) do
    cond do
      left_score > right_score -> left_label
      right_score > left_score -> right_label
      true -> "tie"
    end
  end

  defp guard_recommendation("block"), do: "hold_and_review"
  defp guard_recommendation("warn"), do: "review_before_continue"
  defp guard_recommendation(_status), do: "continue"

  defp benchmark_recommendation("tie", _left_label, _right_label), do: "keep_both_under_review"
  defp benchmark_recommendation(winner, _left_label, _right_label), do: "prefer_#{winner}"

  defp guard_summary(domain, "pass", _triggers), do: "All #{domain} guard rules passed."

  defp guard_summary(_domain, status, triggers) do
    lead =
      triggers
      |> Enum.take(2)
      |> Enum.map_join(", ", &"#{&1["label"]}=#{&1["value"]}")

    "#{String.upcase(status)}: #{length(triggers)} trigger(s)" <>
      if(lead == "", do: ".", else: " (#{lead}).")
  end

  defp benchmark_summary(winner, left_score, right_score, left_label, right_label, breakdown) do
    "#{winner} across #{length(breakdown)} criteria (#{left_label}=#{left_score}, #{right_label}=#{right_score})."
  end

  defp number(value) when is_number(value), do: value * 1.0
  defp number(_value), do: nil
end
