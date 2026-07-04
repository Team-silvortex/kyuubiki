defmodule KyuubikiWeb.WorkflowMaterialRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowMaterialFatigueRuntime
  alias KyuubikiWeb.WorkflowMaterialCardRuntime
  alias KyuubikiWeb.WorkflowMaterialCardBatchRuntime
  alias KyuubikiWeb.WorkflowMaterialCardCandidateRuntime
  alias KyuubikiWeb.WorkflowMaterialEnvelopeRuntime
  alias KyuubikiWeb.WorkflowMaterialCardScreeningRuntime
  alias KyuubikiWeb.WorkflowMaterialScoringRuntime
  alias KyuubikiWeb.WorkflowMaterialStateRuntime
  alias KyuubikiWeb.WorkflowMaterialThermalShockRuntime

  def evaluate_material_margins(payload, config) when is_map(payload) and is_map(config) do
    limits = Map.get(config, "limits")

    with true <- is_map(limits) and map_size(limits) > 0,
         {:ok, evaluations} <- evaluate_limits(payload, limits) do
      prefix = Map.get(config, "output_prefix", "material")
      critical = Enum.max_by(evaluations, & &1.failure_index)
      violation_count = Enum.count(evaluations, &(&1.failure_index > 1.0))

      summary =
        evaluations
        |> Enum.reduce(%{}, &put_evaluation(&2, &1, prefix))
        |> Map.merge(%{
          "#{prefix}_constraint_count" => length(evaluations),
          "#{prefix}_violation_count" => violation_count,
          "#{prefix}_failure_index" => critical.failure_index,
          "#{prefix}_safety_factor" => 1.0 / critical.failure_index,
          "#{prefix}_critical_metric" => critical.field,
          "#{prefix}_critical_actual" => critical.actual,
          "#{prefix}_critical_limit" => critical.limit,
          "#{prefix}_status" => if(violation_count == 0, do: "pass", else: "fail")
        })

      {:ok, summary}
    else
      false -> {:error, :missing_material_limits}
      {:error, _reason} = error -> error
    end
  end

  def evaluate_material_margins(_payload, _config), do: {:error, :invalid_material_margin_payload}

  def rank_material_candidates(payload, config) when is_map(payload) and is_map(config) do
    prefix = Map.get(config, "margin_prefix", "material")

    with candidates when candidates != [] <- candidate_entries(payload) do
      rankings =
        candidates
        |> Enum.map(&rank_candidate(&1, prefix))
        |> Enum.sort_by(&{not &1.feasible, -&1.safety_factor, &1.failure_index, &1.candidate_id})

      best = hd(rankings)
      feasible_count = Enum.count(rankings, & &1.feasible)

      {:ok,
       %{
         "material_candidate_count" => length(rankings),
         "material_feasible_count" => feasible_count,
         "material_best_candidate_id" => best.candidate_id,
         "material_best_candidate_feasible" => best.feasible,
         "material_best_safety_factor" => best.safety_factor,
         "material_best_failure_index" => best.failure_index,
         "material_failure_reasons" => failure_reasons(rankings),
         "material_rankings" => Enum.map(rankings, &ranking_summary/1)
       }
       |> maybe_put_best_summary(best, config)}
    else
      [] -> {:error, :missing_material_candidates}
    end
  end

  def rank_material_candidates(_payload, _config), do: {:error, :invalid_material_candidates}

  defdelegate compose_material_study_envelope(payload, config),
    to: WorkflowMaterialEnvelopeRuntime

  defdelegate validate_material_card(payload, config), to: WorkflowMaterialCardRuntime
  defdelegate validate_material_card_batch(payload, config), to: WorkflowMaterialCardBatchRuntime

  defdelegate build_material_card_candidate_summaries(payload, config),
    to: WorkflowMaterialCardCandidateRuntime

  defdelegate explain_material_card_screening(payload, config),
    to: WorkflowMaterialCardScreeningRuntime

  defdelegate estimate_material_fatigue_life(payload, config), to: WorkflowMaterialFatigueRuntime

  defdelegate evaluate_material_thermal_shock(payload, config),
    to: WorkflowMaterialThermalShockRuntime

  defdelegate score_material_candidates(payload, config), to: WorkflowMaterialScoringRuntime
  defdelegate plan_material_experiments(payload, config), to: WorkflowMaterialScoringRuntime

  defdelegate analyze_material_experiment_results(payload, config),
    to: WorkflowMaterialScoringRuntime

  defdelegate decide_material_iteration(payload, config), to: WorkflowMaterialScoringRuntime

  defdelegate prepare_material_next_round_request(payload, config),
    to: WorkflowMaterialScoringRuntime

  defdelegate build_material_exploration_snapshot(payload, config),
    to: WorkflowMaterialStateRuntime

  def extract_material_pareto_frontier(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, objectives} <- parse_objectives(config),
         candidates when candidates != [] <- pareto_entries(payload),
         {:ok, evaluated} <- evaluate_pareto(candidates, objectives, config) do
      include_infeasible = Map.get(config, "include_infeasible", false)
      {frontier, dominated} = split_pareto(evaluated, objectives, include_infeasible)
      frontier = Enum.sort_by(frontier, &{-&1["objective_score"], &1["candidate_id"]})

      {:ok,
       %{
         "material_pareto_candidate_count" => length(evaluated),
         "material_pareto_feasible_count" => Enum.count(evaluated, & &1.feasible),
         "material_pareto_frontier_count" => length(frontier),
         "material_pareto_best_candidate_id" =>
           frontier |> List.first(%{}) |> Map.get("candidate_id", ""),
         "material_pareto_objectives" => objectives,
         "material_pareto_frontier" => frontier,
         "material_pareto_dominated" => dominated
       }}
    else
      [] -> {:error, :missing_material_candidates}
      {:error, _reason} = error -> error
    end
  end

  def extract_material_pareto_frontier(_payload, _config),
    do: {:error, :invalid_material_pareto_payload}

  defp evaluate_limits(payload, limits) do
    evaluations =
      limits
      |> Enum.flat_map(fn {field, spec} ->
        with {:ok, limit, direction} <- parse_limit(spec),
             actual when is_number(actual) <- lookup_number(payload, field),
             true <- limit > 0.0 do
          failure_index =
            case direction do
              "min" -> limit / actual
              "abs" -> abs(actual) / limit
              _ -> actual / limit
            end

          [%{field: field, actual: actual, limit: limit, failure_index: failure_index}]
        else
          _ -> []
        end
      end)

    if evaluations == [],
      do: {:error, :missing_matching_material_limits},
      else: {:ok, evaluations}
  end

  defp parse_limit(spec) when is_number(spec), do: {:ok, spec * 1.0, "max"}

  defp parse_limit(spec) when is_map(spec) do
    limit = Map.get(spec, "limit") || Map.get(spec, "max") || Map.get(spec, "min")
    direction = Map.get(spec, "direction") || Map.get(spec, "kind") || default_direction(spec)
    if is_number(limit), do: {:ok, limit * 1.0, direction}, else: :error
  end

  defp parse_limit(_spec), do: :error

  defp default_direction(%{"min" => _}), do: "min"
  defp default_direction(_spec), do: "max"

  defp put_evaluation(summary, evaluation, prefix) do
    Map.merge(summary, %{
      "#{prefix}_#{evaluation.field}_actual" => evaluation.actual,
      "#{prefix}_#{evaluation.field}_limit" => evaluation.limit,
      "#{prefix}_#{evaluation.field}_failure_index" => evaluation.failure_index,
      "#{prefix}_#{evaluation.field}_safety_factor" => 1.0 / evaluation.failure_index
    })
  end

  defp candidate_entries(%{"candidates" => candidates}) when is_map(candidates),
    do: object_entries(candidates)

  defp candidate_entries(payload), do: object_entries(payload)

  defp object_entries(object) when is_map(object) do
    object
    |> Enum.filter(fn {_id, value} -> is_map(value) end)
    |> Enum.map(fn {id, summary} -> {id, summary} end)
  end

  defp rank_candidate({candidate_id, summary}, prefix) do
    failure_index = lookup_number(summary, "#{prefix}_failure_index") || :infinity
    safety_factor = lookup_number(summary, "#{prefix}_safety_factor") || 0.0
    violation_count = lookup_number(summary, "#{prefix}_violation_count") || 1.0
    status = Map.get(summary, "#{prefix}_status", "unknown")
    critical_metric = Map.get(summary, "#{prefix}_critical_metric", "unknown")

    %{
      candidate_id: candidate_id,
      feasible: status == "pass" and violation_count == 0.0 and failure_index <= 1.0,
      safety_factor: safety_factor,
      failure_index: failure_index,
      critical_metric: critical_metric,
      summary: summary
    }
  end

  defp failure_reasons(rankings) do
    rankings
    |> Enum.reject(& &1.feasible)
    |> Enum.reduce(%{}, fn ranking, acc ->
      Map.update(acc, ranking.critical_metric, 1, &(&1 + 1))
    end)
  end

  defp ranking_summary(ranking) do
    %{
      "candidate_id" => ranking.candidate_id,
      "feasible" => ranking.feasible,
      "safety_factor" => ranking.safety_factor,
      "failure_index" => ranking.failure_index,
      "critical_metric" => ranking.critical_metric
    }
  end

  defp maybe_put_best_summary(output, _best, %{"include_best_summary" => false}), do: output

  defp maybe_put_best_summary(output, best, _config),
    do: Map.put(output, "material_best_summary", best.summary)

  defp parse_objectives(%{"objectives" => objectives})
       when is_list(objectives) and objectives != [] do
    objectives
    |> Enum.reduce_while({:ok, []}, fn objective, {:ok, acc} ->
      field = Map.get(objective, "field")
      goal = Map.get(objective, "goal", "min")

      if is_binary(field) and goal in ["min", "max"] do
        {:cont,
         {:ok,
          [
            %{"field" => field, "goal" => goal, "weight" => Map.get(objective, "weight", 1.0)}
            | acc
          ]}}
      else
        {:halt, {:error, :invalid_material_pareto_objective}}
      end
    end)
    |> case do
      {:ok, parsed} -> {:ok, Enum.reverse(parsed)}
      error -> error
    end
  end

  defp parse_objectives(_config), do: {:error, :missing_material_pareto_objectives}

  defp pareto_entries(%{"rows" => rows}) when is_list(rows) do
    rows
    |> Enum.with_index()
    |> Enum.filter(fn {row, _index} -> is_map(row) end)
    |> Enum.map(fn {row, index} -> {Map.get(row, "candidate_id", "candidate_#{index}"), row} end)
  end

  defp pareto_entries(%{"candidates" => candidates}) when is_map(candidates),
    do: object_entries(candidates)

  defp pareto_entries(payload), do: object_entries(payload)

  defp evaluate_pareto(candidates, objectives, config) do
    candidates
    |> Enum.reduce_while({:ok, []}, fn {candidate_id, summary}, {:ok, acc} ->
      metrics =
        objectives
        |> Enum.map(fn %{"field" => field} -> {field, lookup_number(summary, field)} end)

      if Enum.all?(metrics, fn {_field, value} -> is_number(value) end) do
        score =
          Enum.reduce(objectives, 0.0, fn objective, total ->
            value = lookup_number(summary, objective["field"])
            signed = if objective["goal"] == "max", do: value, else: -value
            total + signed * objective["weight"]
          end)

        {:cont,
         {:ok,
          [
            %{
              candidate_id: candidate_id,
              summary: summary,
              metrics: Map.new(metrics),
              feasible: feasible?(summary, config),
              objective_score: score
            }
            | acc
          ]}}
      else
        {:halt, {:error, :missing_material_pareto_metric}}
      end
    end)
    |> case do
      {:ok, evaluated} -> {:ok, Enum.reverse(evaluated)}
      error -> error
    end
  end

  defp split_pareto(evaluated, objectives, include_infeasible) do
    Enum.reduce(evaluated, {[], []}, fn candidate, {frontier, dominated} ->
      cond do
        not include_infeasible and not candidate.feasible ->
          {frontier, [dominated_summary(candidate, "infeasible") | dominated]}

        dominator =
            Enum.find(evaluated, &dominates?(&1, candidate, objectives, include_infeasible)) ->
          {frontier, [dominated_summary(candidate, dominator.candidate_id) | dominated]}

        true ->
          {[frontier_summary(candidate) | frontier], dominated}
      end
    end)
  end

  defp dominates?(left, right, _objectives, _include_infeasible) when left == right, do: false

  defp dominates?(left, right, _objectives, false) when not left.feasible or not right.feasible,
    do: left.feasible and not right.feasible

  defp dominates?(left, right, objectives, _include_infeasible) do
    Enum.all?(objectives, &no_worse?(left, right, &1)) and
      Enum.any?(objectives, &strictly_better?(left, right, &1))
  end

  defp no_worse?(left, right, %{"field" => field, "goal" => "max"}),
    do: left.metrics[field] >= right.metrics[field]

  defp no_worse?(left, right, %{"field" => field}),
    do: left.metrics[field] <= right.metrics[field]

  defp strictly_better?(left, right, %{"field" => field, "goal" => "max"}),
    do: left.metrics[field] > right.metrics[field]

  defp strictly_better?(left, right, %{"field" => field}),
    do: left.metrics[field] < right.metrics[field]

  defp frontier_summary(candidate),
    do: %{
      "candidate_id" => candidate.candidate_id,
      "feasible" => candidate.feasible,
      "objective_score" => candidate.objective_score,
      "metrics" => candidate.metrics,
      "summary" => candidate.summary
    }

  defp dominated_summary(candidate, by),
    do: %{
      "candidate_id" => candidate.candidate_id,
      "feasible" => candidate.feasible,
      "objective_score" => candidate.objective_score,
      "dominated_by" => by,
      "metrics" => candidate.metrics
    }

  defp feasible?(summary, config) do
    field = Map.get(config, "feasible_field", "material_status")

    case Map.get(summary, field) do
      value when value in [true, "pass", "feasible"] -> true
      value when value in [false, "fail", "infeasible"] -> false
      value when is_number(value) -> value != 0
      _ -> Map.get(summary, "objective_feasible", true)
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
