defmodule KyuubikiWeb.WorkflowCfdRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDomainMetricResolver
  alias KyuubikiWeb.WorkflowDecisionChecks
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def extract_stokes_flow_result_diagnostics(payload, config) do
    Check.run("extract.stokes_flow_result_diagnostics", fn ->
      payload = Check.object(payload, "payload")
      config = Check.object(config, "config")
      Check.converged(payload, "payload")
      nodes = samples(payload, "nodes")
      elements = samples(payload, "elements")
      prefix = normalize_prefix(Map.get(config, "output_prefix", "cfd"))
      node_stats = sample_stats(nodes, ~w(velocity_magnitude pressure), "nodes")

      element_stats =
        sample_stats(
          elements,
          ~w(divergence_error reynolds_number viscous_dissipation),
          "elements"
        )

      %{
        "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
        "diagnostic_domain" => "fluid",
        "diagnostic_subject" => "stokes_flow_result",
        "diagnostic_prefix" => prefix,
        "diagnostic_node_count" => length(nodes),
        "diagnostic_element_count" => length(elements)
      }
      |> merge_bounds("#{prefix}_velocity", node_stats["velocity_magnitude"])
      |> merge_bounds("#{prefix}_pressure", node_stats["pressure"])
      |> merge_peak("#{prefix}_divergence_error", element_stats["divergence_error"])
      |> merge_peak("#{prefix}_reynolds_number", element_stats["reynolds_number"])
      |> merge_peak("#{prefix}_viscous_dissipation", element_stats["viscous_dissipation"])
      |> Map.put("#{prefix}_velocity_mean", node_stats["velocity_magnitude"].mean)
      |> Map.put("#{prefix}_pressure_mean", node_stats["pressure"].mean)
      |> Map.put("#{prefix}_divergence_error_mean", element_stats["divergence_error"].mean)
      |> Map.put("#{prefix}_reynolds_number_mean", element_stats["reynolds_number"].mean)
      |> Map.put("#{prefix}_viscous_dissipation_total", element_stats["viscous_dissipation"].sum)
    end)
  end

  def evaluate_cfd_guard(payload, config) do
    WorkflowDecisionChecks.guard(
      "transform.evaluate_cfd_guard",
      payload,
      config,
      &do_evaluate_cfd_guard/2
    )
  end

  defp do_evaluate_cfd_guard(payload, config) do
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

  def benchmark_cfd_pair(payload, config) do
    WorkflowDecisionChecks.pair(
      "transform.benchmark_cfd_pair",
      payload,
      config,
      &do_benchmark_cfd_pair/2
    )
  end

  defp do_benchmark_cfd_pair(payload, config) do
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

  defp samples(payload, source) do
    rows = Check.list(payload[source], "payload.#{source}")
    if rows == [], do: Check.fail("payload.#{source} must be a non-empty array")
    rows
  end

  defp sample_stats(rows, fields, source) do
    {_count, stats} =
      Enum.reduce(rows, {0, %{}}, fn row, {index, stats} ->
        path = "payload.#{source}[#{index}]"
        row = Check.object(row, path)

        stats =
          Enum.reduce(fields, stats, fn field, acc ->
            value =
              case WorkflowDomainMetricResolver.checked_metric_value(row, field, path) do
                {:ok, value} -> Check.number(value, "#{path}.#{field}")
                {:error, reason} -> Check.fail(reason)
              end

            next = observe(Map.get(acc, field), value, row["id"], index + 1, "#{path}.#{field}")

            next =
              if field == "viscous_dissipation" do
                sum =
                  Check.calculate("#{path}.#{field}.sum", fn ->
                    Map.get(next, :sum, 0.0) + value
                  end)

                Map.put(next, :sum, sum)
              else
                next
              end

            Map.put(acc, field, next)
          end)

        {index + 1, stats}
      end)

    stats
  end

  defp observe(nil, value, id, _count, _path),
    do: %{min: value, max: value, mean: value, peak: value, peak_id: id}

  defp observe(stats, value, id, count, path) do
    # Weighted means avoid overflowing an unused raw sum, matching native diagnostics.
    mean =
      Check.calculate("#{path}.mean", fn -> stats.mean * ((count - 1) / count) + value / count end)

    stats = %{stats | min: min(stats.min, value), max: max(stats.max, value), mean: mean}
    if abs(value) >= abs(stats.peak), do: %{stats | peak: value, peak_id: id}, else: stats
  end

  defp merge_bounds(summary, key, stats) do
    span = Check.calculate("#{key}_span", fn -> stats.max - stats.min end)

    Map.merge(summary, %{
      "#{key}_min" => stats.min,
      "#{key}_max" => stats.max,
      "#{key}_span" => span
    })
  end

  defp merge_peak(summary, key, stats),
    do:
      Map.merge(summary, %{"#{key}_peak" => stats.peak, "#{key}_peak_element_id" => stats.peak_id})

  defp number?(value), do: is_integer(value) or is_float(value)

  defp guard_triggered?(payload, rule) do
    field = Map.get(rule, "field")
    threshold = Map.get(rule, "threshold", Map.get(rule, "value"))
    value = WorkflowDomainMetricResolver.metric_value(payload, field)

    number?(value) and number?(threshold) and
      compare_guard(value, threshold, Map.get(rule, "comparison", "gte"))
  end

  defp guard_trigger(payload, rule) do
    field = Map.fetch!(rule, "field")
    value = WorkflowDomainMetricResolver.metric_value(payload, field)

    %{
      "field" => field,
      "value" => value,
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
    number?(
      WorkflowDomainMetricResolver.metric_value(left, criterion_field(criterion, "left_field"))
    ) and
      number?(
        WorkflowDomainMetricResolver.metric_value(
          right,
          criterion_field(criterion, "right_field")
        )
      )
  end

  defp benchmark_criterion(left, right, criterion, left_label, right_label) do
    left_field = criterion_field(criterion, "left_field")
    right_field = criterion_field(criterion, "right_field")
    left_value = WorkflowDomainMetricResolver.metric_value(left, left_field)
    right_value = WorkflowDomainMetricResolver.metric_value(right, right_field)
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
