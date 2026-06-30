defmodule KyuubikiWeb.WorkflowMaterialScoringRuntime do
  @moduledoc false

  def score_material_candidates(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, criteria} <- parse_score_criteria(config),
         candidates when candidates != [] <- candidate_entries(payload),
         {:ok, scored} <- score_candidates(candidates, criteria, config) do
      rankings =
        scored
        |> Enum.sort_by(&{-&1.final_score, -&1.weighted_score, &1.candidate_id})
        |> Enum.map(&score_ranking_summary(&1, config))

      best = List.first(rankings, %{})

      {:ok,
       %{
         "material_score_candidate_count" => length(scored),
         "material_score_feasible_count" => Enum.count(scored, & &1.feasible),
         "material_score_best_candidate_id" => Map.get(best, "candidate_id", ""),
         "material_score_best_score" => Map.get(best, "final_score", 0.0),
         "material_score_criteria" => criteria,
         "material_score_rankings" => rankings
       }}
    else
      [] -> {:error, :missing_material_candidates}
      {:error, _reason} = error -> error
    end
  end

  def score_material_candidates(_payload, _config), do: {:error, :invalid_material_score_payload}

  def plan_material_experiments(payload, config) when is_map(payload) and is_map(config) do
    with rankings when rankings != [] <- ranking_entries(payload),
         planned <- build_experiment_plan(rankings, config) do
      {:ok,
       %{
         "material_experiment_plan_count" => length(planned),
         "material_experiment_primary_candidate_id" =>
           planned |> List.first(%{}) |> Map.get("candidate_id", ""),
         "material_experiment_plan" => planned,
         "material_experiment_policy" => %{
           "top_k" => numeric_config(config, "top_k", 3),
           "include_infeasible" => Map.get(config, "include_infeasible", false),
           "label" => Map.get(config, "label", "material-screening")
         }
       }}
    else
      [] -> {:error, :missing_material_score_rankings}
    end
  end

  def plan_material_experiments(_payload, _config),
    do: {:error, :invalid_material_experiment_plan_payload}

  def analyze_material_experiment_results(payload, config)
      when is_map(payload) and is_map(config) do
    with experiments when experiments != [] <- experiment_result_entries(payload),
         analyzed <- analyze_experiment_results(experiments, config) do
      best = List.first(analyzed, %{})

      {:ok,
       %{
         "material_experiment_result_count" => length(analyzed),
         "material_experiment_validated_count" =>
           Enum.count(analyzed, &(Map.get(&1, "status") == "validated")),
         "material_experiment_best_candidate_id" => Map.get(best, "candidate_id", ""),
         "material_experiment_best_observed_score" => Map.get(best, "observed_score", 0.0),
         "material_experiment_result_rankings" => analyzed,
         "material_experiment_result_policy" => %{
           "observed_score_field" => Map.get(config, "observed_score_field", "observed_score"),
           "expected_score_field" => Map.get(config, "expected_score_field", "expected_score"),
           "pass_field" => Map.get(config, "pass_field", "passed")
         }
       }}
    else
      [] -> {:error, :missing_material_experiment_results}
    end
  end

  def analyze_material_experiment_results(_payload, _config),
    do: {:error, :invalid_material_experiment_result_payload}

  def decide_material_iteration(payload, config) when is_map(payload) and is_map(config) do
    rankings = Map.get(payload, "material_experiment_result_rankings", [])

    with true <- is_list(rankings) and rankings != [] do
      best = List.first(rankings, %{})
      target_score = numeric_config(config, "target_score", 0.7)
      min_validated = config |> numeric_config("min_validated", 1) |> round()
      current_round = current_round(payload, config)
      max_rounds = config |> numeric_config("max_rounds", 5) |> round()
      validated_count = numeric_field(payload, "material_experiment_validated_count") |> round()
      best_score = numeric_field(payload, "material_experiment_best_observed_score")

      decision =
        iteration_decision(
          best_score,
          target_score,
          validated_count,
          min_validated,
          current_round,
          max_rounds
        )

      {:ok,
       %{
         "material_iteration_decision" => decision.decision,
         "material_iteration_next_action" => decision.next_action,
         "material_iteration_reason" => decision.reason,
         "material_iteration_best_candidate_id" => Map.get(best, "candidate_id", ""),
         "material_iteration_best_observed_score" => best_score,
         "material_iteration_target_score" => target_score,
         "material_iteration_validated_count" => validated_count,
         "material_iteration_min_validated" => min_validated,
         "material_iteration_current_round" => current_round,
         "material_iteration_max_rounds" => max_rounds
       }}
    else
      _ -> {:error, :missing_material_iteration_rankings}
    end
  end

  def decide_material_iteration(_payload, _config),
    do: {:error, :invalid_material_iteration_payload}

  def prepare_material_next_round_request(payload, config)
      when is_map(payload) and is_map(config) do
    action = Map.get(payload, "material_iteration_next_action", "run_more_experiments")
    decision = Map.get(payload, "material_iteration_decision", "continue")

    current_round =
      payload |> numeric_field("material_iteration_current_round") |> round() |> max(1)

    next_round = current_round + 1

    {:ok,
     %{
       "material_next_round_enabled" => decision != "stop",
       "material_next_round_action" => action,
       "material_next_round_index" => next_round,
       "material_next_round_requested_candidate_count" =>
         config |> numeric_config("requested_candidate_count", 3) |> round() |> max(1),
       "material_next_round_seed_candidate_id" =>
         Map.get(payload, "material_iteration_best_candidate_id", ""),
       "material_next_round_target_score" =>
         Map.get(
           payload,
           "material_iteration_target_score",
           numeric_config(config, "target_score", 0.7)
         ),
       "material_next_round_constraints" => Map.get(config, "constraints", %{}),
       "material_next_round_stop_reason" => stop_reason(decision, payload),
       "material_next_round_source_decision" => decision
     }}
  end

  def prepare_material_next_round_request(_payload, _config),
    do: {:error, :invalid_material_next_round_request_payload}

  defp parse_score_criteria(config) do
    case Map.get(config, "criteria") || Map.get(config, "objectives") do
      entries when is_list(entries) and entries != [] ->
        parse_score_criteria_entries(entries)

      _ ->
        {:error, :missing_material_score_criteria}
    end
  end

  defp parse_score_criteria_entries(entries) do
    entries
    |> Enum.reduce_while({:ok, []}, fn entry, {:ok, acc} ->
      field = Map.get(entry, "field")
      goal = Map.get(entry, "goal", "min")
      weight = Map.get(entry, "weight", 1.0)

      if is_binary(field) and goal in ["min", "max"] and is_number(weight) and weight > 0.0 do
        {:cont, {:ok, [%{"field" => field, "goal" => goal, "weight" => weight * 1.0} | acc]}}
      else
        {:halt, {:error, :invalid_material_score_criterion}}
      end
    end)
    |> case do
      {:ok, parsed} -> {:ok, Enum.reverse(parsed)}
      error -> error
    end
  end

  defp score_candidates(candidates, criteria, config) do
    with {:ok, metrics_by_candidate} <- collect_candidate_metrics(candidates, criteria) do
      ranges = criterion_ranges(metrics_by_candidate, criteria)
      total_weight = Enum.reduce(criteria, 0.0, &(&1["weight"] + &2))
      infeasible_penalty = numeric_config(config, "infeasible_penalty", 1.0)

      scored =
        Enum.map(candidates, fn {candidate_id, summary} ->
          metrics = Map.fetch!(metrics_by_candidate, candidate_id)
          breakdown = score_breakdown(metrics, ranges, criteria)

          weighted_score =
            Enum.reduce(breakdown, 0.0, &(&1["weighted_score"] + &2)) / total_weight

          feasible = feasible?(summary, config)

          %{
            candidate_id: candidate_id,
            summary: summary,
            metrics: metrics,
            feasible: feasible,
            breakdown: breakdown,
            weighted_score: weighted_score,
            final_score:
              if(feasible, do: weighted_score, else: weighted_score - infeasible_penalty)
          }
        end)

      {:ok, scored}
    end
  end

  defp collect_candidate_metrics(candidates, criteria) do
    candidates
    |> Enum.reduce_while({:ok, %{}}, fn {candidate_id, summary}, {:ok, acc} ->
      metrics =
        criteria
        |> Enum.map(fn %{"field" => field} -> {field, lookup_number(summary, field)} end)

      if Enum.all?(metrics, fn {_field, value} -> is_number(value) end) do
        {:cont, {:ok, Map.put(acc, candidate_id, Map.new(metrics))}}
      else
        {:halt, {:error, :missing_material_score_metric}}
      end
    end)
  end

  defp criterion_ranges(metrics_by_candidate, criteria) do
    Map.new(criteria, fn %{"field" => field} ->
      values = Enum.map(metrics_by_candidate, fn {_candidate_id, metrics} -> metrics[field] end)
      {field, %{min: Enum.min(values), max: Enum.max(values)}}
    end)
  end

  defp score_breakdown(metrics, ranges, criteria) do
    Enum.map(criteria, fn %{"field" => field, "goal" => goal, "weight" => weight} ->
      normalized = normalize_score(metrics[field], Map.fetch!(ranges, field), goal)

      %{
        "field" => field,
        "goal" => goal,
        "weight" => weight,
        "actual" => metrics[field],
        "normalized_score" => normalized,
        "weighted_score" => normalized * weight
      }
    end)
  end

  defp normalize_score(_value, %{min: same, max: same}, _goal), do: 1.0
  defp normalize_score(value, %{min: min, max: max}, "max"), do: (value - min) / (max - min)
  defp normalize_score(value, %{min: min, max: max}, _goal), do: (max - value) / (max - min)

  defp score_ranking_summary(scored, config) do
    summary = %{
      "candidate_id" => scored.candidate_id,
      "feasible" => scored.feasible,
      "weighted_score" => scored.weighted_score,
      "final_score" => scored.final_score,
      "metrics" => scored.metrics,
      "criteria_breakdown" => scored.breakdown
    }

    if Map.get(config, "include_candidate_summary", false) do
      Map.put(summary, "summary", scored.summary)
    else
      summary
    end
  end

  defp ranking_entries(%{"material_score_rankings" => rankings}) when is_list(rankings),
    do: Enum.filter(rankings, &is_map/1)

  defp ranking_entries(%{"rankings" => rankings}) when is_list(rankings),
    do: Enum.filter(rankings, &is_map/1)

  defp ranking_entries(_payload), do: []

  defp experiment_result_entries(%{"results" => results}) when is_list(results),
    do: Enum.filter(results, &is_map/1)

  defp experiment_result_entries(%{"material_experiment_results" => results})
       when is_list(results),
       do: Enum.filter(results, &is_map/1)

  defp experiment_result_entries(%{"material_experiment_plan" => plan}) when is_list(plan),
    do: Enum.filter(plan, &is_map/1)

  defp experiment_result_entries(_payload), do: []

  defp build_experiment_plan(rankings, config) do
    top_k = config |> numeric_config("top_k", 3) |> round() |> max(1)
    include_infeasible = Map.get(config, "include_infeasible", false)

    rankings
    |> Enum.filter(&(include_infeasible or Map.get(&1, "feasible", true)))
    |> Enum.sort_by(&{-numeric_field(&1, "final_score"), Map.get(&1, "candidate_id", "")})
    |> Enum.take(top_k)
    |> Enum.with_index(1)
    |> Enum.map(fn {ranking, index} ->
      %{
        "experiment_id" => "#{Map.get(config, "label", "material-screening")}-#{index}",
        "candidate_id" => Map.get(ranking, "candidate_id", ""),
        "priority" => index,
        "reason" => experiment_reason(ranking, index),
        "expected_score" => numeric_field(ranking, "final_score"),
        "feasible" => Map.get(ranking, "feasible", true),
        "metrics" => Map.get(ranking, "metrics", %{}),
        "criteria_breakdown" => Map.get(ranking, "criteria_breakdown", [])
      }
    end)
  end

  defp analyze_experiment_results(experiments, config) do
    observed_field = Map.get(config, "observed_score_field", "observed_score")
    expected_field = Map.get(config, "expected_score_field", "expected_score")
    pass_field = Map.get(config, "pass_field", "passed")

    experiments
    |> Enum.map(fn experiment ->
      observed = numeric_field(experiment, observed_field)
      expected = numeric_field(experiment, expected_field)

      experiment
      |> Map.take(["experiment_id", "candidate_id", "priority", "metrics"])
      |> Map.merge(%{
        "observed_score" => observed,
        "expected_score" => expected,
        "score_error" => observed - expected,
        "score_error_abs" => abs(observed - expected),
        "status" => experiment_status(experiment, pass_field),
        "result_note" => experiment_result_note(observed, expected)
      })
    end)
    |> Enum.sort_by(
      &{Map.get(&1, "status") != "validated", -numeric_field(&1, "observed_score"),
       numeric_field(&1, "priority")}
    )
  end

  defp experiment_status(experiment, pass_field) do
    case Map.get(experiment, pass_field, Map.get(experiment, "feasible", true)) do
      value when value in [true, "pass", "validated", "ok"] -> "validated"
      value when value in [false, "fail", "invalid", "blocked"] -> "rejected"
      value when is_number(value) -> if(value != 0, do: "validated", else: "rejected")
      _ -> "unknown"
    end
  end

  defp experiment_result_note(observed, expected) when observed >= expected,
    do: "observed score met or exceeded expected score"

  defp experiment_result_note(_observed, _expected),
    do: "observed score fell below expected score"

  defp current_round(payload, config) do
    config_round = numeric_config(config, "current_round", 0.0)

    if config_round > 0.0 do
      round(config_round)
    else
      payload |> numeric_field("material_iteration_current_round") |> round() |> max(1)
    end
  end

  defp iteration_decision(
         best_score,
         target_score,
         validated_count,
         min_validated,
         current_round,
         max_rounds
       ) do
    cond do
      validated_count <= 0 ->
        %{
          decision: "replan",
          next_action: "generate_new_candidates",
          reason: "no validated material experiment results are available"
        }

      best_score >= target_score and validated_count >= min_validated ->
        %{
          decision: "stop",
          next_action: "accept_candidate",
          reason: "best observed score reached the target score"
        }

      current_round >= max_rounds ->
        %{
          decision: "stop",
          next_action: "review_manually",
          reason: "maximum material exploration rounds reached before target score"
        }

      true ->
        %{
          decision: "continue",
          next_action: "run_more_experiments",
          reason: "target score has not been reached yet"
        }
    end
  end

  defp stop_reason("stop", payload), do: Map.get(payload, "material_iteration_reason", "")
  defp stop_reason(_decision, _payload), do: ""

  defp experiment_reason(ranking, 1) do
    "primary candidate from weighted material score #{format_score(ranking)}"
  end

  defp experiment_reason(ranking, _index) do
    "backup candidate from weighted material score #{format_score(ranking)}"
  end

  defp format_score(ranking),
    do: :erlang.float_to_binary(numeric_field(ranking, "final_score"), decimals: 4)

  defp candidate_entries(%{"rows" => rows}) when is_list(rows) do
    rows
    |> Enum.with_index()
    |> Enum.filter(fn {row, _index} -> is_map(row) end)
    |> Enum.map(fn {row, index} -> {Map.get(row, "candidate_id", "candidate_#{index}"), row} end)
  end

  defp candidate_entries(%{"candidates" => candidates}) when is_map(candidates),
    do: object_entries(candidates)

  defp candidate_entries(payload), do: object_entries(payload)

  defp object_entries(object) when is_map(object) do
    object
    |> Enum.filter(fn {_id, value} -> is_map(value) end)
    |> Enum.map(fn {id, summary} -> {id, summary} end)
  end

  defp feasible?(summary, config) do
    field = Map.get(config, "feasible_field", "material_status")

    case Map.get(summary, field) do
      value when value in [true, "pass", "feasible"] -> true
      value when value in [false, "fail", "infeasible"] -> false
      value when is_number(value) -> value != 0
      _ -> Map.get(summary, "objective_feasible", true)
    end
  end

  defp numeric_config(config, key, default) do
    case Map.get(config, key, default) do
      value when is_number(value) -> value * 1.0
      _ -> default
    end
  end

  defp numeric_field(payload, field) do
    case Map.get(payload, field) do
      value when is_number(value) -> value * 1.0
      _ -> 0.0
    end
  end

  defp lookup_number(payload, field) when is_map(payload) do
    case get_in(payload, String.split(field, ".")) ||
           get_in(payload, ["summary" | String.split(field, ".")]) do
      value when is_number(value) -> value * 1.0
      _ -> nil
    end
  end
end
