defmodule KyuubikiWeb.WorkflowThermalRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDecisionChecks
  alias KyuubikiWeb.WorkflowThermalDiagnostics

  defdelegate extract_thermal_result_diagnostics(payload, config),
    to: WorkflowThermalDiagnostics,
    as: :thermal

  defdelegate extract_thermo_result_diagnostics(payload, config),
    to: WorkflowThermalDiagnostics,
    as: :thermo

  def evaluate_thermal_guard(payload, config) do
    WorkflowDecisionChecks.guard(
      "transform.evaluate_thermal_guard",
      payload,
      config,
      &evaluate_guard/2
    )
  end

  def benchmark_coupled_heat_pair(payload, config) do
    WorkflowDecisionChecks.pair(
      "transform.benchmark_coupled_heat_pair",
      payload,
      config,
      &benchmark_pair/2
    )
  end

  defp evaluate_guard(payload, config) do
    rules = config["rules"]

    triggers =
      Enum.flat_map(rules, fn rule ->
        value = payload[rule["field"]]

        if guard_triggered?(value, rule["threshold"], rule["comparison"]) do
          [
            Map.take(rule, ~w(field threshold comparison severity label))
            |> Map.put("value", value)
          ]
        else
          []
        end
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

  defp benchmark_pair(%{"left" => left, "right" => right}, config) do
    left_label = config["left_label"]
    right_label = config["right_label"]

    breakdown =
      Enum.map(config["criteria"], fn criterion ->
        left_field = criterion["left_field"]
        right_field = criterion["right_field"]
        left_value = left[left_field]
        right_value = right[right_field]
        goal = criterion["goal"]
        weight = criterion["weight"]
        {left_score, right_score} = score_pair(left_value, right_value, goal, weight)

        %{
          "field" => criterion["field"],
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
      end)

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
       "benchmark_left_win_count" =>
         Enum.count(breakdown, &(&1["left_score"] > &1["right_score"])),
       "benchmark_right_win_count" =>
         Enum.count(breakdown, &(&1["right_score"] > &1["left_score"])),
       "benchmark_tie_count" => Enum.count(breakdown, &(&1["right_score"] == &1["left_score"])),
       "benchmark_breakdown" => breakdown,
       "benchmark_recommendation" =>
         if(winner == "tie", do: "keep_both_under_review", else: "prefer_#{winner}"),
       "benchmark_summary" =>
         "#{winner} across #{length(breakdown)} criteria (#{left_label}=#{left_score}, #{right_label}=#{right_score})."
     }}
  end

  defp score_pair(left, right, goal, weight) do
    cond do
      left == right -> {weight * 0.5, weight * 0.5}
      (goal == "max" and left > right) or (goal == "min" and left < right) -> {weight, 0.0}
      true -> {0.0, weight}
    end
  end

  defp benchmark_winner(left, right, left_label, right_label) do
    cond do
      left > right -> left_label
      right > left -> right_label
      true -> "tie"
    end
  end

  defp guard_triggered?(value, threshold, "gt"), do: value > threshold
  defp guard_triggered?(value, threshold, "gte"), do: value >= threshold
  defp guard_triggered?(value, threshold, "lt"), do: value < threshold
  defp guard_triggered?(value, threshold, "lte"), do: value <= threshold
  defp guard_triggered?(value, threshold, "eq"), do: value == threshold

  defp guard_recommendation("block"), do: "hold_and_review"
  defp guard_recommendation("warn"), do: "review_before_continue"
  defp guard_recommendation(_status), do: "continue"

  defp guard_summary("pass", _triggers), do: "All thermal guard rules passed."

  defp guard_summary(status, triggers) do
    lead =
      triggers
      |> Enum.take(2)
      |> Enum.map_join(", ", fn trigger ->
        "#{trigger["label"]}=#{trigger["value"]}"
      end)

    "#{String.upcase(status)}: #{length(triggers)} trigger(s)" <>
      if(lead == "", do: ".", else: " (#{lead}).")
  end
end
