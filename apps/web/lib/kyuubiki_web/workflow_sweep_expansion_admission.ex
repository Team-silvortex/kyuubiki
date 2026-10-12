defmodule KyuubikiWeb.WorkflowSweepExpansionAdmission do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowQualitySweepAdmission, as: Quality

  @quality_fields ~w(source_plan_contract coupled_readiness optimization_hint repair_strategy
    repair_focus sweep_enabled sweep_action sweep_blocking_reason source_ranking_complete
    source_rejected_candidates selected_candidate_ready sweep_budget converged stability_result)

  def validate(payload, config) do
    Check.converged(payload, "payload")
    check_config(config)
    wrapped = Map.has_key?(payload, "quality_sweep_expansion_contract")

    {body, nested_config, enabled, blocked} =
      if wrapped do
        if payload["quality_sweep_expansion_contract"] != "kyuubiki.quality_sweep_expansion/v1",
          do: Check.fail("unsupported quality_sweep_expansion_contract")

        enabled = Sweep.flag(payload, "expansion_enabled")
        ready = Sweep.flag(payload, "expansion_budget_ready")
        blocked = budget_blocked?(payload) || !ready
        body = payload["payload"]

        if body != nil || (enabled && !blocked) do
          Check.object(body, "payload.payload")
          Check.converged(body, "payload.payload")
        end

        nested_config = check_config(payload["config"])
        {body || %{}, nested_config, enabled, blocked}
      else
        {payload, %{}, true, false}
      end

    aliases =
      for source <- [body, nested_config, config],
          Map.has_key?(source, "case_metadata"),
          do: source["case_metadata"]

    quality =
      wrapped ||
        Enum.any?(aliases, fn value ->
          is_map(value) && Enum.any?(@quality_fields, &Map.has_key?(value, &1))
        end)

    effective_config = Map.merge(nested_config, config)
    axes = Map.get(body, "axes", effective_config["axes"])
    supplied = Map.has_key?(body, "axes") || Map.has_key?(effective_config, "axes")

    actual =
      cond do
        supplied -> Sweep.axes_count(axes, blocked)
        enabled && !blocked -> Check.fail("transform.expand_parameter_sweep requires axes")
        true -> nil
      end

    if quality && aliases != [] do
      [metadata | rest] = aliases
      # Validate declarations before merging can discard a conflicting evidence alias.
      if Enum.any?(rest, &(&1 !== metadata)),
        do: Check.fail("quality case_metadata aliases conflict across payload and config")

      if metadata != nil do
        Check.object(metadata, "quality case_metadata")
        metadata_enabled = Quality.admit_plan(metadata)
        metadata_blocked = budget_blocked?(metadata)

        if enabled && !blocked && (!metadata_enabled || metadata_blocked),
          do:
            Check.fail(
              "quality expansion conflicts with disabled or budget-blocked case_metadata"
            )

        if wrapped do
          for field <- ~w(source_candidate_id source_plan_contract) do
            if payload[field] != nil && metadata[field] != nil &&
                 payload[field] !== metadata[field],
               do: Check.fail("quality expansion #{field} conflicts with case_metadata")
          end
        end
      end
    end

    sources =
      if quality, do: [payload, body | Enum.filter(aliases, &is_map/1)], else: [payload, body]

    validate_counts(sources, actual, enabled && !blocked)
    actual
  end

  defp validate_counts(sources, actual, active) do
    expected =
      Enum.reduce(sources, actual, fn source, expected ->
        budget = Check.config(source["sweep_budget"])

        Enum.reduce([source, budget], expected, fn entry, previous ->
          if Map.has_key?(entry, "case_count_estimate") do
            count = Sweep.count(entry["case_count_estimate"], "case_count_estimate")

            if previous != nil && previous != count,
              do: Check.fail("case_count_estimate conflicts with axes or another count mirror")

            count
          else
            previous
          end
        end)
      end)

    for source <- sources do
      budget = Check.config(source["sweep_budget"])
      blocked = Sweep.flag(budget, "case_budget_exceeded", false)

      if active && blocked,
        do: Check.fail("parameter sweep execution conflicts with a supplied budget block")

      if Map.has_key?(budget, "max_cases") do
        limit = Sweep.count(budget["max_cases"], "sweep_budget.max_cases")

        if !blocked && expected != nil && expected > limit,
          do:
            Check.fail(
              "sweep_budget must remain blocked when its declared case limit is exceeded"
            )
      end
    end
  end

  defp check_config(value) do
    config = Check.config(value)
    Sweep.count(Map.get(config, "max_cases", 256), "max_cases")
    Sweep.text(config, "id_prefix", "case", "config")
    config
  end

  defp budget_blocked?(value) do
    budget = Check.config(value["sweep_budget"])
    Sweep.flag(budget, "case_budget_exceeded", false)
  end
end
