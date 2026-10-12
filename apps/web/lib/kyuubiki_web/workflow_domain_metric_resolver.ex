defmodule KyuubikiWeb.WorkflowDomainMetricResolver do
  @moduledoc false

  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  @aliases %{
    "peak_frequency_hz" => ~w(response_peak_frequency_hz dominant_frequency_hz freq_peak_hz),
    "max_displacement" =>
      ~w(peak_displacement displacement_amplitude_peak u_max max_translation displacement_peak),
    "max_velocity" => ~w(peak_velocity velocity_amplitude_peak v_max),
    "max_acceleration" => ~w(peak_acceleration acceleration_amplitude_peak a_max),
    "max_force" => ~w(peak_force force_amplitude_peak dynamic_force_peak),
    "max_stress" => ~w(peak_stress von_mises_peak max_von_mises_stress stress_peak),
    "mass" => ~w(total_mass structure_mass model_mass),
    "stiffness_margin" => ~w(minimum_stiffness_margin min_stiffness_margin stability_margin),
    "thermal_temperature_max" => ~w(max_temperature temperature_max peak_temperature),
    "thermal_flux_peak_magnitude" => ~w(max_heat_flux heat_flux_peak peak_heat_flux),
    "thermo_temperature_delta_max" =>
      ~w(max_temperature_delta temperature_delta_max peak_temperature_delta),
    "thermo_stress_peak" => ~w(max_stress peak_stress thermal_stress_peak),
    "thermal_total_energy" => ~w(total_thermal_energy thermal_energy_total total_heat_energy),
    "electrostatic_field_peak_magnitude" =>
      ~w(max_electric_field peak_electric_field electric_field_peak),
    "electrostatic_peak_energy_density" =>
      ~w(max_energy_density peak_energy_density electric_energy_density_peak),
    "electrostatic_flux_peak_magnitude" => ~w(max_flux_density peak_flux_density),
    "electrostatic_total_stored_energy" =>
      ~w(total_stored_energy stored_energy_total electric_total_energy),
    "electrostatic_potential_span" => ~w(potential_span voltage_span max_potential),
    "magnetostatic_field_peak_magnitude" =>
      ~w(max_magnetic_field_strength peak_magnetic_field_strength h_peak),
    "magnetostatic_flux_peak_magnitude" => ~w(max_flux_density peak_flux_density b_peak),
    "magnetostatic_energy_density_peak" =>
      ~w(max_energy_density peak_energy_density magnetic_energy_density_peak),
    "magnetostatic_current_density_sum" =>
      ~w(total_current_density current_density_total sum_current_density),
    "magnetostatic_total_stored_energy" =>
      ~w(total_stored_energy stored_energy_total magnetic_total_energy),
    "max_sound_pressure_level_db" => ~w(peak_spl_db spl_max_db sound_pressure_level_max_db),
    "max_acoustic_intensity" => ~w(peak_acoustic_intensity acoustic_intensity_peak intensity_max),
    "max_pressure_amplitude" => ~w(max_pressure peak_pressure pressure_amplitude_peak),
    "total_damping_loss" =>
      ~w(damping_loss_total total_acoustic_damping_loss damping_energy_loss),
    "min_frequency_hz" => ~w(first_frequency_hz natural_frequency_min_hz mode_1_frequency_hz),
    "max_frequency_hz" => ~w(last_frequency_hz natural_frequency_max_hz modal_frequency_max_hz),
    "total_mass" => ~w(modal_mass_total participating_mass_total mass_total),
    "frequency_span_hz" => ~w(modal_frequency_span_hz natural_frequency_span_hz),
    "mode_1_participation_norm" =>
      ~w(first_mode_participation_norm mode1_participation_norm primary_mode_participation_norm),
    "cfd_divergence_error_peak" => ~w(max_divergence_error divergence_peak div_u_peak),
    "cfd_reynolds_number_peak" => ~w(max_reynolds_number reynolds_peak re_peak),
    "cfd_viscous_dissipation_total" =>
      ~w(total_viscous_dissipation viscous_dissipation_sum dissipation_total),
    "cfd_velocity_span" => ~w(velocity_span speed_span),
    "cfd_pressure_span" => ~w(pressure_span p_span),
    "velocity_magnitude" => ~w(speed u_mag),
    "pressure" => ~w(p static_pressure),
    "divergence_error" => ~w(div_u divergence),
    "reynolds_number" => ~w(reynolds re),
    "viscous_dissipation" => ~w(dissipation nu_dissipation),
    "transport_total_flux_peak_magnitude" =>
      ~w(max_transport_flux peak_transport_flux transport_flux_peak),
    "transport_peclet_peak" => ~w(max_peclet peclet_max peak_peclet),
    "transport_concentration_span" => ~w(concentration_span concentration_range species_span),
    "transport_source_sum" => ~w(net_source source_balance total_source source_sum)
  }

  @frequency_aliases %{
    "frequency_hz" => ~w(frequency freq_hz response_frequency_hz),
    "max_displacement" => ~w(peak_displacement displacement_amplitude u_peak),
    "max_velocity" => ~w(peak_velocity velocity_amplitude v_peak),
    "max_acceleration" => ~w(peak_acceleration acceleration_amplitude a_peak),
    "max_force" => ~w(peak_force force_amplitude dynamic_force_peak)
  }
  @transient_fields %{
    "max_displacement" => "ux",
    "max_velocity" => "vx",
    "max_acceleration" => "ax"
  }
  @dynamic_amplitude_fields ~w(max_displacement max_velocity max_acceleration max_force)

  # Display callers may omit an invalid metric; decision admission must retain the error.
  def metric_value(payload, field) do
    case checked_metric_value(payload, field) do
      {:ok, value} -> value
      {:error, _reason} -> nil
    end
  end

  def fetch_metric(payload, field) do
    case checked_metric_value(payload, field) do
      {:ok, value} when is_number(value) -> {:ok, value}
      _ -> :error
    end
  end

  def checked_metric_value(payload, field, path \\ "payload") do
    Check.run("metric", fn ->
      payload = Check.object(payload, path)
      field = Check.text(%{"field" => field}, "field", "metric")
      Check.calculate("#{path}.#{field}", fn -> resolve(payload, field, path) end)
    end)
  end

  defp resolve(payload, field, path) do
    first_number(payload, [field | Map.get(@aliases, field, [])], path) ||
      derived(payload, field, path)
  end

  defp derived(payload, "peak_frequency_hz", path) do
    case collection(payload, "frequencies", path) do
      nil -> nil
      rows -> peak_frequency(rows, "#{path}.frequencies")
    end
  end

  defp derived(payload, field, path) when field in @dynamic_amplitude_fields do
    case collection(payload, "frequencies", path) do
      nil ->
        transient_peak(payload, field, path)

      rows ->
        rows |> sample_bounds("#{path}.frequencies", &frequency_number(&1, field, &2)) |> upper()
    end
  end

  defp derived(payload, "thermo_temperature_delta_max" = field, path) do
    bounds_delta(
      payload,
      ~w(temperature_min min_temperature),
      ~w(temperature_max max_temperature),
      "#{path}.#{field}",
      path
    )
  end

  defp derived(payload, "electrostatic_potential_span" = field, path) do
    bounds_delta(
      payload,
      ~w(potential_min min_voltage voltage_min),
      ~w(potential_max max_voltage voltage_max),
      "#{path}.#{field}",
      path
    )
  end

  defp derived(payload, "transport_concentration_span" = field, path) do
    bounds_delta(
      payload,
      ~w(concentration_min min_concentration),
      ~w(concentration_max max_concentration),
      "#{path}.#{field}",
      path
    )
  end

  defp derived(payload, field, path) when field in ~w(min_frequency_hz max_frequency_hz) do
    case collection(payload, "modes", path) do
      nil ->
        nil

      rows ->
        case sample_bounds(rows, "#{path}.modes", &required_number(&1, "frequency_hz", &2)) do
          nil -> nil
          {min, max} -> if field == "min_frequency_hz", do: min, else: max
        end
    end
  end

  defp derived(payload, "frequency_span_hz" = field, path) do
    min = resolve(payload, "min_frequency_hz", path)
    max = resolve(payload, "max_frequency_hz", path)
    difference(min, max, "#{path}.#{field}")
  end

  defp derived(payload, "mode_1_participation_norm", path) do
    case collection(payload, "modes", path) do
      [first | _] ->
        first
        |> Check.object("#{path}.modes[0]")
        |> required_number("participation_norm", "#{path}.modes[0]")

      _ ->
        nil
    end
  end

  defp derived(payload, "velocity_magnitude", path) do
    x = first_number(payload, ["vx"], path)
    y = first_number(payload, ["vy"], path)
    if x != nil and y != nil, do: Check.hypot(x, y, "#{path}.velocity_magnitude")
  end

  defp derived(payload, field, path) do
    if String.ends_with?(field, "_span") do
      prefix = String.replace_suffix(field, "_span", "")
      bounds_delta(payload, ["#{prefix}_min"], ["#{prefix}_max"], "#{path}.#{field}", path)
    end
  end

  defp transient_peak(payload, field, path) do
    case Map.get(@transient_fields, field) do
      nil ->
        nil

      component ->
        case collection(payload, "nodes", path) do
          nil ->
            nil

          rows ->
            rows
            |> sample_bounds("#{path}.nodes", fn row, path ->
              abs(required_number(row, component, path))
            end)
            |> upper()
        end
    end
  end

  defp upper(nil), do: nil
  defp upper({_min, max}), do: max

  defp sample_bounds(rows, path, read) do
    {_index, bounds} =
      Enum.reduce(rows, {0, nil}, fn row, {index, bounds} ->
        row_path = "#{path}[#{index}]"
        value = row |> Check.object(row_path) |> read.(row_path)

        next =
          case bounds do
            nil -> {value, value}
            {min, max} -> {min(min, value), max(max, value)}
          end

        {index + 1, next}
      end)

    bounds
  end

  defp peak_frequency(rows, path) do
    {_index, peak} =
      Enum.reduce(rows, {0, nil}, fn row, {index, peak} ->
        row_path = "#{path}[#{index}]"
        row = Check.object(row, row_path)
        frequency = frequency_number(row, "frequency_hz", row_path)
        amplitude = frequency_number(row, "max_displacement", row_path)

        next =
          case peak do
            {_frequency, previous} when previous > amplitude -> peak
            _ -> {frequency, amplitude}
          end

        {index + 1, next}
      end)

    case peak do
      nil -> nil
      {frequency, _amplitude} -> frequency
    end
  end

  defp frequency_number(row, field, path) do
    first_number(row, [field | Map.get(@frequency_aliases, field, [])], path) ||
      Check.fail("#{path}.#{field} is missing a numeric sample")
  end

  defp required_number(row, field, path), do: Check.number(row[field], "#{path}.#{field}")

  defp first_number(payload, fields, path) do
    Enum.reduce_while(fields, nil, fn field, _ ->
      case Map.fetch(payload, field) do
        :error -> {:cont, nil}
        {:ok, value} -> {:halt, Check.number(value, "#{path}.#{field}")}
      end
    end)
  end

  defp collection(payload, field, path) do
    case Map.fetch(payload, field) do
      :error -> nil
      {:ok, value} -> Check.list(value, "#{path}.#{field}")
    end
  end

  defp bounds_delta(payload, min_fields, max_fields, result_path, path) do
    min = first_number(payload, min_fields, path)
    max = first_number(payload, max_fields, path)
    difference(min, max, result_path)
  end

  defp difference(min, max, path) when is_number(min) and is_number(max),
    do: Check.calculate(path, fn -> abs(max - min) end)

  defp difference(_min, _max, _path), do: nil
end
