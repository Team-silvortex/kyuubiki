defmodule KyuubikiWeb.WorkflowQualityObjectiveRuntime do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.WorkflowParameterSweepRuntime

  def compose_quality_objective(payload, config) when is_map(payload) and is_map(config) do
    entries = quality_entries(payload)

    if entries == [] do
      {:error, :invalid_quality_objective_payload}
    else
      missing_penalty = number_config(config, "missing_metric_penalty", 5.0)
      not_ready_penalty = number_config(config, "not_ready_penalty", 25.0)
      max_ready_score = number_config(config, "max_ready_score", 12.0)

      terms =
        Enum.map(entries, fn {source_id, summary} ->
          quality_term(source_id, summary, config, missing_penalty, not_ready_penalty)
        end)

      score = Enum.sum(Enum.map(terms, & &1["contribution"]))
      blocked_count = Enum.count(terms, &(&1["ready"] == false))
      missing_count = terms |> Enum.map(& &1["missing_metric_count"]) |> Enum.sum()
      grade = composite_grade(score, blocked_count, max_ready_score)

      {:ok,
       %{
         "composite_quality_contract" => "kyuubiki.composite_quality_objective/v1",
         "composite_quality_score" => score,
         "composite_quality_grade" => grade,
         "composite_quality_ready" => grade != "block",
         "composite_quality_term_count" => length(terms),
         "composite_quality_missing_metric_count" => missing_count,
         "composite_quality_blocked_term_count" => blocked_count,
         "composite_quality_max_ready_score" => max_ready_score,
         "composite_quality_terms" => terms,
         "composite_quality_summary" =>
           "Composite quality #{grade}: score=#{Float.round(score, 4)}, blocked_terms=#{blocked_count}, missing_metrics=#{missing_count}."
       }}
    end
  end

  def compose_quality_objective(_payload, _config),
    do: {:error, :invalid_quality_objective_payload}

  def rank_quality_candidates(payload, config) when is_map(payload) and is_map(config) do
    candidates = candidate_entries(payload)
    objective_config = Map.get(config, "objective", config)

    ranking =
      candidates
      |> Enum.flat_map(fn {candidate_id, candidate_payload} ->
        case compose_quality_objective(candidate_payload, objective_config) do
          {:ok, objective} -> [candidate_rank(candidate_id, candidate_payload, objective)]
          {:error, _reason} -> []
        end
      end)
      |> Enum.sort_by(&{if(&1["ready"], do: 0, else: 1), &1["score"], &1["candidate_id"]})
      |> Enum.with_index(1)
      |> Enum.map(fn {entry, rank} -> Map.put(entry, "rank", rank) end)

    case ranking do
      [] ->
        {:error, :invalid_quality_candidate_payload}

      [best | _] ->
        {:ok,
         %{
           "quality_candidate_ranking_contract" => "kyuubiki.quality_candidate_ranking/v1",
           "candidate_count" => length(ranking),
           "ready_candidate_count" => Enum.count(ranking, & &1["ready"]),
           "best_candidate_id" => best["candidate_id"],
           "best_candidate_ready" => best["ready"],
           "best_candidate_score" => best["score"],
           "ranking" => ranking,
           "ranking_summary" =>
             "Best quality candidate #{best["candidate_id"]}: score=#{best["score"]}, ready=#{best["ready"]}."
         }}
    end
  end

  def rank_quality_candidates(_payload, _config),
    do: {:error, :invalid_quality_candidate_payload}

  def prepare_quality_next_round_request(payload, config)
      when is_map(payload) and is_map(config) do
    with %{} = selected <- selected_candidate(payload) do
      score = number(selected["score"], payload["best_candidate_score"], 0.0)
      ready = boolean(selected["ready"], payload["best_candidate_ready"], false)
      target_score = number_config(config, "target_score", 3.0)
      action = next_round_action(ready, score, target_score, config)

      {:ok,
       %{
         "quality_next_round_contract" => "kyuubiki.quality_next_round_request/v1",
         "action" => action,
         "selected_candidate_id" =>
           Map.get(selected, "candidate_id", Map.get(payload, "best_candidate_id")),
         "selected_candidate_score" => score,
         "selected_candidate_ready" => ready,
         "target_score" => target_score,
         "source_ranking_contract" => Map.get(payload, "quality_candidate_ranking_contract"),
         "request_payload" => %{
           "seed_candidate_id" =>
             Map.get(selected, "candidate_id", Map.get(payload, "best_candidate_id")),
           "seed_objective" => Map.get(selected, "objective"),
           "constraints" => Map.get(config, "constraints", %{}),
           "search_space" => Map.get(config, "search_space", %{}),
           "max_candidates" => number_config(config, "max_candidates", 8)
         },
         "next_round_summary" =>
           "Quality exploration #{action}: selected=#{Map.get(selected, "candidate_id")}, score=#{score}, target=#{target_score}."
       }}
    else
      _ -> {:error, :invalid_quality_next_round_payload}
    end
  end

  def prepare_quality_next_round_request(_payload, _config),
    do: {:error, :invalid_quality_next_round_payload}

  def build_quality_parameter_sweep_plan(payload, config)
      when is_map(payload) and is_map(config) do
    request = Map.get(payload, "request_payload", %{})
    search_space = Map.get(request, "search_space", Map.get(config, "search_space", %{}))
    base = Map.get(config, "base", Map.get(payload, "base", Map.get(request, "base", %{})))
    axes = search_space_axes(search_space, config)

    cond do
      Map.get(payload, "action") == "stop" ->
        {:ok,
         %{
           "quality_parameter_sweep_plan_contract" => "kyuubiki.quality_parameter_sweep_plan/v1",
           "sweep_enabled" => false,
           "sweep_action" => "stop",
           "case_count_estimate" => 0,
           "axes" => [],
           "base" => base,
           "plan_summary" => "Quality exploration stopped; no parameter sweep was planned."
         }}

      axes == [] ->
        {:error, :invalid_quality_parameter_sweep_search_space}

      true ->
        {:ok,
         %{
           "quality_parameter_sweep_plan_contract" => "kyuubiki.quality_parameter_sweep_plan/v1",
           "sweep_enabled" => true,
           "sweep_action" => Map.get(payload, "action", "continue"),
           "source_candidate_id" => Map.get(payload, "selected_candidate_id"),
           "target_score" => Map.get(payload, "target_score"),
           "id_prefix" => Map.get(config, "id_prefix", "quality_round"),
           "max_cases" => number(config["max_cases"], request["max_candidates"], 64),
           "case_count_estimate" => estimate_case_count(axes),
           "axes" => axes,
           "base" => base,
           "plan_summary" =>
             "Quality parameter sweep planned with #{length(axes)} axis/axes and #{estimate_case_count(axes)} estimated cases."
         }}
    end
  end

  def build_quality_parameter_sweep_plan(_payload, _config),
    do: {:error, :invalid_quality_parameter_sweep_payload}

  def materialize_quality_sweep_expansion(payload, config)
      when is_map(payload) and is_map(config) do
    if Map.get(payload, "sweep_enabled") == false do
      {:ok,
       %{
         "quality_sweep_expansion_contract" => "kyuubiki.quality_sweep_expansion/v1",
         "expansion_enabled" => false,
         "reason" => Map.get(payload, "sweep_action", "stopped"),
         "payload" => nil,
         "config" => nil
       }}
    else
      axes = Map.get(payload, "axes", [])

      if axes == [] do
        {:error, :invalid_quality_sweep_expansion_plan}
      else
        id_prefix = Map.get(config, "id_prefix", Map.get(payload, "id_prefix", "quality_round"))
        max_cases = number(config["max_cases"], payload["max_cases"], 64)

        {:ok,
         %{
           "quality_sweep_expansion_contract" => "kyuubiki.quality_sweep_expansion/v1",
           "expansion_enabled" => true,
           "source_plan_contract" => Map.get(payload, "quality_parameter_sweep_plan_contract"),
           "source_candidate_id" => Map.get(payload, "source_candidate_id"),
           "case_count_estimate" => Map.get(payload, "case_count_estimate"),
           "payload" => %{"base" => Map.get(payload, "base", %{}), "axes" => axes},
           "config" => %{"id_prefix" => id_prefix, "max_cases" => max_cases},
           "expansion_summary" =>
             "Materialized quality sweep expansion with #{length(axes)} axis/axes."
         }}
      end
    end
  end

  def materialize_quality_sweep_expansion(_payload, _config),
    do: {:error, :invalid_quality_sweep_expansion_payload}

  def compose_quality_execution_batch(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, operator_id} <- required_string(config, "operator_id"),
         {:ok, cases} <- normalize_execution_cases(payload, config),
         {:ok, tasks} <- build_execution_tasks(cases, operator_id, config) do
      {:ok,
       %{
         "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
         "operator_id" => operator_id,
         "task_count" => length(tasks),
         "agent_rpc_method" => OperatorTaskIR.agent_rpc_method(),
         "tasks" => tasks,
         "case_index" => Enum.map(tasks, &Map.take(&1, ["case_id", "task_id", "task_digest"])),
         "batch_summary" =>
           "Prepared #{length(tasks)} quality execution task(s) for #{operator_id}."
       }}
    end
  end

  def compose_quality_execution_batch(_payload, _config),
    do: {:error, :invalid_quality_execution_batch_payload}

  defp quality_entries(%{"qualities" => qualities}) when is_map(qualities),
    do: Enum.filter(qualities, fn {_id, value} -> is_map(value) end)

  defp quality_entries(payload), do: Enum.filter(payload, fn {_id, value} -> is_map(value) end)

  defp candidate_entries(%{"candidates" => candidates}) when is_map(candidates),
    do: Enum.filter(candidates, fn {_id, value} -> is_map(value) end)

  defp candidate_entries(%{"candidates" => candidates}) when is_list(candidates) do
    candidates
    |> Enum.filter(&is_map/1)
    |> Enum.with_index(1)
    |> Enum.map(fn {candidate, index} ->
      {Map.get(candidate, "id", "candidate_#{index}"), candidate}
    end)
  end

  defp candidate_entries(payload), do: Enum.filter(payload, fn {_id, value} -> is_map(value) end)

  defp candidate_rank(candidate_id, candidate_payload, objective) do
    %{
      "candidate_id" => candidate_id,
      "candidate_label" => Map.get(candidate_payload, "label", candidate_id),
      "score" => objective["composite_quality_score"],
      "grade" => objective["composite_quality_grade"],
      "ready" => objective["composite_quality_ready"],
      "term_count" => objective["composite_quality_term_count"],
      "missing_metric_count" => objective["composite_quality_missing_metric_count"],
      "blocked_term_count" => objective["composite_quality_blocked_term_count"],
      "objective" => objective
    }
  end

  defp normalize_execution_cases(
         %{"quality_sweep_expansion_contract" => _contract} = payload,
         config
       ) do
    with {:ok, expanded} <- WorkflowParameterSweepRuntime.expand_parameter_sweep(payload, config),
         cases when is_list(cases) and cases != [] <- Map.get(expanded, "cases") do
      {:ok, cases}
    else
      {:error, reason} -> {:error, reason}
      _ -> {:error, :invalid_quality_execution_batch_cases}
    end
  end

  defp normalize_execution_cases(%{"cases" => cases}, _config)
       when is_list(cases) and cases != [],
       do: {:ok, Enum.filter(cases, &is_map/1)}

  defp normalize_execution_cases(_payload, _config),
    do: {:error, :invalid_quality_execution_batch_cases}

  defp build_execution_tasks(cases, operator_id, config) do
    cases
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, []}, fn {case_payload, index}, {:ok, acc} ->
      case build_execution_task(case_payload, index, operator_id, config) do
        {:ok, task} -> {:cont, {:ok, [task | acc]}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
    |> case do
      {:ok, tasks} -> {:ok, Enum.reverse(tasks)}
      error -> error
    end
  end

  defp build_execution_task(case_payload, index, operator_id, config) do
    case_id = case_id(case_payload, index)

    with {:ok, input_artifact} <- case_input_artifact(case_payload, config),
         {:ok, task_ir} <-
           OperatorTaskIR.build(
             operator_id,
             input_artifact,
             operator_config(case_payload, config),
             task_id: "#{Map.get(config, "task_id_prefix", "quality-task")}:#{case_id}",
             orchestration_context: map_config(config, "orchestration_context"),
             dataset_contract: map_config(config, "dataset_contract"),
             placement_tags: list_config(config, "placement_tags"),
             required_capabilities: list_config(config, "required_capabilities")
           ) do
      {:ok,
       %{
         "case_id" => case_id,
         "parameters" => Map.get(case_payload, "parameters", %{}),
         "operator_id" => operator_id,
         "task_id" => task_ir["task_id"],
         "task_digest" => get_in(task_ir, ["integrity", "task_digest"]),
         "task_ir" => task_ir,
         "agent_rpc" => %{
           "method" => OperatorTaskIR.agent_rpc_method(),
           "params" =>
             OperatorTaskIR.agent_rpc_params(task_ir, mode: Map.get(config, "rpc_mode")),
           "routing_opts" => OperatorTaskIR.agent_routing_opts(task_ir)
         }
       }}
    end
  end

  defp case_input_artifact(case_payload, %{"payload_source" => "case"}), do: {:ok, case_payload}

  defp case_input_artifact(case_payload, %{"payload_source" => source}) when is_binary(source) do
    case Map.get(case_payload, source) do
      value when is_map(value) -> {:ok, value}
      _ -> {:error, {:missing_quality_execution_case_payload, source}}
    end
  end

  defp case_input_artifact(case_payload, _config) do
    case Map.get(case_payload, "model") do
      value when is_map(value) -> {:ok, value}
      _ -> {:error, {:missing_quality_execution_case_payload, "model"}}
    end
  end

  defp operator_config(case_payload, config) do
    base = map_config(config, "operator_config")

    if Map.get(config, "merge_case_config", false) do
      Map.merge(base, map_config(case_payload, "config"))
    else
      base
    end
  end

  defp case_id(case_payload, index), do: Map.get(case_payload, "id", "case_#{index}")

  defp required_string(config, key) do
    case Map.get(config, key) do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> {:error, {:missing_quality_execution_batch_config, key}}
    end
  end

  defp map_config(config, key) do
    case Map.get(config, key) do
      value when is_map(value) -> value
      _ -> %{}
    end
  end

  defp list_config(config, key) do
    case Map.get(config, key) do
      values when is_list(values) -> Enum.filter(values, &is_binary/1)
      _ -> []
    end
  end

  defp selected_candidate(%{"ranking" => [first | _rest]}) when is_map(first), do: first
  defp selected_candidate(_payload), do: nil

  defp next_round_action(false, _score, _target_score, %{"require_ready" => true}), do: "replan"

  defp next_round_action(_ready, score, target_score, _config) when score <= target_score,
    do: "stop"

  defp next_round_action(_ready, _score, _target_score, _config), do: "continue"

  defp search_space_axes(search_space, config) when is_map(search_space) do
    samples = round(number_config(config, "samples_per_axis", 3))

    search_space
    |> Enum.flat_map(fn {path, spec} ->
      values = axis_values(spec, samples)
      if values == [], do: [], else: [%{"label" => path, "path" => path, "values" => values}]
    end)
  end

  defp search_space_axes(_search_space, _config), do: []

  defp axis_values(values, _samples) when is_list(values), do: values

  defp axis_values(%{"values" => values}, _samples) when is_list(values), do: values

  defp axis_values(%{"min" => min, "max" => max}, samples)
       when is_number(min) and is_number(max) and samples > 1 do
    step = (max - min) / (samples - 1)
    Enum.map(0..(samples - 1), &(min + step * &1))
  end

  defp axis_values(_spec, _samples), do: []

  defp estimate_case_count(axes) do
    Enum.reduce(axes, 1, fn axis, total ->
      total * length(Map.get(axis, "values", []))
    end)
  end

  defp quality_term(source_id, summary, config, missing_penalty, not_ready_penalty) do
    {score_field, score} = quality_number(summary, "_quality_score")
    domain = quality_domain(summary, score_field)
    ready = quality_bool(summary, "_quality_ready", false)
    grade = quality_string(summary, "_quality_grade", "unknown")

    missing_count =
      quality_number(summary, "_quality_missing_metric_count", 0.0) |> elem(1) |> round()

    weight = quality_weight(config, source_id, domain)
    weighted_score = score * weight
    missing_cost = missing_count * missing_penalty
    readiness_cost = if ready, do: 0.0, else: not_ready_penalty
    contribution = weighted_score + missing_cost + readiness_cost

    %{
      "source" => source_id,
      "domain" => domain,
      "score_field" => score_field,
      "score" => score,
      "weight" => weight,
      "weighted_score" => weighted_score,
      "ready" => ready,
      "grade" => grade,
      "missing_metric_count" => missing_count,
      "missing_metric_penalty" => missing_cost,
      "readiness_penalty" => readiness_cost,
      "contribution" => contribution
    }
  end

  defp quality_number(summary, suffix, default \\ nil) do
    case Enum.find(summary, fn {key, value} ->
           String.ends_with?(key, suffix) and is_number(value)
         end) do
      {key, value} -> {key, value * 1.0}
      nil when is_number(default) -> {String.trim_leading(suffix, "_"), default * 1.0}
      nil -> raise ArgumentError, "missing #{suffix}"
    end
  end

  defp quality_bool(summary, suffix, default),
    do:
      summary
      |> Enum.find_value(default, fn {key, value} ->
        if String.ends_with?(key, suffix) and is_boolean(value), do: value
      end)

  defp quality_string(summary, suffix, default),
    do:
      summary
      |> Enum.find_value(default, fn {key, value} ->
        if String.ends_with?(key, suffix) and is_binary(value), do: value
      end)

  defp quality_domain(summary, score_field) do
    Enum.find_value(summary, String.replace_suffix(score_field, "_quality_score", ""), fn {key,
                                                                                           value} ->
      if String.ends_with?(key, "_quality_contract") and is_binary(value) do
        value
        |> String.replace_prefix("kyuubiki.", "")
        |> String.replace_suffix("_quality_score/v1", "")
      end
    end)
  end

  defp quality_weight(config, source_id, domain) do
    weights = Map.get(config, "weights", %{})
    value = Map.get(weights, source_id, Map.get(weights, domain, 1.0))
    if is_number(value) and value >= 0.0, do: value * 1.0, else: 1.0
  end

  defp number_config(config, key, default) do
    value = Map.get(config, key, default)
    if is_number(value) and value >= 0.0, do: value * 1.0, else: default
  end

  defp number(primary, _fallback, _default) when is_number(primary), do: primary * 1.0
  defp number(_primary, fallback, _default) when is_number(fallback), do: fallback * 1.0
  defp number(_primary, _fallback, default), do: default * 1.0

  defp boolean(primary, _fallback, _default) when is_boolean(primary), do: primary
  defp boolean(_primary, fallback, _default) when is_boolean(fallback), do: fallback
  defp boolean(_primary, _fallback, default), do: default

  defp composite_grade(score, blocked_count, max_ready_score) do
    cond do
      blocked_count > 0 or score > max_ready_score -> "block"
      score > max_ready_score * 0.7 -> "review"
      score > max_ready_score * 0.35 -> "good"
      true -> "excellent"
    end
  end
end
