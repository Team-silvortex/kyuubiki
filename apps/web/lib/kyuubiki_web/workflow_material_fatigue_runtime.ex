defmodule KyuubikiWeb.WorkflowMaterialFatigueRuntime do
  @moduledoc false

  def estimate_material_fatigue_life(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, fatigue_config} <- parse_config(config),
         candidates when candidates != [] <- candidate_entries(payload),
         {:ok, assessments} <- assess_candidates(candidates, fatigue_config) do
      sorted = Enum.sort_by(assessments, &{-&1["fatigue_safety_factor"], &1["candidate_id"]})
      best = List.first(sorted, %{})
      pass_count = Enum.count(sorted, &(Map.get(&1, "fatigue_status") == "pass"))

      {:ok,
       %{
         "material_fatigue_candidate_count" => length(sorted),
         "material_fatigue_pass_count" => pass_count,
         "material_fatigue_best_candidate_id" => Map.get(best, "candidate_id", ""),
         "material_fatigue_best_safety_factor" => Map.get(best, "fatigue_safety_factor", 0.0),
         "material_fatigue_target_cycles" => fatigue_config.target_cycles,
         "material_fatigue_model" => fatigue_config.model,
         "material_fatigue_assessments" => sorted
       }
       |> maybe_merge_single_assessment(sorted)}
    else
      [] -> {:error, :missing_material_fatigue_candidates}
      {:error, _reason} = error -> error
    end
  end

  def estimate_material_fatigue_life(_payload, _config),
    do: {:error, :invalid_material_fatigue_payload}

  defp parse_config(config) do
    fatigue_strength = numeric_config(config, "fatigue_strength")
    reference_cycles = numeric_config(config, "reference_cycles", 1.0e6)
    slope_exponent = numeric_config(config, "slope_exponent", 5.0)
    target_cycles = numeric_config(config, "target_cycles", reference_cycles)

    cond do
      not is_number(fatigue_strength) or fatigue_strength <= 0.0 ->
        {:error, :missing_material_fatigue_strength}

      reference_cycles <= 0.0 or slope_exponent <= 0.0 or target_cycles <= 0.0 ->
        {:error, :invalid_material_fatigue_config}

      true ->
        {:ok,
         %{
           fatigue_strength: fatigue_strength,
           reference_cycles: reference_cycles,
           slope_exponent: slope_exponent,
           target_cycles: target_cycles,
           stress_amplitude_field: Map.get(config, "stress_amplitude_field", "stress_amplitude"),
           mean_stress_field: Map.get(config, "mean_stress_field", "mean_stress"),
           ultimate_strength_field:
             Map.get(config, "ultimate_strength_field", "ultimate_strength"),
           fallback_stress_field: Map.get(config, "fallback_stress_field", "max_stress"),
           model: Map.get(config, "model", "basquin_goodman")
         }}
    end
  end

  defp assess_candidates(candidates, config) do
    candidates
    |> Enum.reduce_while({:ok, []}, fn {candidate_id, summary}, {:ok, acc} ->
      case assess_candidate(candidate_id, summary, config) do
        {:ok, assessment} -> {:cont, {:ok, [assessment | acc]}}
        {:error, _reason} = error -> {:halt, error}
      end
    end)
    |> case do
      {:ok, assessments} -> {:ok, Enum.reverse(assessments)}
      error -> error
    end
  end

  defp assess_candidate(candidate_id, summary, config) do
    with amplitude when is_number(amplitude) and amplitude > 0.0 <-
           fatigue_stress_amplitude(summary, config),
         {:ok, adjusted_amplitude, correction} <-
           goodman_adjusted_amplitude(summary, amplitude, config) do
      estimated_cycles =
        config.reference_cycles *
          :math.pow(config.fatigue_strength / adjusted_amplitude, config.slope_exponent)

      safety_factor = estimated_cycles / config.target_cycles

      {:ok,
       %{
         "candidate_id" => candidate_id,
         "fatigue_status" => if(safety_factor >= 1.0, do: "pass", else: "fail"),
         "fatigue_estimated_cycles" => estimated_cycles,
         "fatigue_target_cycles" => config.target_cycles,
         "fatigue_safety_factor" => safety_factor,
         "fatigue_stress_amplitude" => amplitude,
         "fatigue_adjusted_stress_amplitude" => adjusted_amplitude,
         "fatigue_correction" => correction,
         "summary" => summary
       }}
    else
      nil -> {:error, :missing_material_fatigue_stress_amplitude}
      false -> {:error, :invalid_material_fatigue_stress_amplitude}
      {:error, _reason} = error -> error
    end
  end

  defp fatigue_stress_amplitude(summary, config) do
    lookup_number(summary, config.stress_amplitude_field) ||
      half_range(summary) ||
      lookup_number(summary, config.fallback_stress_field)
  end

  defp half_range(summary) do
    with max_stress when is_number(max_stress) <- lookup_number(summary, "max_stress"),
         min_stress when is_number(min_stress) <- lookup_number(summary, "min_stress") do
      abs(max_stress - min_stress) / 2.0
    else
      _ -> nil
    end
  end

  defp goodman_adjusted_amplitude(summary, amplitude, config) do
    mean_stress = lookup_number(summary, config.mean_stress_field) || 0.0
    ultimate_strength = lookup_number(summary, config.ultimate_strength_field)

    cond do
      not is_number(ultimate_strength) ->
        {:ok, amplitude, %{"kind" => "none"}}

      ultimate_strength <= 0.0 or mean_stress >= ultimate_strength ->
        {:error, :invalid_material_fatigue_goodman_limit}

      true ->
        factor = 1.0 - mean_stress / ultimate_strength

        {:ok, amplitude / factor,
         %{
           "kind" => "goodman",
           "mean_stress" => mean_stress,
           "ultimate_strength" => ultimate_strength,
           "correction_factor" => factor
         }}
    end
  end

  defp maybe_merge_single_assessment(output, [single]) do
    Map.merge(output, %{
      "material_fatigue_status" => single["fatigue_status"],
      "material_fatigue_estimated_cycles" => single["fatigue_estimated_cycles"],
      "material_fatigue_safety_factor" => single["fatigue_safety_factor"],
      "material_fatigue_stress_amplitude" => single["fatigue_stress_amplitude"]
    })
  end

  defp maybe_merge_single_assessment(output, _assessments), do: output

  defp candidate_entries(%{"candidates" => candidates}) when is_map(candidates) do
    candidates
    |> Enum.filter(fn {_id, value} -> is_map(value) end)
    |> Enum.map(fn {id, summary} -> {id, summary} end)
  end

  defp candidate_entries(payload) when is_map(payload), do: [{"summary", payload}]

  defp numeric_config(config, field, default \\ nil) do
    case Map.get(config, field, default) do
      value when is_number(value) -> value * 1.0
      _ -> default
    end
  end

  defp lookup_number(payload, field) when is_map(payload) and is_binary(field) do
    case get_in(payload, String.split(field, ".")) ||
           get_in(payload, ["summary" | String.split(field, ".")]) do
      value when is_number(value) -> value * 1.0
      _ -> nil
    end
  end
end
