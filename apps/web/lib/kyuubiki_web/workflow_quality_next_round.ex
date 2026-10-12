defmodule KyuubikiWeb.WorkflowQualityNextRound do
  @moduledoc false

  alias KyuubikiWeb.WorkflowQualityComposition, as: Quality
  alias KyuubikiWeb.WorkflowQualityRanking, as: Ranking
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def run(payload, config) do
    Check.run("transform.prepare_quality_next_round_request", fn ->
      payload = Check.object(payload, "payload")
      Check.converged(payload, "payload")
      config = Quality.checked_config(config)

      require_ready =
        Quality.boolean(Map.get(config, "require_ready", false), "config.require_ready")

      ranking = Check.list(payload["ranking"], "payload.ranking")
      if ranking == [], do: Check.fail("payload.ranking must not be empty")

      ranking =
        Enum.with_index(ranking)
        |> Enum.map(fn {entry, index} -> checked_entry(entry, index) end)

      Enum.reduce(ranking, nil, fn entry, previous ->
        key = Ranking.order_key(entry)

        if previous != nil && previous > key,
          do: Check.fail("payload.ranking is out of order: ready, score, candidate_id")

        key
      end)

      [selected | _] = ranking
      {complete, rejected} = completeness(payload, ranking)
      id = selected["candidate_id"]
      score = selected["score"]

      for {key, value} <- [
            {"best_candidate_id", id},
            {"best_candidate_score", score},
            {"best_candidate_ready", selected["ready"]}
          ] do
        if Map.has_key?(payload, key) && payload[key] != value,
          do: Check.fail("payload.#{key} disagrees with selected candidate")
      end

      objective = optional_object(selected["objective"], "selected.objective")
      Check.converged(objective, "selected.objective")

      blockers =
        Check.list(
          Map.get(
            selected,
            "blocking_terms",
            Map.get(objective, "composite_quality_blocking_terms", [])
          ),
          "selected.blocking_terms"
        )

      objective_blockers =
        Check.list(
          Map.get(objective, "composite_quality_blocking_terms", []),
          "selected.objective.composite_quality_blocking_terms"
        )

      blockers = Enum.uniq(blockers ++ objective_blockers)

      objective_ready =
        Quality.boolean(
          Map.get(objective, "composite_quality_ready", true),
          "selected.objective.composite_quality_ready"
        )

      ready = selected["ready"] && objective_ready && blockers == []
      dominant = Map.get(selected, "dominant_term", objective["composite_quality_dominant_term"])

      coupled =
        Map.get(
          selected,
          "coupled_readiness",
          Map.get(objective, "coupled_readiness", payload["coupled_readiness"])
        )

      coupled_map = optional_object(coupled, "selected.coupled_readiness")

      state =
        Check.choice(
          coupled_map,
          "coupled_readiness_state",
          "ready",
          ~w(ready pass warn block),
          "selected.coupled_readiness"
        )

      recommendation =
        Check.choice(
          coupled_map,
          "coupled_readiness_recommendation",
          "continue_to_next_round",
          ~w(continue_to_next_round review_before_next_round hold_and_repair_inputs),
          "selected.coupled_readiness"
        )

      target = Quality.nonnegative(Map.get(config, "target_score", 3), "config.target_score")

      action =
        cond do
          !complete || blockers != [] || !objective_ready || state == "block" ||
              recommendation == "hold_and_repair_inputs" ->
            "replan"

          state == "warn" || recommendation == "review_before_next_round" ->
            "continue"

          !ready ->
            if require_ready, do: "replan", else: "continue"

          score <= target ->
            "stop"

          true ->
            "continue"
        end

      hint =
        if !complete do
          %{
            "action" => "repair_rejected_candidates",
            "focus_source" => get_in(List.first(rejected) || %{}, ["candidate_id"]),
            "blocking_count" => length(rejected),
            "reason" =>
              "Complete failed candidate evaluations before accepting the research ranking."
          }
        else
          iteration_hint(dominant, blockers, state, recommendation)
        end

      %{
        "quality_next_round_contract" => "kyuubiki.quality_next_round_request/v1",
        "action" => action,
        "selected_candidate_id" => id,
        "selected_candidate_score" => score,
        "selected_candidate_ready" => ready,
        "selected_candidate_metadata" => selected["metadata"],
        "selected_dominant_term" => dominant,
        "selected_blocking_terms" => blockers,
        "selected_iteration_hint" => hint,
        "selected_coupled_readiness" => coupled,
        "source_coupled_readiness_contract" => coupled_map["coupled_readiness_contract"],
        "coupled_readiness_state" => coupled_map["coupled_readiness_state"],
        "coupled_readiness_recommendation" => coupled_map["coupled_readiness_recommendation"],
        "target_score" => target,
        "source_ranking_contract" => payload["quality_candidate_ranking_contract"],
        "source_ranking_complete" => complete,
        "source_rejected_candidates" => rejected,
        "request_payload" => %{
          "seed_candidate_id" => id,
          "seed_objective" => selected["objective"],
          "seed_metadata" => selected["metadata"],
          "optimization_hint" => hint,
          "coupled_readiness" => coupled,
          "constraints" => Map.get(config, "constraints", %{}),
          "search_space" => Map.get(config, "search_space", %{}),
          "max_candidates" =>
            Quality.nonnegative(Map.get(config, "max_candidates", 8), "config.max_candidates")
        },
        "next_round_summary" =>
          "Quality exploration #{action}: selected=#{id}, score=#{score}, target=#{target}."
      }
    end)
  end

  defp checked_entry(value, index) do
    path = "payload.ranking[#{index}]"
    entry = Check.object(value, path)
    Check.converged(entry, path)
    id = Check.text(entry, "candidate_id", path)
    if is_nil(id), do: Check.fail("#{path}.candidate_id is required")
    score = Quality.nonnegative(entry["score"], "#{path}.score")
    ready = Quality.boolean(Map.get(entry, "ready", false), "#{path}.ready")

    if Map.has_key?(entry, "rank") && Check.unsigned(entry["rank"], "#{path}.rank") != index + 1,
      do: Check.fail("#{path}.rank disagrees with its position")

    checked_objective(entry["objective"], score, "#{path}.objective")
    Map.merge(entry, %{"score" => score, "ready" => ready})
  end

  defp checked_objective(value, score, path) do
    objective = optional_object(value, path)
    Check.converged(objective, path)

    if Map.has_key?(objective, "composite_quality_score") &&
         Quality.nonnegative(
           objective["composite_quality_score"],
           "#{path}.composite_quality_score"
         ) != score,
       do: Check.fail("#{path}.composite_quality_score disagrees with ranking score")

    Quality.boolean(
      Map.get(objective, "composite_quality_ready", true),
      "#{path}.composite_quality_ready"
    )

    Check.list(
      Map.get(objective, "composite_quality_blocking_terms", []),
      "#{path}.composite_quality_blocking_terms"
    )
  end

  defp completeness(payload, ranking) do
    rejected =
      Check.list(Map.get(payload, "rejected_candidates", []), "payload.rejected_candidates")

    rejected =
      Enum.with_index(rejected)
      |> Enum.map(fn {entry, index} ->
        path = "payload.rejected_candidates[#{index}]"
        entry = Check.object(entry, path)
        id = Check.text(entry, "candidate_id", path)
        error = Check.text(entry, "error", path)
        if is_nil(id) || is_nil(error), do: Check.fail("#{path} requires candidate_id and error")
        entry
      end)

    ids = Enum.map(ranking ++ rejected, & &1["candidate_id"])

    if length(Enum.uniq(ids)) != length(ids),
      do: Check.fail("payload.ranking contains duplicate candidate ids")

    for {key, actual} <- [
          {"candidate_count", length(ranking)},
          {"input_candidate_count", length(ids)},
          {"rejected_candidate_count", length(rejected)},
          {"ready_candidate_count", Enum.count(ranking, & &1["ready"])}
        ] do
      if Map.has_key?(payload, key) && Check.unsigned(payload[key], "payload.#{key}") != actual,
        do: Check.fail("payload.#{key} disagrees with candidate details")
    end

    complete =
      Quality.boolean(
        Map.get(payload, "ranking_complete", rejected == []),
        "payload.ranking_complete"
      )

    if complete && rejected != [],
      do: Check.fail("payload.ranking_complete cannot be true with rejected candidates")

    {complete, rejected}
  end

  defp optional_object(nil, _path), do: %{}
  defp optional_object(value, path), do: Check.object(value, path)

  defp iteration_hint(dominant, blockers, state, recommendation) do
    cond do
      state == "block" || recommendation == "hold_and_repair_inputs" ->
        %{
          "action" => "fix_coupled_readiness",
          "reason" => "Repair coupled readiness before accepting the ranking."
        }

      state == "warn" || recommendation == "review_before_next_round" ->
        %{
          "action" => "review_coupled_readiness",
          "reason" => "Review coupled readiness before stopping research."
        }

      blockers != [] ->
        blocker = Check.object(hd(blockers), "selected.blocking_terms[0]")

        source_terms =
          Check.list(
            Map.get(blocker, "source_blocking_terms", []),
            "selected.blocking_terms[0].source_blocking_terms"
          )

        source =
          optional_object(
            List.first(source_terms),
            "selected.blocking_terms[0].source_blocking_terms[0]"
          )

        nested =
          optional_object(blocker["dominant_term"], "selected.blocking_terms[0].dominant_term")

        %{
          "action" =>
            if(blocker["domain"] == "validation",
              do: "fix_validation_failure",
              else: "fix_blocking_term"
            ),
          "focus_domain" => blocker["domain"],
          "focus_source" => blocker["source"],
          "focus_field" => Map.get(source, "field", Map.get(nested, "field", blocker["field"])),
          "blocking_count" => length(blockers),
          "source_blocking_term" => List.first(source_terms)
        }

      true ->
        dominant = optional_object(dominant, "selected.dominant_term")

        nested =
          optional_object(dominant["dominant_term"], "selected.dominant_term.dominant_term")

        %{
          "action" => "reduce_dominant_term",
          "focus_domain" => dominant["domain"],
          "focus_source" => dominant["source"],
          "focus_field" => nested["field"],
          "blocking_count" => 0,
          "source_dominant_term" => dominant["dominant_term"]
        }
    end
  end
end
