defmodule KyuubikiWeb.WorkflowQualityComposition do
  @moduledoc false

  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  @numeric_config ~w(missing_metric_penalty not_ready_penalty max_ready_score target_score max_candidates)
  @suffixes ~w(score ready grade contract missing_metric_count watch_count dominant_term blocking_terms)
  @compact ~w(source domain grade ready contribution dominant_term)

  def run(payload, config) do
    Check.run("transform.compose_quality_objective", fn ->
      payload = Check.object(payload, "payload")
      config = checked_config(config)
      entries = entries(payload)
      terms = Enum.map(entries, fn {id, summary} -> term(id, summary, config) end)

      score =
        Check.calculate("composite_quality_score", fn ->
          Enum.sum(Enum.map(terms, & &1["contribution"]))
        end)

      missing = total_count(terms, "missing_metric_count")
      watch = total_count(terms, "watch_count")
      blocked = Enum.reject(terms, & &1["ready"])
      limit = nonnegative(Map.get(config, "max_ready_score", 12), "config.max_ready_score")
      grade = grade(score, blocked, limit)

      dominant =
        Enum.reduce(terms, fn term, best ->
          if term["contribution"] >= best["contribution"], do: term, else: best
        end)

      %{
        "composite_quality_contract" => "kyuubiki.composite_quality_objective/v1",
        "composite_quality_score" => score,
        "composite_quality_grade" => grade,
        "composite_quality_ready" => grade != "block",
        "composite_quality_term_count" => length(terms),
        "composite_quality_missing_metric_count" => missing,
        "composite_quality_watch_count" => watch,
        "composite_quality_blocked_term_count" => length(blocked),
        "composite_quality_max_ready_score" => limit,
        "composite_quality_dominant_term" => Map.take(dominant, @compact),
        "composite_quality_blocking_terms" =>
          Enum.map(blocked, fn term ->
            term |> Map.take(@compact) |> Map.put("source_blocking_terms", term["blocking_terms"])
          end),
        "composite_quality_terms" => terms,
        "composite_quality_summary" =>
          "Composite quality #{grade}: score=#{score}, blocked_terms=#{length(blocked)}, missing_metrics=#{missing}."
      }
    end)
  end

  def checked_config(value, path \\ "config") do
    config = if is_nil(value), do: %{}, else: Check.object(value, path)
    weights = Check.object(Map.get(config, "weights", %{}), "#{path}.weights")
    Enum.each(weights, fn {id, value} -> nonnegative(value, "#{path}.weights.#{id}") end)

    for key <- @numeric_config,
        Map.has_key?(config, key),
        do: nonnegative(config[key], "#{path}.#{key}")

    config
  end

  def nonnegative(value, path) do
    number = Check.number(value, path)
    if number < 0, do: Check.fail("#{path} must be nonnegative")
    number
  end

  def boolean(value, _path) when is_boolean(value), do: value
  def boolean(_value, path), do: Check.fail("#{path} must be a boolean")

  defp entries(payload) do
    source =
      if Map.has_key?(payload, "qualities") do
        Check.converged(payload, "payload")
        Check.object(payload["qualities"], "payload.qualities")
      else
        Map.drop(payload, ~w(id label metadata))
      end

    if map_size(source) == 0, do: Check.fail("payload.qualities requires named quality summaries")
    Enum.sort_by(source, &elem(&1, 0))
  end

  defp term(id, raw, config) do
    if String.trim(id) == "", do: Check.fail("payload.qualities contains a blank source id")
    path = "payload.qualities.#{id}"
    summary = Check.object(raw, path)
    Check.converged(summary, path)

    evidence =
      if summary["validation_contract"] == "kyuubiki.summary_tolerance_validation/v1",
        do: validation(summary, path),
        else: domain(summary, path)

    weights = Map.get(config, "weights", %{})

    weight =
      nonnegative(
        Map.get(weights, id, Map.get(weights, evidence["domain"], 1)),
        "config.weights.#{id}"
      )

    weighted = Check.calculate("#{path}.weighted_score", fn -> evidence["score"] * weight end)
    missing = if evidence["domain"] == "validation", do: 0, else: evidence["missing_metric_count"]

    penalty =
      nonnegative(Map.get(config, "missing_metric_penalty", 5), "config.missing_metric_penalty")

    missing_cost = Check.calculate("#{path}.missing_metric_penalty", fn -> missing * penalty end)

    readiness_cost =
      if evidence["ready"],
        do: 0.0,
        else: nonnegative(Map.get(config, "not_ready_penalty", 25), "config.not_ready_penalty")

    contribution =
      Check.calculate("#{path}.contribution", fn -> weighted + missing_cost + readiness_cost end)

    Map.merge(evidence, %{
      "source" => id,
      "weight" => weight,
      "weighted_score" => weighted,
      "missing_metric_penalty" => missing_cost,
      "readiness_penalty" => readiness_cost,
      "contribution" => contribution
    })
  end

  defp domain(summary, path) do
    fields = Enum.filter(Map.keys(summary), &String.ends_with?(&1, "_quality_score"))
    if length(fields) != 1, do: Check.fail("#{path} requires exactly one quality score field")
    [score_field] = fields
    prefix = String.replace_suffix(score_field, "_score", "")

    # Read companion fields from the same domain, never from another suffix match.
    for key <- Map.keys(summary),
        suffix <- @suffixes,
        String.ends_with?(key, "_quality_#{suffix}"),
        key != "#{prefix}_#{suffix}" do
      Check.fail("#{path}.#{key} conflicts with #{score_field}")
    end

    score = nonnegative(summary[score_field], "#{path}.#{score_field}")

    blockers =
      Check.list(
        Map.get(summary, "#{prefix}_blocking_terms", []),
        "#{path}.#{prefix}_blocking_terms"
      )

    ready =
      boolean(Map.get(summary, "#{prefix}_ready", false), "#{path}.#{prefix}_ready") &&
        blockers == []

    declared_grade = Check.text(summary, "#{prefix}_grade", path, "unknown")
    contract = Check.text(summary, "#{prefix}_contract", path)
    fallback_domain = String.replace_suffix(score_field, "_quality_score", "")

    domain =
      if is_binary(contract) && String.starts_with?(contract, "kyuubiki.") &&
           String.ends_with?(contract, "_quality_score/v1"),
         do:
           contract
           |> String.replace_prefix("kyuubiki.", "")
           |> String.replace_suffix("_quality_score/v1", ""),
         else: fallback_domain

    %{
      "domain" => domain,
      "score_field" => score_field,
      "score" => score,
      "ready" => ready,
      "grade" => if(ready, do: declared_grade, else: "block"),
      "missing_metric_count" =>
        Check.unsigned(
          Map.get(summary, "#{prefix}_missing_metric_count", 0),
          "#{path}.#{prefix}_missing_metric_count"
        ),
      "watch_count" =>
        Check.unsigned(
          Map.get(summary, "#{prefix}_watch_count", 0),
          "#{path}.#{prefix}_watch_count"
        ),
      "dominant_term" => summary["#{prefix}_dominant_term"],
      "blocking_terms" => blockers
    }
  end

  defp validation(summary, path) do
    counts =
      for key <- ~w(checked failed missing), into: %{} do
        field = "validation_#{key}_field_count"

        value =
          if Map.has_key?(summary, field), do: Check.unsigned(summary[field], "#{path}.#{field}")

        {key, value}
      end

    passed = boolean(Map.get(summary, "validation_passed", false), "#{path}.validation_passed")

    required =
      boolean(
        Map.get(summary, "validation_fail_on_missing", true),
        "#{path}.validation_fail_on_missing"
      )

    failures =
      Check.list(Map.get(summary, "validation_failures", []), "#{path}.validation_failures")

    missing =
      Check.list(
        Map.get(summary, "validation_missing_fields", []),
        "#{path}.validation_missing_fields"
      )

    ready =
      passed && is_integer(counts["checked"]) && counts["checked"] > 0 && counts["failed"] == 0 &&
        failures == [] && is_integer(counts["missing"]) &&
        (!required || (counts["missing"] == 0 && missing == []))

    relative =
      nonnegative(
        Map.get(summary, "validation_max_relative_error", 0),
        "#{path}.validation_max_relative_error"
      )

    absolute =
      nonnegative(
        Map.get(summary, "validation_max_absolute_error", 0),
        "#{path}.validation_max_absolute_error"
      )

    score = Check.calculate("#{path}.validation_score", fn -> max(relative * 1000, absolute) end)

    blockers =
      Enum.map(failures, fn failure ->
        failure = Check.object(failure, "#{path}.validation_failures[]")

        failure
        |> Map.take(~w(field absolute_error relative_error))
        |> Map.put("status", "tolerance_failed")
      end) ++ Enum.map(missing, &%{"field" => &1, "status" => "missing_validation_field"})

    blockers =
      cond do
        ready ->
          []

        blockers == [] ->
          [
            %{
              "field" => "validation_passed",
              "status" => "invalid_validation_evidence",
              "reason" =>
                "Passing validation requires nonempty comparisons and consistent success counters."
            }
          ]

        true ->
          blockers
      end

    %{
      "domain" => "validation",
      "score_field" => "validation_max_relative_error",
      "score" => score,
      "ready" => ready,
      "grade" => if(ready, do: "pass", else: "block"),
      "missing_metric_count" => counts["missing"] || 0,
      "watch_count" => counts["failed"] || 0,
      "dominant_term" => %{
        "field" => "validation_max_relative_error",
        "status" => if(ready, do: "pass", else: "block"),
        "absolute_error" => summary["validation_max_absolute_error"],
        "relative_error" => summary["validation_max_relative_error"]
      },
      "blocking_terms" => blockers
    }
  end

  defp total_count(terms, key),
    do: Check.unsigned(Enum.sum(Enum.map(terms, & &1[key])), "composite_quality_#{key}")

  defp grade(score, blocked, limit) do
    cond do
      blocked != [] || score > limit -> "block"
      score > limit * 0.7 -> "review"
      score > limit * 0.35 -> "good"
      true -> "excellent"
    end
  end
end
