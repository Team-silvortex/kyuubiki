defmodule KyuubikiWeb.WorkflowMaterialThermalShockRuntime do
  @moduledoc false

  def evaluate_material_thermal_shock(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, shock_config} <- parse_config(config),
         candidates when candidates != [] <- candidate_entries(payload),
         {:ok, assessments} <- assess_candidates(candidates, shock_config) do
      sorted =
        Enum.sort_by(assessments, &{-&1["thermal_shock_safety_factor"], &1["candidate_id"]})

      best = List.first(sorted, %{})
      pass_count = Enum.count(sorted, &(Map.get(&1, "thermal_shock_status") == "pass"))

      {:ok,
       %{
         "material_thermal_shock_candidate_count" => length(sorted),
         "material_thermal_shock_pass_count" => pass_count,
         "material_thermal_shock_best_candidate_id" => Map.get(best, "candidate_id", ""),
         "material_thermal_shock_best_safety_factor" =>
           Map.get(best, "thermal_shock_safety_factor", 0.0),
         "material_thermal_shock_assessments" => sorted
       }
       |> maybe_merge_single_assessment(sorted)}
    else
      [] -> {:error, :missing_material_thermal_shock_candidates}
      {:error, _reason} = error -> error
    end
  end

  def evaluate_material_thermal_shock(_payload, _config),
    do: {:error, :invalid_material_thermal_shock_payload}

  defp parse_config(config) do
    constraint_factor = numeric_config(config, "constraint_factor", 1.0)
    safety_factor_target = numeric_config(config, "safety_factor_target", 1.0)

    if constraint_factor > 0.0 and safety_factor_target > 0.0 do
      {:ok,
       %{
         constraint_factor: constraint_factor,
         safety_factor_target: safety_factor_target,
         temperature_delta_field: Map.get(config, "temperature_delta_field", "temperature_delta"),
         thermal_expansion_field: Map.get(config, "thermal_expansion_field", "thermal_expansion"),
         youngs_modulus_field: Map.get(config, "youngs_modulus_field", "youngs_modulus"),
         poisson_ratio_field: Map.get(config, "poisson_ratio_field", "poisson_ratio"),
         strength_field: Map.get(config, "strength_field", "yield_strength"),
         fallback_strength_field: Map.get(config, "fallback_strength_field", "tensile_strength"),
         fracture_toughness_field:
           Map.get(config, "fracture_toughness_field", "fracture_toughness"),
         flaw_size_field: Map.get(config, "flaw_size_field", "flaw_size")
       }}
    else
      {:error, :invalid_material_thermal_shock_config}
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
    with delta_t when is_number(delta_t) and delta_t > 0.0 <- temperature_delta(summary, config),
         alpha when is_number(alpha) and alpha > 0.0 <-
           lookup_number(summary, config.thermal_expansion_field),
         youngs_modulus when is_number(youngs_modulus) and youngs_modulus > 0.0 <-
           lookup_number(summary, config.youngs_modulus_field),
         strength when is_number(strength) and strength > 0.0 <- strength(summary, config) do
      poisson_ratio = lookup_number(summary, config.poisson_ratio_field) || 0.0
      thermal_strain = alpha * delta_t
      thermal_stress = thermal_stress(youngs_modulus, thermal_strain, poisson_ratio, config)
      stress_index = thermal_stress / strength
      fracture_index = fracture_index(summary, thermal_stress, config)
      risk_index = Enum.max([stress_index, fracture_index])
      safety_factor = config.safety_factor_target / risk_index

      {:ok,
       %{
         "candidate_id" => candidate_id,
         "thermal_shock_status" => if(safety_factor >= 1.0, do: "pass", else: "fail"),
         "thermal_shock_risk_index" => risk_index,
         "thermal_shock_safety_factor" => safety_factor,
         "thermal_shock_temperature_delta" => delta_t,
         "thermal_shock_thermal_strain" => thermal_strain,
         "thermal_shock_estimated_stress" => thermal_stress,
         "thermal_shock_strength_limit" => strength,
         "thermal_shock_stress_index" => stress_index,
         "thermal_shock_fracture_index" => fracture_index,
         "summary" => summary
       }}
    else
      nil -> {:error, :missing_material_thermal_shock_property}
      false -> {:error, :invalid_material_thermal_shock_property}
    end
  end

  defp temperature_delta(summary, config) do
    lookup_number(summary, config.temperature_delta_field) ||
      with max_temp when is_number(max_temp) <- lookup_number(summary, "max_temperature"),
           min_temp when is_number(min_temp) <- lookup_number(summary, "min_temperature") do
        abs(max_temp - min_temp)
      else
        _ -> nil
      end
  end

  defp thermal_stress(youngs_modulus, thermal_strain, poisson_ratio, config) do
    denominator =
      if poisson_ratio > -1.0 and poisson_ratio < 0.49, do: 1.0 - poisson_ratio, else: 1.0

    youngs_modulus * thermal_strain * config.constraint_factor / denominator
  end

  defp strength(summary, config) do
    lookup_number(summary, config.strength_field) ||
      lookup_number(summary, config.fallback_strength_field)
  end

  defp fracture_index(summary, thermal_stress, config) do
    toughness = lookup_number(summary, config.fracture_toughness_field)
    flaw_size = lookup_number(summary, config.flaw_size_field)

    if is_number(toughness) and toughness > 0.0 and is_number(flaw_size) and flaw_size > 0.0 do
      thermal_stress * :math.sqrt(:math.pi() * flaw_size) / toughness
    else
      0.0
    end
  end

  defp maybe_merge_single_assessment(output, [single]) do
    Map.merge(output, %{
      "material_thermal_shock_status" => single["thermal_shock_status"],
      "material_thermal_shock_risk_index" => single["thermal_shock_risk_index"],
      "material_thermal_shock_safety_factor" => single["thermal_shock_safety_factor"],
      "material_thermal_shock_estimated_stress" => single["thermal_shock_estimated_stress"]
    })
  end

  defp maybe_merge_single_assessment(output, _assessments), do: output

  defp candidate_entries(%{"candidates" => candidates}) when is_map(candidates) do
    candidates
    |> Enum.filter(fn {_id, value} -> is_map(value) end)
    |> Enum.map(fn {id, summary} -> {id, summary} end)
  end

  defp candidate_entries(payload) when is_map(payload), do: [{"summary", payload}]

  defp numeric_config(config, field, default) do
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
