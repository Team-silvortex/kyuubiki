defmodule KyuubikiWeb.WorkflowDomainQualityRuntime do
  @moduledoc false

  @domains %{
    "transform.score_acoustic_quality" => %{
      id: "acoustic",
      label: "Acoustic",
      ready: 7.0,
      terms: [
        {"max_sound_pressure_level_db", "Peak sound pressure level", 85.0, 3.0, :min},
        {"max_acoustic_intensity", "Peak acoustic intensity", 0.25, 2.0, :min},
        {"max_pressure_amplitude", "Peak pressure amplitude", 1.0, 1.0, :min},
        {"total_damping_loss", "Damping loss", 0.1, 1.0, :max}
      ]
    },
    "transform.score_modal_quality" => %{
      id: "modal",
      label: "Modal",
      ready: 8.0,
      terms: [
        {"min_frequency_hz", "First natural frequency", 20.0, 4.0, :max},
        {"total_mass", "Total modal mass", 25.0, 1.0, :min},
        {"mode_1_participation_norm", "Mode 1 participation", 2.0, 1.0, :min},
        {"frequency_span_hz", "Modal frequency spread", 250.0, 0.5, :min}
      ]
    },
    "transform.score_structural_quality" => %{
      id: "structural",
      label: "Structural",
      ready: 8.0,
      terms: [
        {"max_displacement", "Maximum displacement", 0.02, 3.0, :min},
        {"max_stress", "Maximum stress", 250.0, 3.0, :min},
        {"mass", "Mass", 15.0, 1.0, :min},
        {"stiffness_margin", "Stiffness margin", 1.2, 1.0, :max}
      ]
    },
    "transform.score_thermal_quality" => %{
      id: "thermal",
      label: "Thermal",
      ready: 8.0,
      terms: [
        {"thermal_temperature_max", "Peak thermal temperature", 120.0, 3.0, :min},
        {"thermo_temperature_delta_max", "Peak thermo-mechanical temperature delta", 80.0, 2.0,
         :min},
        {"thermal_flux_peak_magnitude", "Peak heat flux magnitude", 20.0, 2.0, :min},
        {"thermo_stress_peak", "Peak thermo-mechanical stress", 250.0, 1.0, :min}
      ]
    },
    "transform.score_electrostatic_quality" => %{
      id: "electrostatic",
      label: "Electrostatic",
      ready: 8.0,
      terms: [
        {"electrostatic_field_peak_magnitude", "Peak electric field magnitude", 10.0, 4.0, :min},
        {"electrostatic_peak_energy_density", "Peak electrostatic energy density", 0.8, 2.0,
         :min},
        {"electrostatic_potential_span", "Potential span", 4.0, 1.0, :max}
      ]
    },
    "transform.score_magnetostatic_quality" => %{
      id: "magnetostatic",
      label: "Magnetostatic",
      ready: 8.0,
      terms: [
        {"magnetostatic_field_peak_magnitude", "Peak magnetic field strength", 12.0, 3.0, :min},
        {"magnetostatic_flux_peak_magnitude", "Peak magnetic flux density", 16.0, 2.0, :min},
        {"magnetostatic_energy_density_peak", "Peak magnetic energy density", 8.0, 2.0, :min},
        {"magnetostatic_current_density_sum", "Current density sum", 10.0, 1.0, :min}
      ]
    },
    "transform.score_cfd_quality" => %{
      id: "cfd",
      label: "CFD",
      ready: 8.0,
      terms: [
        {"cfd_divergence_error_peak", "Divergence peak", 0.05, 4.0, :min},
        {"cfd_reynolds_number_peak", "Reynolds peak", 10.0, 2.0, :min},
        {"cfd_viscous_dissipation_total", "Viscous dissipation", 1.0, 1.0, :min},
        {"cfd_velocity_span", "Velocity span", 2.0, 0.5, :min},
        {"cfd_pressure_span", "Pressure span", 5.0, 0.5, :min}
      ]
    },
    "transform.score_transport_quality" => %{
      id: "transport",
      label: "Transport",
      ready: 8.0,
      terms: [
        {"transport_total_flux_peak_magnitude", "Peak total transport flux magnitude", 1.5, 3.0,
         :min},
        {"transport_peclet_peak", "Peak Peclet number", 200.0, 2.0, :min},
        {"transport_concentration_span", "Concentration span", 1.0, 2.0, :min},
        {"transport_source_sum", "Net source balance", 2.0, 1.0, :min}
      ]
    }
  }

  def supported_operator_ids, do: Map.keys(@domains)

  def score(operator_id, payload, config) when is_map(payload) and is_map(config) do
    with {:ok, domain} <- fetch_domain(operator_id) do
      terms = Enum.map(domain.terms, &score_term(payload, config, &1))
      missing = Enum.count(terms, &(Map.get(&1, "status") == "missing"))
      score = Enum.reduce(terms, 0.0, &(&2 + Map.get(&1, "penalty", 0.0)))
      max_ready = config_number(config, "max_ready_score", domain.ready)
      grade = quality_grade(score, missing, max_ready)
      {:ok, quality_payload(domain, terms, score, missing, max_ready, grade)}
    end
  end

  def score(_operator_id, _payload, _config), do: {:error, :invalid_quality_score_payload}

  defp fetch_domain(operator_id) do
    case Map.fetch(@domains, operator_id) do
      {:ok, domain} -> {:ok, domain}
      :error -> {:error, {:unsupported_quality_score_operator, operator_id}}
    end
  end

  defp score_term(payload, config, {field, label, default_target, default_weight, goal}) do
    target = configured_number(config, "targets", field, default_target) |> max(1.0e-12)
    weight = configured_number(config, "weights", field, default_weight) |> max(0.0)

    case metric_value(payload, field) do
      value when is_number(value) ->
        penalty = quality_ratio(value, target, goal) * weight

        %{
          "field" => field,
          "label" => label,
          "value" => value,
          "target" => target,
          "weight" => weight,
          "goal" => Atom.to_string(goal),
          "penalty" => penalty,
          "status" => if(meets_target?(value, target, goal), do: "ok", else: "watch")
        }

      _ ->
        %{
          "field" => field,
          "label" => label,
          "target" => target,
          "weight" => weight,
          "penalty" => 0.0,
          "status" => "missing"
        }
    end
  end

  defp metric_value(payload, "frequency_span_hz") do
    number(payload["frequency_span_hz"]) ||
      with min when is_number(min) <- number(payload["min_frequency_hz"]),
           max when is_number(max) <- number(payload["max_frequency_hz"]) do
        max - min
      else
        _ -> nil
      end
  end

  defp metric_value(payload, field) do
    number(payload[field]) || span_value(payload, field)
  end

  defp span_value(payload, field) do
    with true <- String.ends_with?(field, "_span"),
         prefix <- String.replace_suffix(field, "_span", ""),
         min when is_number(min) <- number(payload["#{prefix}_min"]),
         max when is_number(max) <- number(payload["#{prefix}_max"]) do
      max - min
    else
      _ -> nil
    end
  end

  defp quality_payload(domain, terms, score, missing, max_ready, grade) do
    id = domain.id

    %{
      "#{id}_quality_contract" => "kyuubiki.#{id}_quality_score/v1",
      "#{id}_quality_score" => score,
      "#{id}_quality_grade" => grade,
      "#{id}_quality_ready" => grade != "block",
      "#{id}_quality_missing_metric_count" => missing,
      "#{id}_quality_term_count" => length(terms),
      "#{id}_quality_max_ready_score" => max_ready,
      "#{id}_quality_terms" => terms,
      "#{id}_quality_summary" =>
        "#{domain.label} quality #{grade}: score=#{format_score(score)}, missing=#{missing}, ready_limit=#{format_score(max_ready)}."
    }
  end

  defp quality_ratio(value, target, :max), do: target / max(abs(value), 1.0e-12)
  defp quality_ratio(value, target, :min), do: abs(value) / target

  defp meets_target?(value, target, :max), do: value >= target
  defp meets_target?(value, target, :min), do: abs(value) <= target

  defp configured_number(config, group, field, default) do
    config
    |> get_in([group, field])
    |> number()
    |> Kernel.||(default)
  end

  defp config_number(config, field, default) do
    case number(config[field]) do
      value when is_number(value) and value >= 0.0 -> value
      _ -> default
    end
  end

  defp number(value) when is_number(value) do
    if finite?(value), do: value * 1.0
  end

  defp number(_value), do: nil

  defp finite?(value), do: value == value and value not in [:infinity, :neg_infinity]

  defp quality_grade(score, missing, max_ready) do
    cond do
      missing > 0 or score > max_ready -> "block"
      score > max_ready * 0.7 -> "review"
      score > max_ready * 0.35 -> "good"
      true -> "excellent"
    end
  end

  defp format_score(value), do: :erlang.float_to_binary(value * 1.0, decimals: 4)
end
