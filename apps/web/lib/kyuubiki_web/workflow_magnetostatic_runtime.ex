defmodule KyuubikiWeb.WorkflowMagnetostaticRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDomainMetricResolver
  alias KyuubikiWeb.WorkflowDecisionChecks
  alias KyuubikiWeb.WorkflowDiagnosticSamples, as: Samples

  def extract_magnetostatic_result_diagnostics(payload, config) do
    Samples.extract(
      payload,
      config,
      "magnetostatic",
      ~w(vector_potential current_density energy_density field flux),
      fn diagnostics, config ->
        diagnostics
        |> Samples.distribution(
          :nodes,
          "vector_potential",
          Samples.field(
            config,
            "vector_potential_field",
            ~w(vector_potential a magnetic_vector_potential)
          )
        )
        |> Samples.distribution(
          :nodes,
          "current_density",
          Samples.field(
            config,
            "current_density_field",
            ~w(current_density j current_density_sum)
          )
        )
        |> Samples.scalar_peak(
          "energy_density",
          Samples.field(
            config,
            "energy_density_field",
            ~w(energy_area_density energy_density magnetic_energy_density)
          ),
          Samples.field(config, "energy_density_field", ~w(stored_energy))
        )
        |> Samples.vector_peak("field", %{
          x: Samples.field(config, "field_x_field", ~w(magnetic_field_strength_x h_x field_x)),
          y: Samples.field(config, "field_y_field", ~w(magnetic_field_strength_y h_y field_y)),
          z: Samples.field(config, "field_z_field", []),
          magnitude:
            Samples.field(
              config,
              "field_magnitude_field",
              ~w(magnetic_field_strength_magnitude h_mag field_mag)
            )
        })
        |> Samples.vector_peak("flux", %{
          x: Samples.field(config, "flux_x_field", ~w(magnetic_flux_density_x b_x flux_x)),
          y: Samples.field(config, "flux_y_field", ~w(magnetic_flux_density_y b_y flux_y)),
          z: Samples.field(config, "flux_z_field", []),
          magnitude:
            Samples.field(
              config,
              "flux_magnitude_field",
              ~w(magnetic_flux_density_magnitude b_mag flux_mag)
            )
        })
      end
    )
  end

  def evaluate_magnetostatic_guard(payload, config) do
    WorkflowDecisionChecks.guard(
      "transform.evaluate_magnetostatic_guard",
      payload,
      config,
      &do_evaluate_magnetostatic_guard/2
    )
  end

  defp do_evaluate_magnetostatic_guard(payload, config) do
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

  def benchmark_magnetostatic_pair(payload, config) do
    WorkflowDecisionChecks.pair(
      "transform.benchmark_magnetostatic_pair",
      payload,
      config,
      &do_benchmark_magnetostatic_pair/2,
      &normalize_prefix/1
    )
  end

  defp do_benchmark_magnetostatic_pair(%{"left" => left, "right" => right}, config) do
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

  defp fetch_numeric_field(entry, field) when is_map(entry) and is_binary(field) do
    WorkflowDomainMetricResolver.fetch_metric(entry, field)
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
