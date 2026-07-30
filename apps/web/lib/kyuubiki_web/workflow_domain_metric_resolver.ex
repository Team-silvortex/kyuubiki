defmodule KyuubikiWeb.WorkflowDomainMetricResolver do
  @moduledoc false

  @dynamic_amplitude_fields ~w(max_displacement max_velocity max_acceleration max_force)

  def metric_value(payload, "frequency_span_hz") when is_map(payload) do
    number(payload["frequency_span_hz"]) ||
      domain_alias_value(payload, "frequency_span_hz") ||
      with min when is_number(min) <- metric_value(payload, "min_frequency_hz"),
           max when is_number(max) <- metric_value(payload, "max_frequency_hz") do
        abs(max - min)
      else
        _ ->
          case mode_frequency_bounds(payload) do
            {min, max} -> abs(max - min)
            nil -> nil
          end
      end
  end

  def metric_value(payload, field) when is_map(payload) and is_binary(field) do
    number(payload[field]) || dynamic_alias_value(payload, field) ||
      domain_alias_value(payload, field) ||
      derived_dynamic_value(payload, field) || derived_domain_value(payload, field) ||
      span_value(payload, field)
  end

  def metric_value(_payload, _field), do: nil

  def fetch_metric(payload, field) do
    case metric_value(payload, field) do
      value when is_number(value) -> {:ok, value}
      _ -> :error
    end
  end

  defp dynamic_alias_value(payload, "peak_frequency_hz") do
    first_number(payload, [
      "response_peak_frequency_hz",
      "dominant_frequency_hz",
      "freq_peak_hz"
    ])
  end

  defp dynamic_alias_value(payload, "max_displacement") do
    first_number(payload, ["peak_displacement", "displacement_amplitude_peak", "u_max"])
  end

  defp dynamic_alias_value(payload, "max_velocity") do
    first_number(payload, ["peak_velocity", "velocity_amplitude_peak", "v_max"])
  end

  defp dynamic_alias_value(payload, "max_acceleration") do
    first_number(payload, ["peak_acceleration", "acceleration_amplitude_peak", "a_max"])
  end

  defp dynamic_alias_value(payload, "max_force") do
    first_number(payload, ["peak_force", "force_amplitude_peak", "dynamic_force_peak"])
  end

  defp dynamic_alias_value(_payload, _field), do: nil

  defp domain_alias_value(payload, "max_displacement") do
    first_number(payload, ["peak_displacement", "max_translation", "displacement_peak", "u_max"])
  end

  defp domain_alias_value(payload, "max_stress") do
    first_number(payload, [
      "peak_stress",
      "von_mises_peak",
      "max_von_mises_stress",
      "stress_peak"
    ])
  end

  defp domain_alias_value(payload, "mass") do
    first_number(payload, ["total_mass", "structure_mass", "model_mass"])
  end

  defp domain_alias_value(payload, "stiffness_margin") do
    first_number(payload, [
      "minimum_stiffness_margin",
      "min_stiffness_margin",
      "stability_margin"
    ])
  end

  defp domain_alias_value(payload, "thermal_temperature_max") do
    first_number(payload, ["max_temperature", "temperature_max", "peak_temperature"])
  end

  defp domain_alias_value(payload, "thermal_flux_peak_magnitude") do
    first_number(payload, ["max_heat_flux", "heat_flux_peak", "peak_heat_flux"])
  end

  defp domain_alias_value(payload, "thermo_temperature_delta_max") do
    first_number(payload, [
      "max_temperature_delta",
      "temperature_delta_max",
      "peak_temperature_delta"
    ])
  end

  defp domain_alias_value(payload, "thermo_stress_peak") do
    first_number(payload, ["max_stress", "peak_stress", "thermal_stress_peak"])
  end

  defp domain_alias_value(payload, "thermal_total_energy") do
    first_number(payload, [
      "total_thermal_energy",
      "thermal_energy_total",
      "total_heat_energy"
    ])
  end

  defp domain_alias_value(payload, "electrostatic_field_peak_magnitude") do
    first_number(payload, ["max_electric_field", "peak_electric_field", "electric_field_peak"])
  end

  defp domain_alias_value(payload, "electrostatic_peak_energy_density") do
    first_number(payload, [
      "max_energy_density",
      "peak_energy_density",
      "electric_energy_density_peak"
    ])
  end

  defp domain_alias_value(payload, "electrostatic_flux_peak_magnitude") do
    first_number(payload, ["max_flux_density", "peak_flux_density"])
  end

  defp domain_alias_value(payload, "electrostatic_total_stored_energy") do
    first_number(payload, [
      "total_stored_energy",
      "stored_energy_total",
      "electric_total_energy"
    ])
  end

  defp domain_alias_value(payload, "electrostatic_potential_span") do
    first_number(payload, ["potential_span", "voltage_span", "max_potential"])
  end

  defp domain_alias_value(payload, "magnetostatic_field_peak_magnitude") do
    first_number(payload, [
      "max_magnetic_field_strength",
      "peak_magnetic_field_strength",
      "h_peak"
    ])
  end

  defp domain_alias_value(payload, "magnetostatic_flux_peak_magnitude") do
    first_number(payload, ["max_flux_density", "peak_flux_density", "b_peak"])
  end

  defp domain_alias_value(payload, "magnetostatic_energy_density_peak") do
    first_number(payload, [
      "max_energy_density",
      "peak_energy_density",
      "magnetic_energy_density_peak"
    ])
  end

  defp domain_alias_value(payload, "magnetostatic_current_density_sum") do
    first_number(payload, [
      "total_current_density",
      "current_density_total",
      "sum_current_density"
    ])
  end

  defp domain_alias_value(payload, "magnetostatic_total_stored_energy") do
    first_number(payload, [
      "total_stored_energy",
      "stored_energy_total",
      "magnetic_total_energy"
    ])
  end

  defp domain_alias_value(payload, "max_sound_pressure_level_db") do
    first_number(payload, ["peak_spl_db", "spl_max_db", "sound_pressure_level_max_db"])
  end

  defp domain_alias_value(payload, "max_acoustic_intensity") do
    first_number(payload, [
      "peak_acoustic_intensity",
      "acoustic_intensity_peak",
      "intensity_max"
    ])
  end

  defp domain_alias_value(payload, "max_pressure_amplitude") do
    first_number(payload, ["max_pressure", "peak_pressure", "pressure_amplitude_peak"])
  end

  defp domain_alias_value(payload, "total_damping_loss") do
    first_number(payload, [
      "damping_loss_total",
      "total_acoustic_damping_loss",
      "damping_energy_loss"
    ])
  end

  defp domain_alias_value(payload, "min_frequency_hz") do
    first_number(payload, [
      "first_frequency_hz",
      "natural_frequency_min_hz",
      "mode_1_frequency_hz"
    ])
  end

  defp domain_alias_value(payload, "max_frequency_hz") do
    first_number(payload, [
      "last_frequency_hz",
      "natural_frequency_max_hz",
      "modal_frequency_max_hz"
    ])
  end

  defp domain_alias_value(payload, "total_mass") do
    first_number(payload, ["modal_mass_total", "participating_mass_total", "mass_total"])
  end

  defp domain_alias_value(payload, "frequency_span_hz") do
    first_number(payload, ["modal_frequency_span_hz", "natural_frequency_span_hz"])
  end

  defp domain_alias_value(payload, "mode_1_participation_norm") do
    first_number(payload, [
      "first_mode_participation_norm",
      "mode1_participation_norm",
      "primary_mode_participation_norm"
    ])
  end

  defp domain_alias_value(payload, "cfd_divergence_error_peak") do
    first_number(payload, ["max_divergence_error", "divergence_peak", "div_u_peak"])
  end

  defp domain_alias_value(payload, "cfd_reynolds_number_peak") do
    first_number(payload, ["max_reynolds_number", "reynolds_peak", "re_peak"])
  end

  defp domain_alias_value(payload, "cfd_viscous_dissipation_total") do
    first_number(payload, [
      "total_viscous_dissipation",
      "viscous_dissipation_sum",
      "dissipation_total"
    ])
  end

  defp domain_alias_value(payload, "cfd_velocity_span") do
    first_number(payload, ["velocity_span", "speed_span"])
  end

  defp domain_alias_value(payload, "cfd_pressure_span") do
    first_number(payload, ["pressure_span", "p_span"])
  end

  defp domain_alias_value(payload, "velocity_magnitude") do
    first_number(payload, ["speed", "u_mag"]) || vector_magnitude(payload, "vx", "vy")
  end

  defp domain_alias_value(payload, "pressure") do
    first_number(payload, ["p", "static_pressure"])
  end

  defp domain_alias_value(payload, "divergence_error") do
    first_number(payload, ["div_u", "divergence"])
  end

  defp domain_alias_value(payload, "reynolds_number") do
    first_number(payload, ["reynolds", "re"])
  end

  defp domain_alias_value(payload, "viscous_dissipation") do
    first_number(payload, ["dissipation", "nu_dissipation"])
  end

  defp domain_alias_value(payload, "transport_total_flux_peak_magnitude") do
    first_number(payload, ["max_transport_flux", "peak_transport_flux", "transport_flux_peak"])
  end

  defp domain_alias_value(payload, "transport_peclet_peak") do
    first_number(payload, ["max_peclet", "peclet_max", "peak_peclet"])
  end

  defp domain_alias_value(payload, "transport_concentration_span") do
    first_number(payload, ["concentration_span", "concentration_range", "species_span"])
  end

  defp domain_alias_value(payload, "transport_source_sum") do
    first_number(payload, ["net_source", "source_balance", "total_source", "source_sum"])
  end

  defp domain_alias_value(_payload, _field), do: nil

  defp derived_dynamic_value(payload, "peak_frequency_hz") do
    payload |> Map.get("frequencies") |> peak_frequency()
  end

  defp derived_dynamic_value(payload, field) when field in @dynamic_amplitude_fields do
    payload
    |> Map.get("frequencies")
    |> max_frequency_field(field)
    |> Kernel.||(max_transient_node_field(payload, field))
  end

  defp derived_dynamic_value(_payload, _field), do: nil

  defp derived_domain_value(payload, "thermo_temperature_delta_max") do
    bounds_delta(payload, ["temperature_max", "max_temperature"], [
      "temperature_min",
      "min_temperature"
    ])
  end

  defp derived_domain_value(payload, "electrostatic_potential_span") do
    bounds_delta(payload, ["potential_max", "max_voltage", "voltage_max"], [
      "potential_min",
      "min_voltage",
      "voltage_min"
    ])
  end

  defp derived_domain_value(payload, "transport_concentration_span") do
    bounds_delta(payload, ["concentration_max", "max_concentration"], [
      "concentration_min",
      "min_concentration"
    ])
  end

  defp derived_domain_value(payload, "min_frequency_hz") do
    case mode_frequency_bounds(payload) do
      {min, _max} -> min
      nil -> nil
    end
  end

  defp derived_domain_value(payload, "max_frequency_hz") do
    case mode_frequency_bounds(payload) do
      {_min, max} -> max
      nil -> nil
    end
  end

  defp derived_domain_value(payload, "mode_1_participation_norm") do
    with modes when is_list(modes) <- Map.get(payload, "modes"),
         first_mode when is_map(first_mode) <- List.first(modes) do
      number(first_mode["participation_norm"])
    else
      _ -> nil
    end
  end

  defp derived_domain_value(_payload, _field), do: nil

  defp bounds_delta(payload, max_fields, min_fields) do
    with max when is_number(max) <- first_number(payload, max_fields),
         min when is_number(min) <- first_number(payload, min_fields) do
      abs(max - min)
    else
      _ -> nil
    end
  end

  defp mode_frequency_bounds(payload) do
    case Map.get(payload, "modes") do
      modes when is_list(modes) ->
        modes
        |> Enum.filter(&is_map/1)
        |> Enum.flat_map(fn mode ->
          case number(mode["frequency_hz"]) do
            frequency when is_number(frequency) -> [frequency]
            _ -> []
          end
        end)
        |> case do
          [] -> nil
          frequencies -> {Enum.min(frequencies), Enum.max(frequencies)}
        end

      _ ->
        nil
    end
  end

  defp peak_frequency(frequencies) when is_list(frequencies) do
    frequencies
    |> Enum.filter(&is_map/1)
    |> Enum.flat_map(fn entry ->
      with frequency when is_number(frequency) <- frequency_entry_number(entry, "frequency_hz"),
           displacement when is_number(displacement) <-
             frequency_entry_number(entry, "max_displacement") do
        [{frequency, displacement}]
      else
        _ -> []
      end
    end)
    |> Enum.max_by(fn {_frequency, displacement} -> displacement end, fn -> nil end)
    |> case do
      {frequency, _displacement} -> frequency
      nil -> nil
    end
  end

  defp peak_frequency(_frequencies), do: nil

  defp max_frequency_field(frequencies, field) when is_list(frequencies) do
    frequencies
    |> Enum.filter(&is_map/1)
    |> Enum.flat_map(fn entry ->
      case frequency_entry_number(entry, field) do
        value when is_number(value) -> [value]
        _ -> []
      end
    end)
    |> Enum.max(fn -> nil end)
  end

  defp max_frequency_field(_frequencies, _field), do: nil

  defp frequency_entry_number(entry, "frequency_hz") do
    number(entry["frequency_hz"]) ||
      first_number(entry, ["frequency", "freq_hz", "response_frequency_hz"])
  end

  defp frequency_entry_number(entry, "max_displacement") do
    number(entry["max_displacement"]) ||
      first_number(entry, ["peak_displacement", "displacement_amplitude", "u_peak"])
  end

  defp frequency_entry_number(entry, "max_velocity") do
    number(entry["max_velocity"]) ||
      first_number(entry, ["peak_velocity", "velocity_amplitude", "v_peak"])
  end

  defp frequency_entry_number(entry, "max_acceleration") do
    number(entry["max_acceleration"]) ||
      first_number(entry, ["peak_acceleration", "acceleration_amplitude", "a_peak"])
  end

  defp frequency_entry_number(entry, "max_force") do
    number(entry["max_force"]) ||
      first_number(entry, ["peak_force", "force_amplitude", "dynamic_force_peak"])
  end

  defp frequency_entry_number(_entry, _field), do: nil

  defp max_transient_node_field(payload, field) do
    with node_field when is_binary(node_field) <- transient_node_field(field),
         nodes when is_list(nodes) <- Map.get(payload, "nodes") do
      nodes
      |> Enum.filter(&is_map/1)
      |> Enum.flat_map(fn node ->
        case number(node[node_field]) do
          value when is_number(value) -> [abs(value)]
          _ -> []
        end
      end)
      |> Enum.max(fn -> nil end)
    else
      _ -> nil
    end
  end

  defp transient_node_field("max_displacement"), do: "ux"
  defp transient_node_field("max_velocity"), do: "vx"
  defp transient_node_field("max_acceleration"), do: "ax"
  defp transient_node_field(_field), do: nil

  defp first_number(map, fields) when is_map(map) do
    Enum.find_value(fields, fn field -> number(map[field]) end)
  end

  defp first_number(_map, _fields), do: nil

  defp vector_magnitude(map, x_field, y_field) when is_map(map) do
    with x when is_number(x) <- number(map[x_field]),
         y when is_number(y) <- number(map[y_field]) do
      :math.sqrt(x * x + y * y)
    else
      _ -> nil
    end
  end

  defp vector_magnitude(_map, _x_field, _y_field), do: nil

  defp span_value(payload, field) do
    with true <- String.ends_with?(field, "_span"),
         prefix <- String.replace_suffix(field, "_span", ""),
         min when is_number(min) <- number(payload["#{prefix}_min"]),
         max when is_number(max) <- number(payload["#{prefix}_max"]) do
      abs(max - min)
    else
      _ -> nil
    end
  end

  defp number(value) when is_number(value) do
    if finite?(value), do: value * 1.0
  end

  defp number(_value), do: nil

  defp finite?(value), do: value == value and value not in [:infinity, :neg_infinity]
end
