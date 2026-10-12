defmodule KyuubikiWeb.WorkflowDecisionChecks do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDomainMetricResolver, as: Metrics
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  # Validate rules before domain rendering without duplicating any Engine physics.
  def guard(operator, payload, config, evaluate) do
    run(operator, fn ->
      payload = result(payload, "payload")
      config = Check.object(config, "config")

      {rules, payload} =
        entries(config, "rules")
        |> Enum.with_index()
        |> Enum.map_reduce(payload, fn {rule, index}, metrics ->
          path = "config.rules[#{index}]"
          rule = Check.object(rule, path)
          field = required_text(rule, "field", path)
          threshold = Check.optional_number(rule, "threshold", path)
          legacy_value = Check.optional_number(rule, "value", path)

          if threshold != nil and legacy_value != nil and threshold != legacy_value,
            do: Check.fail("#{path}.threshold conflicts with #{path}.value")

          threshold = threshold || legacy_value
          if threshold == nil, do: Check.fail("#{path}.threshold (or value) is required")
          value = metric(metrics, field, "#{path}: payload")

          normalized =
            Map.merge(rule, %{
              "field" => field,
              "threshold" => threshold,
              "comparison" => Check.choice(rule, "comparison", "gte", ~w(gt gte lt lte eq), path),
              "severity" => Check.choice(rule, "severity", "warn", ~w(warn block), path),
              "label" => Check.text(rule, "label", path, field)
            })

          {normalized, Map.put(metrics, field, value)}
        end)

      evaluate.(payload, Map.put(config, "rules", rules))
    end)
  end

  def pair(operator, payload, config, evaluate, normalize_label \\ &Function.identity/1) do
    run(operator, fn ->
      payload = result(payload, "payload")
      left = result(payload["left"], "payload.left")
      right = result(payload["right"], "payload.right")
      config = Check.object(config, "config")
      left_label = label(config, "left_label", "left", normalize_label)
      right_label = label(config, "right_label", "right", normalize_label)

      if left_label == right_label,
        do: Check.fail("config.left_label and config.right_label must remain distinct")

      {criteria, {left, right}} =
        entries(config, "criteria")
        |> Enum.with_index()
        |> Enum.map_reduce({left, right}, fn {criterion, index}, {left, right} ->
          path = "config.criteria[#{index}]"
          criterion = Check.object(criterion, path)
          common = Check.text(criterion, "field", path)
          left_field = required_text(criterion, "left_field", path, common)
          right_field = required_text(criterion, "right_field", path, common)
          weight = Check.optional_number(criterion, "weight", path) || 1.0
          if weight <= 0, do: Check.fail("#{path}.weight must be positive")
          left_value = metric(left, left_field, "#{path}: payload.left")
          right_value = metric(right, right_field, "#{path}: payload.right")
          Check.calculate("#{path}.delta", fn -> right_value - left_value end)

          normalized =
            Map.merge(criterion, %{
              "field" => common || left_field,
              "left_field" => left_field,
              "right_field" => right_field,
              "weight" => weight,
              "goal" => Check.choice(criterion, "goal", "min", ~w(min max), path)
            })

          {normalized,
           {Map.put(left, left_field, left_value), Map.put(right, right_field, right_value)}}
        end)

      config =
        Map.merge(config, %{
          "criteria" => criteria,
          "left_label" => left_label,
          "right_label" => right_label
        })

      evaluate.(Map.merge(payload, %{"left" => left, "right" => right}), config)
    end)
  end

  defp run(operator, work) do
    case Check.run(operator, fn -> Check.calculate("decision", work) end) do
      {:ok, result} -> result
      {:error, _reason} = error -> error
    end
  end

  defp result(value, path) do
    object = Check.object(value, path)
    Check.converged(object, path)
    object
  end

  defp entries(config, key) do
    entries = Check.list(config[key], "config.#{key}")
    if entries == [], do: Check.fail("config.#{key} must not be empty")
    entries
  end

  defp required_text(object, key, path, default \\ nil) do
    Check.text(object, key, path, default) || Check.fail("#{path}.#{key} is required")
  end

  defp label(config, key, default, normalize) do
    value = config |> Check.text(key, "config", default) |> normalize.()
    if value == "", do: Check.fail("config.#{key} must be nonblank after normalization")

    if String.downcase(value) == "tie",
      do: Check.fail("config.#{key} cannot use reserved label tie")

    value
  end

  defp metric(payload, field, path) do
    case Metrics.checked_metric_value(payload, field, path) do
      {:ok, value} -> Check.number(value, "#{path}.#{field}")
      {:error, reason} -> Check.fail(reason)
    end
  end
end
