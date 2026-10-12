defmodule KyuubikiWeb.WorkflowElectrostaticRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDomainMetricResolver
  alias KyuubikiWeb.WorkflowDecisionChecks
  alias KyuubikiWeb.WorkflowDiagnosticSamples, as: Samples
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def extract_electrostatic_result_diagnostics(payload, config) do
    Samples.extract(
      payload,
      config,
      "electrostatic",
      ~w(potential charge_density energy_density field),
      fn diagnostics, config ->
        source =
          Check.choice(config, "charge_density_source", "nodes", ~w(nodes node elements element))

        source = if source in ~w(nodes node), do: :nodes, else: :elements

        diagnostics
        |> Samples.distribution(
          :nodes,
          "potential",
          Samples.field(config, "potential_field", ~w(potential phi voltage))
        )
        |> Samples.distribution(
          source,
          "charge_density",
          Samples.field(config, "charge_density_field", ~w(charge_density rho_e q_density))
        )
        |> Samples.scalar_peak(
          "energy_density",
          Samples.field(
            config,
            "energy_density_field",
            ~w(energy_density electric_energy_density stored_energy_density)
          )
        )
        |> Samples.vector_peak("field", %{
          x: Samples.field(config, "field_x_field", ~w(electric_field_x e_x)),
          y: Samples.field(config, "field_y_field", ~w(electric_field_y e_y)),
          z: Samples.field(config, "field_z_field", []),
          magnitude:
            Samples.field(
              config,
              "field_magnitude_field",
              ~w(electric_field_magnitude e_mag electric_field_abs)
            )
        })
      end
    )
  end

  def evaluate_electrostatic_guard(payload, config) do
    WorkflowDecisionChecks.guard(
      "transform.evaluate_electrostatic_guard",
      payload,
      config,
      &do_evaluate_electrostatic_guard/2
    )
  end

  defp do_evaluate_electrostatic_guard(payload, config) do
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

  def benchmark_electrostatic_pair(payload, config) do
    WorkflowDecisionChecks.pair(
      "transform.benchmark_electrostatic_pair",
      payload,
      config,
      &do_benchmark_electrostatic_pair/2,
      &normalize_prefix/1
    )
  end

  defp do_benchmark_electrostatic_pair(%{"left" => left, "right" => right}, config) do
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

  defp rule_comparison(rule) when is_map(rule) do
    case Map.get(rule, "comparison", "gt") do
      value when value in ["gt", "gte", "lt", "lte"] -> value
      _ -> "gt"
    end
  end

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

  defp benchmark_weight(criterion) when is_map(criterion) do
    case Map.get(criterion, "weight", 1.0) do
      value when is_number(value) and value > 0 -> value * 1.0
      _ -> 1.0
    end
  end

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

  defp fetch_numeric_field(entry, field) when is_map(entry) and is_binary(field) do
    WorkflowDomainMetricResolver.fetch_metric(entry, field)
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
