defmodule KyuubikiWeb.WorkflowQualitySweepAdmission do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowQualityReadiness, as: Readiness

  def admit(payload) do
    Check.converged(payload, "payload")
    request_value = payload["request_payload"]

    request =
      Check.object(
        if(is_nil(request_value), do: %{}, else: request_value),
        "payload.request_payload"
      )

    Check.converged(request, "payload.request_payload")
    action = Map.get(payload, "action", "continue")

    if action not in ~w(continue replan stop),
      do: Check.fail("payload.action must be continue, replan or stop")

    complete = Sweep.flag(payload, "source_ranking_complete")
    selected_ready = Sweep.flag(payload, "selected_candidate_ready", false)

    rejected =
      Check.list(Map.get(payload, "source_rejected_candidates", []), "source_rejected_candidates")

    Enum.each(rejected, &Check.object(&1, "source_rejected_candidates entry"))

    hint =
      resolve_hint([
        {request["optimization_hint"], "request_payload.optimization_hint"},
        {payload["selected_iteration_hint"], "selected_iteration_hint"}
      ])

    seed = request["seed_objective"]
    seed = if is_nil(seed), do: %{}, else: Check.object(seed, "request_payload.seed_objective")
    Check.converged(seed, "request_payload.seed_objective")

    coupled =
      Readiness.resolve([
        {request["coupled_readiness"], "request_payload.coupled_readiness"},
        {payload["selected_coupled_readiness"], "selected_coupled_readiness"},
        {payload["coupled_readiness"], "coupled_readiness"},
        {seed["coupled_readiness"], "request_payload.seed_objective.coupled_readiness"}
      ])

    for field <- ~w(coupled_readiness_state coupled_readiness_recommendation) do
      if payload[field] != nil && payload[field] !== (coupled || %{})[field],
        do: Check.fail("#{field} conflicts with coupled_readiness report")
    end

    contract = payload["source_coupled_readiness_contract"]

    if contract != nil && contract !== (coupled || %{})["coupled_readiness_contract"],
      do: Check.fail("source_coupled_readiness_contract conflicts with coupled_readiness report")

    incomplete = !complete || rejected != [] || hint["action"] == "repair_rejected_candidates"
    # Validate aliases even for a disabled plan; only execution intent is skipped.
    if !incomplete, do: validate_intent(action, selected_ready, hint, coupled)

    %{
      request: request,
      action: action,
      hint: hint,
      coupled: coupled,
      incomplete: incomplete,
      rejected: rejected
    }
  end

  def admit_plan(payload) do
    Check.converged(payload, "plan")
    enabled = Sweep.flag(payload, "sweep_enabled")
    hint = resolve_hint([{payload["optimization_hint"], "optimization_hint"}])
    coupled = Readiness.resolve([{payload["coupled_readiness"], "coupled_readiness"}])

    default_action =
      cond do
        !enabled -> "stop"
        hint["action"] == "fix_coupled_readiness" -> "replan"
        true -> "continue"
      end

    action = Map.get(payload, "sweep_action", default_action)

    if action not in ~w(continue replan stop),
      do: Check.fail("sweep_action must be continue, replan or stop")

    complete = Sweep.flag(payload, "source_ranking_complete")
    ready = Sweep.flag(payload, "selected_candidate_ready", false)

    rejected =
      Check.list(Map.get(payload, "source_rejected_candidates", []), "source_rejected_candidates")

    Enum.each(rejected, &Check.object(&1, "source_rejected_candidates entry"))
    reason = payload["sweep_blocking_reason"]
    if reason != nil, do: Sweep.text(payload, "sweep_blocking_reason", nil, "plan")

    if enabled do
      if action == "stop" || !complete || rejected != [] || reason != nil ||
           hint["action"] == "repair_rejected_candidates",
         do: Check.fail("sweep_enabled conflicts with stopped or incomplete plan")

      validate_intent(action, ready, hint, coupled)
    end

    strategy = payload["repair_strategy"]

    if strategy != nil && strategy !== repair_strategy(hint["action"]),
      do: Check.fail("repair_strategy conflicts with optimization_hint")

    if payload["repair_focus"] != nil do
      focus = Check.object(payload["repair_focus"], "repair_focus")

      expected = %{
        "field" => hint["focus_field"],
        "source" => hint["focus_source"],
        "domain" => hint["focus_domain"],
        "readiness_state" => (coupled || %{})["coupled_readiness_state"],
        "readiness_recommendation" => (coupled || %{})["coupled_readiness_recommendation"]
      }

      for {field, value} <- expected do
        if Map.has_key?(focus, field) && focus[field] !== value,
          do: Check.fail("repair_focus.#{field} conflicts with plan evidence")
      end
    end

    enabled
  end

  def repair_strategy(action) do
    case action do
      "fix_validation_failure" -> "rerun_validation_focused_sweep"
      "fix_coupled_readiness" -> "repair_coupled_readiness_sweep"
      "review_coupled_readiness" -> "review_coupled_readiness_sweep"
      "fix_blocking_term" -> "repair_blocking_term_sweep"
      "reduce_dominant_term" -> "reduce_dominant_term_sweep"
      _ -> "general_quality_sweep"
    end
  end

  defp resolve_hint(sources) do
    Enum.reduce(sources, nil, fn
      {nil, _}, selected ->
        selected

      {value, path}, selected ->
        object = Check.object(value, path)
        if Map.has_key?(object, "action"), do: Sweep.text(object, "action", nil, path)

        for field <- ~w(focus_field focus_source focus_domain),
            object[field] != nil,
            do: Sweep.text(object, field, nil, path)

        if selected != nil && selected !== object,
          do: Check.fail("optimization_hint conflicts with selected_iteration_hint")

        object
    end) || %{}
  end

  defp validate_intent(action, selected_ready, hint, coupled) do
    blocked = Readiness.blocked?(coupled)
    review = Readiness.review?(coupled)

    expected =
      cond do
        blocked -> "fix_coupled_readiness"
        review -> "review_coupled_readiness"
        true -> nil
      end

    if expected do
      if action == "stop" || (blocked && action != "replan") || hint["action"] != expected,
        do:
          Check.fail(
            "coupled_readiness requires explicit #{expected} intent, not action=#{action}"
          )

      if blocked && selected_ready,
        do: Check.fail("selected_candidate_ready conflicts with blocked coupled_readiness")

      expected_hint = Readiness.iteration_hint(coupled)

      for field <-
            ~w(focus_source readiness_state recommendation blocking_count warning_count blocking_domains required_missing warning_domains) do
        if Map.has_key?(hint, field) && hint[field] !== expected_hint[field],
          do: Check.fail("optimization_hint.#{field} conflicts with coupled_readiness evidence")
      end
    else
      if hint["action"] in ~w(fix_coupled_readiness review_coupled_readiness),
        do: Check.fail("optimization_hint requires matching coupled_readiness evidence")
    end
  end
end
