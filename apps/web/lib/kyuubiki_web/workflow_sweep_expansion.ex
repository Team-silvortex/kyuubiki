defmodule KyuubikiWeb.WorkflowSweepExpansion do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  @contract "kyuubiki.quality_sweep_expansion/v1"

  def run(payload, config) do
    Check.run("transform.expand_parameter_sweep", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)

      if Map.has_key?(payload, "quality_sweep_expansion_contract") do
        if payload["quality_sweep_expansion_contract"] != @contract,
          do: Check.fail("unsupported quality_sweep_expansion_contract")

        {budget, blocked} = Sweep.budget(payload)
        enabled = Sweep.flag(payload, "expansion_enabled")
        ready = Sweep.flag(payload, "expansion_budget_ready")

        cond do
          !enabled ->
            skipped(payload, false)

          !ready || blocked ->
            skipped(payload, true)

          true ->
            nested = Check.object(payload["payload"], "payload.payload")
            nested_config = Check.object(Map.get(payload, "config", %{}), "payload.config")
            expand(nested, Map.merge(nested_config, config), {payload, budget})
        end
      else
        expand(payload, config, nil)
      end
    end)
  end

  defp expand(payload, config, quality_context) do
    base = Sweep.object(Map.get(payload, "base", payload["model"]), "payload.base")
    raw_axes = Map.get(payload, "axes", config["axes"])
    size = Sweep.axes_count(raw_axes)
    max_cases = Sweep.count(Map.get(config, "max_cases", 256), "config.max_cases")
    id_prefix = Sweep.text(config, "id_prefix", "case", "config")

    cond do
      size > max_cases && quality_context != nil ->
        {source, budget} = quality_context

        source
        |> Map.put("case_count_estimate", size)
        |> Map.put(
          "sweep_budget",
          Map.merge(budget, %{
            "case_budget_exceeded" => true,
            "case_count_estimate" => size,
            "max_cases" => max_cases
          })
        )
        |> skipped(true)

      size > max_cases ->
        Check.fail("parameter sweep would emit #{size} cases, above max_cases #{max_cases}")

      true ->
        axes = Sweep.prepare_axes(base, raw_axes)

        metadata =
          Sweep.json(
            Map.get(payload, "case_metadata", Map.get(config, "case_metadata", %{})),
            "payload.case_metadata"
          )

        # Mixed-radix selection builds each final model once, without intermediate case trees.
        prepared =
          axes |> Enum.map(&Map.put(&1, :choices, List.to_tuple(&1.values))) |> Enum.reverse()

        cases =
          Enum.map(0..(size - 1), fn index ->
            {choices, _} =
              prepared
              |> Enum.map_reduce(index, fn axis, remaining ->
                count = tuple_size(axis.choices)
                {{axis, elem(axis.choices, rem(remaining, count))}, div(remaining, count)}
              end)

            {model, parameters} =
              Enum.reduce(Enum.reverse(choices), {base, %{}}, fn {axis, value},
                                                                 {model, parameters} ->
                {Sweep.assign(model, axis.target, value), Map.put(parameters, axis.label, value)}
              end)

            label =
              parameters
              |> Enum.sort()
              |> Enum.map_join(", ", fn {key, value} ->
                "#{key}=#{if is_binary(value), do: value, else: Jason.encode!(value)}"
              end)

            %{
              "id" => "#{id_prefix}_#{index}",
              "label" => label,
              "parameters" => parameters,
              "model" => model,
              "metadata" => metadata
            }
          end)

        %{"cases" => cases, "case_count" => size, "axis_count" => length(axes)}
    end
  end

  defp skipped(payload, blocked) do
    common = %{
      "cases" => [],
      "case_count" => 0,
      "axis_count" => 0,
      "sweep_enabled" => blocked,
      "expansion_enabled" => blocked,
      "source_rejected_candidates" => Map.get(payload, "source_rejected_candidates", [])
    }

    if blocked do
      Map.merge(common, %{
        "expansion_budget_ready" => false,
        "expansion_blocking_reason" => "case_budget_exceeded",
        "case_count_estimate" => payload["case_count_estimate"],
        "sweep_budget" => Map.get(payload, "sweep_budget", %{}),
        "source_candidate_id" => payload["source_candidate_id"]
      })
    else
      Map.merge(common, %{
        "sweep_action" => Map.get(payload, "reason", "stopped"),
        "sweep_blocking_reason" => payload["expansion_blocking_reason"]
      })
    end
  end
end
