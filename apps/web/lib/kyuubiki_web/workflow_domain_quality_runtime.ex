defmodule KyuubikiWeb.WorkflowDomainQualityRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDomainMetricResolver

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
    "transform.score_dynamic_quality" => %{
      id: "dynamic",
      label: "Dynamic",
      ready: 8.0,
      terms: [
        {"peak_frequency_hz", "Peak response frequency", 20.0, 3.0, :max},
        {"max_displacement", "Peak displacement amplitude", 0.02, 3.0, :min},
        {"max_acceleration", "Peak acceleration amplitude", 250.0, 1.5, :min},
        {"max_force", "Peak dynamic force", 5000.0, 1.0, :min}
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
      terms = terms_for(domain, config) |> Enum.map(&score_term(payload, config, &1))
      missing = Enum.count(terms, &(Map.get(&1, "status") == "missing"))
      watch = Enum.count(terms, &(Map.get(&1, "status") == "watch"))
      score = Enum.reduce(terms, 0.0, &(&2 + Map.get(&1, "penalty", 0.0)))
      max_ready = config_number(config, "max_ready_score", domain.ready)
      grade = quality_grade(score, missing, max_ready)
      {:ok, quality_payload(domain, payload, terms, score, missing, watch, max_ready, grade)}
    end
  end

  def score(_operator_id, _payload, _config), do: {:error, :invalid_quality_score_payload}

  defp fetch_domain(operator_id) do
    case Map.fetch(@domains, operator_id) do
      {:ok, domain} -> {:ok, domain}
      :error -> {:error, {:unsupported_quality_score_operator, operator_id}}
    end
  end

  defp terms_for(domain, config) do
    case config["enabled_terms"] do
      enabled_terms when is_list(enabled_terms) ->
        enabled_terms
        |> Enum.filter(&is_binary/1)
        |> Enum.map(&quality_term_for(domain, &1))
        |> Enum.reject(&is_nil/1)
        |> case do
          [] -> domain.terms
          selected_terms -> selected_terms
        end

      _ ->
        domain.terms
    end
  end

  defp quality_term_for(domain, field) do
    Enum.find(domain.terms, fn {candidate, _label, _target, _weight, _goal} ->
      candidate == field
    end) || extra_quality_term(domain.id, field)
  end

  defp extra_quality_term("dynamic", "max_velocity"),
    do: {"max_velocity", "Peak velocity amplitude", 2.0, 1.0, :min}

  defp extra_quality_term("thermal", "thermal_total_energy"),
    do: {"thermal_total_energy", "Total thermal energy", 5000.0, 1.0, :min}

  defp extra_quality_term("electrostatic", "electrostatic_total_stored_energy"),
    do:
      {"electrostatic_total_stored_energy", "Total electrostatic stored energy", 10.0, 1.0, :min}

  defp extra_quality_term("magnetostatic", "magnetostatic_total_stored_energy"),
    do:
      {"magnetostatic_total_stored_energy", "Total magnetostatic stored energy", 10.0, 1.0, :min}

  defp extra_quality_term(_domain_id, _field), do: nil

  defp score_term(payload, config, {field, label, default_target, default_weight, goal}) do
    target = configured_number(config, "targets", field, default_target) |> max(1.0e-12)
    weight = configured_number(config, "weights", field, default_weight) |> max(0.0)

    case WorkflowDomainMetricResolver.metric_value(payload, field) do
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

  defp quality_payload(domain, payload, terms, score, missing, watch, max_ready, grade) do
    id = domain.id

    %{
      "#{id}_quality_contract" => "kyuubiki.#{id}_quality_score/v1",
      "#{id}_quality_score" => score,
      "#{id}_quality_grade" => grade,
      "#{id}_quality_ready" => grade != "block",
      "#{id}_quality_missing_metric_count" => missing,
      "#{id}_quality_watch_count" => watch,
      "#{id}_quality_term_count" => length(terms),
      "#{id}_quality_max_ready_score" => max_ready,
      "#{id}_quality_dominant_term" => dominant_quality_term(terms),
      "#{id}_quality_blocking_terms" => blocking_terms(terms, grade),
      "#{id}_quality_terms" => terms,
      "#{id}_quality_summary" =>
        "#{domain.label} quality #{grade}: score=#{format_score(score)}, missing=#{missing}, watch=#{watch}, ready_limit=#{format_score(max_ready)}."
    }
    |> Map.merge(domain_metric_payload(id, payload))
  end

  defp domain_metric_payload("dynamic", payload) do
    metric_payload(payload, [
      {"dynamic_quality_peak_frequency_hz", "peak_frequency_hz"},
      {"dynamic_quality_max_displacement", "max_displacement"},
      {"dynamic_quality_max_velocity", "max_velocity"},
      {"dynamic_quality_max_acceleration", "max_acceleration"},
      {"dynamic_quality_max_force", "max_force"}
    ])
  end

  defp domain_metric_payload("structural", payload) do
    metric_payload(payload, [
      {"structural_quality_max_displacement", "max_displacement"},
      {"structural_quality_max_stress", "max_stress"},
      {"structural_quality_mass", "mass"},
      {"structural_quality_stiffness_margin", "stiffness_margin"}
    ])
  end

  defp domain_metric_payload("thermal", payload) do
    metric_payload(payload, [
      {"thermal_quality_max_temperature", "thermal_temperature_max"},
      {"thermal_quality_temperature_delta", "thermo_temperature_delta_max"},
      {"thermal_quality_peak_flux_magnitude", "thermal_flux_peak_magnitude"},
      {"thermal_quality_peak_stress", "thermo_stress_peak"},
      {"thermal_quality_total_energy", "thermal_total_energy"}
    ])
  end

  defp domain_metric_payload("electrostatic", payload) do
    metric_payload(payload, [
      {"electrostatic_quality_peak_field", "electrostatic_field_peak_magnitude"},
      {"electrostatic_quality_peak_energy_density", "electrostatic_peak_energy_density"},
      {"electrostatic_quality_potential_span", "electrostatic_potential_span"},
      {"electrostatic_quality_total_energy", "electrostatic_total_stored_energy"}
    ])
  end

  defp domain_metric_payload("magnetostatic", payload) do
    metric_payload(payload, [
      {"magnetostatic_quality_peak_field", "magnetostatic_field_peak_magnitude"},
      {"magnetostatic_quality_peak_flux", "magnetostatic_flux_peak_magnitude"},
      {"magnetostatic_quality_peak_energy_density", "magnetostatic_energy_density_peak"},
      {"magnetostatic_quality_current_density_sum", "magnetostatic_current_density_sum"},
      {"magnetostatic_quality_total_energy", "magnetostatic_total_stored_energy"}
    ])
  end

  defp domain_metric_payload("acoustic", payload) do
    metric_payload(payload, [
      {"acoustic_quality_max_spl_db", "max_sound_pressure_level_db"},
      {"acoustic_quality_max_intensity", "max_acoustic_intensity"},
      {"acoustic_quality_max_pressure", "max_pressure_amplitude"},
      {"acoustic_quality_total_damping_loss", "total_damping_loss"}
    ])
  end

  defp domain_metric_payload("modal", payload) do
    metric_payload(payload, [
      {"modal_quality_min_frequency_hz", "min_frequency_hz"},
      {"modal_quality_total_mass", "total_mass"},
      {"modal_quality_frequency_span_hz", "frequency_span_hz"},
      {"modal_quality_mode_1_participation_norm", "mode_1_participation_norm"}
    ])
  end

  defp domain_metric_payload("cfd", payload) do
    metric_payload(payload, [
      {"cfd_quality_divergence_error_peak", "cfd_divergence_error_peak"},
      {"cfd_quality_reynolds_number_peak", "cfd_reynolds_number_peak"},
      {"cfd_quality_viscous_dissipation_total", "cfd_viscous_dissipation_total"},
      {"cfd_quality_velocity_span", "cfd_velocity_span"},
      {"cfd_quality_pressure_span", "cfd_pressure_span"}
    ])
  end

  defp domain_metric_payload("transport", payload) do
    metric_payload(payload, [
      {"transport_quality_peak_flux_magnitude", "transport_total_flux_peak_magnitude"},
      {"transport_quality_peak_peclet", "transport_peclet_peak"},
      {"transport_quality_concentration_span", "transport_concentration_span"},
      {"transport_quality_source_sum", "transport_source_sum"}
    ])
  end

  defp domain_metric_payload(_id, _payload), do: %{}

  defp metric_payload(payload, pairs) do
    Map.new(pairs, fn {output_field, metric_field} ->
      {output_field, WorkflowDomainMetricResolver.metric_value(payload, metric_field)}
    end)
  end

  defp quality_ratio(value, target, :max), do: target / max(abs(value), 1.0e-12)
  defp quality_ratio(value, target, :min), do: abs(value) / target

  defp meets_target?(value, target, :max), do: value >= target
  defp meets_target?(value, target, :min), do: abs(value) <= target

  defp dominant_quality_term(terms) do
    terms
    |> Enum.max_by(&Map.get(&1, "penalty", 0.0), fn -> nil end)
    |> compact_quality_term()
  end

  defp blocking_terms(terms, "block") do
    terms
    |> Enum.filter(&(Map.get(&1, "status") in ["missing", "watch"]))
    |> Enum.map(&compact_quality_term/1)
  end

  defp blocking_terms(_terms, _grade), do: []

  defp compact_quality_term(nil), do: nil

  defp compact_quality_term(term) do
    %{
      "field" => Map.get(term, "field"),
      "label" => Map.get(term, "label"),
      "status" => Map.get(term, "status"),
      "penalty" => Map.get(term, "penalty")
    }
  end

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
