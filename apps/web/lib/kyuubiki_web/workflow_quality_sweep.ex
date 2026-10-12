defmodule KyuubikiWeb.WorkflowQualitySweep do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep

  def plan(payload, config) do
    Check.run("transform.build_quality_parameter_sweep_plan", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      request = Check.object(Map.get(payload, "request_payload", %{}), "payload.request_payload")
      action = Check.choice(payload, "action", "continue", ~w(continue replan stop), "payload")
      complete = Sweep.flag(payload, "source_ranking_complete")

      rejected =
        Check.list(
          Map.get(payload, "source_rejected_candidates", []),
          "payload.source_rejected_candidates"
        )

      hint = Map.get(request, "optimization_hint", payload["selected_iteration_hint"])
      hint = if is_nil(hint), do: %{}, else: Sweep.object(hint, "payload.optimization_hint")

      base =
        Sweep.object(
          Map.get(config, "base", Map.get(payload, "base", Map.get(request, "base", %{}))),
          "base"
        )

      samples = Sweep.count(Map.get(config, "samples_per_axis", 3), "config.samples_per_axis", 2)

      max_cases =
        Sweep.count(
          Map.get(config, "max_cases", Map.get(request, "max_candidates", 64)),
          "config.max_cases"
        )

      prefix = Sweep.text(config, "id_prefix", "quality_round", "config")

      cond do
        !complete || rejected != [] || hint["action"] == "repair_rejected_candidates" ->
          disabled(base, "replan")
          |> Map.merge(%{
            "sweep_blocking_reason" => "candidate_evaluation_incomplete",
            "source_rejected_candidates" => rejected,
            "optimization_hint" => hint
          })

        action == "stop" ->
          disabled(base, "stop")

        true ->
          search =
            Check.object(Map.get(request, "search_space", config["search_space"]), "search_space")

          if map_size(search) == 0, do: Check.fail("search_space must contain usable axes")

          max_axes =
            Sweep.count(Map.get(config, "max_axes", map_size(search)), "config.max_axes", 1)

          planned =
            search
            |> Enum.map(fn {path, spec} -> planned_axis(path, spec, samples) end)
            |> Enum.sort_by(fn axis ->
              {if(focused?(axis.path, hint["focus_field"]), do: 0, else: 1), axis.path}
            end)

          selected = Enum.take(planned, max_axes)
          size = Sweep.product(Enum.map(selected, & &1.count))
          exceeded = size > max_cases
          axes = Enum.map(selected, &materialize_axis(&1, exceeded))

          budget = %{
            "case_budget_exceeded" => exceeded,
            "case_count_estimate" => size,
            "max_cases" => max_cases,
            "usable_axis_count" => length(planned),
            "planned_axis_count" => length(axes),
            "planned_axis_paths" => Enum.map(selected, & &1.path),
            "axis_budget_truncated" => length(axes) < length(planned),
            "status" =>
              cond do
                exceeded -> "case_budget_exceeded"
                length(axes) < length(planned) -> "axis_budget_truncated"
                true -> "ok"
              end,
            "recommendation" =>
              cond do
                exceeded -> "replan_within_case_budget"
                length(axes) < length(planned) -> "schedule_followup_axis_batch"
                true -> "run_planned_sweep"
              end
          }

          %{
            "quality_parameter_sweep_plan_contract" => "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled" => true,
            "sweep_action" => action,
            "source_candidate_id" => payload["selected_candidate_id"],
            "target_score" => payload["target_score"],
            "id_prefix" => prefix,
            "max_cases" => max_cases,
            "case_count_estimate" => size,
            "sweep_budget" => budget,
            "axes" => axes,
            "base" => base,
            "optimization_hint" => hint,
            "seed_metadata" =>
              Sweep.json(
                Map.get(request, "seed_metadata", payload["selected_candidate_metadata"]),
                "payload.seed_metadata"
              ),
            "coupled_readiness" =>
              Map.get(request, "coupled_readiness", payload["selected_coupled_readiness"]),
            "focused_axis_path" => hd(selected).path,
            "plan_summary" =>
              "Quality parameter sweep planned with #{length(axes)} axis/axes and #{size} estimated cases."
          }
      end
    end)
  end

  def materialize(payload, config) do
    Check.run("transform.materialize_quality_sweep_expansion", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      enabled = Sweep.flag(payload, "sweep_enabled")

      max_cases =
        Sweep.count(
          Map.get(config, "max_cases", Map.get(payload, "max_cases", 64)),
          "config.max_cases"
        )

      prefix =
        Sweep.text(config, "id_prefix", Map.get(payload, "id_prefix", "quality_round"), "config")

      if !enabled do
        %{
          "quality_sweep_expansion_contract" => "kyuubiki.quality_sweep_expansion/v1",
          "expansion_enabled" => false,
          "reason" => Map.get(payload, "sweep_action", "stopped"),
          "expansion_blocking_reason" => payload["sweep_blocking_reason"],
          "source_rejected_candidates" => Map.get(payload, "source_rejected_candidates", []),
          "payload" => nil,
          "config" => nil
        }
      else
        {budget, blocked} = Sweep.budget(payload)
        axes = payload["axes"]
        size = Sweep.axes_count(axes, blocked)
        Sweep.require_count(payload, "case_count_estimate", size)
        ready = !blocked && size <= max_cases
        base = Sweep.object(Map.get(payload, "base", %{}), "payload.base")
        if ready, do: Sweep.prepare_axes(base, axes)

        budget =
          Map.merge(budget, %{
            "case_budget_exceeded" => !ready,
            "case_count_estimate" => size,
            "max_cases" => max_cases
          })

        metadata =
          Map.take(
            payload,
            ~w(source_candidate_id seed_metadata target_score optimization_hint coupled_readiness focused_axis_path repair_strategy repair_focus)
          )
          |> Map.put("source_plan_contract", payload["quality_parameter_sweep_plan_contract"])
          |> Map.put("sweep_budget", budget)
          |> Sweep.json("payload.case_metadata")

        %{
          "quality_sweep_expansion_contract" => "kyuubiki.quality_sweep_expansion/v1",
          "expansion_enabled" => true,
          "expansion_budget_ready" => ready,
          "expansion_blocking_reason" => if(ready, do: nil, else: "case_budget_exceeded"),
          "source_plan_contract" => payload["quality_parameter_sweep_plan_contract"],
          "source_candidate_id" => payload["source_candidate_id"],
          "case_count_estimate" => size,
          "sweep_budget" => budget,
          "payload" => %{"base" => base, "axes" => axes, "case_metadata" => metadata},
          "config" => %{"id_prefix" => prefix, "max_cases" => max_cases},
          "expansion_summary" =>
            "Materialized quality sweep expansion with #{length(axes)} axis/axes."
        }
      end
    end)
  end

  defp disabled(base, action),
    do: %{
      "quality_parameter_sweep_plan_contract" => "kyuubiki.quality_parameter_sweep_plan/v1",
      "sweep_enabled" => false,
      "sweep_action" => action,
      "case_count_estimate" => 0,
      "axes" => [],
      "base" => base,
      "plan_summary" => "Quality exploration #{action}; no parameter sweep was planned."
    }

  defp planned_axis(path, spec, samples) do
    Sweep.segments(path, "search_space.#{path}")

    cond do
      is_list(spec) ->
        explicit(path, spec)

      is_map(spec) && Map.has_key?(spec, "values") ->
        explicit(path, spec["values"])

      is_map(spec) ->
        min = Check.number(spec["min"], "search_space.#{path}.min")
        max = Check.number(spec["max"], "search_space.#{path}.max")
        %{path: path, count: samples, range: {min, max}}

      true ->
        Check.fail("search_space.#{path} must contain values or a finite range")
    end
  end

  defp explicit(path, values) do
    values = Sweep.values(values, "search_space.#{path}.values")
    %{path: path, count: length(values), values: values}
  end

  defp materialize_axis(axis, deferred) do
    base = %{"path" => axis.path, "label" => axis.path, "value_count" => axis.count}

    if deferred,
      do: Map.put(base, "values_deferred", true),
      else: Map.put(base, "values", samples(axis))
  end

  defp samples(%{values: values}), do: values

  defp samples(%{range: {min, max}, count: count, path: path}) do
    Enum.map(0..(count - 1), fn index ->
      Check.calculate("search_space.#{path}.sample[#{index}]", fn ->
        t = index / (count - 1)

        cond do
          index == 0 || min == max -> min
          index == count - 1 -> max
          min < 0 == max < 0 -> min + (max - min) * t
          true -> min * (1 - t) + max * t
        end
      end)
    end)
  end

  defp focused?(path, field) when is_binary(field),
    do: path == field || String.ends_with?(path, ".#{field}")

  defp focused?(_path, _field), do: false
end
