defmodule KyuubikiWeb.WorkflowPeakRuntime do
  @moduledoc false

  def extract_electrostatic_peak_field(payload, _config) when is_map(payload) do
    with {:ok, elements} <- fetch_list(payload, "elements"),
         {:ok, peak_element} <- peak_by_field(elements, "electric_field_magnitude") do
      {:ok,
       %{
         "peak_element_id" => Map.get(peak_element, "id"),
         "peak_electric_field" => numeric_value(peak_element, "electric_field_magnitude"),
         "peak_flux_density" => numeric_value(peak_element, "electric_flux_density_magnitude"),
         "peak_average_potential" => numeric_value(peak_element, "average_potential"),
         "peak_electric_field_x" => numeric_value(peak_element, "electric_field_x"),
         "peak_electric_field_y" => numeric_value(peak_element, "electric_field_y"),
         "peak_flux_density_x" => numeric_value(peak_element, "electric_flux_density_x"),
         "peak_flux_density_y" => numeric_value(peak_element, "electric_flux_density_y"),
         "peak_potential_gradient_x" => numeric_value(peak_element, "potential_gradient_x"),
         "peak_potential_gradient_y" => numeric_value(peak_element, "potential_gradient_y"),
         "peak_potential_gradient_magnitude" =>
           magnitude2(
             numeric_value(peak_element, "potential_gradient_x"),
             numeric_value(peak_element, "potential_gradient_y")
           ),
         "electrostatic_peak_field" => numeric_value(peak_element, "electric_field_magnitude"),
         "electrostatic_peak_average_potential" =>
           numeric_value(peak_element, "average_potential"),
         "electrostatic_peak_field_x" => numeric_value(peak_element, "electric_field_x"),
         "electrostatic_peak_field_y" => numeric_value(peak_element, "electric_field_y"),
         "electrostatic_peak_potential_gradient_magnitude" =>
           magnitude2(
             numeric_value(peak_element, "potential_gradient_x"),
             numeric_value(peak_element, "potential_gradient_y")
           ),
         "electrostatic_field_peak_magnitude" =>
           numeric_value(peak_element, "electric_field_magnitude"),
         "electrostatic_field_peak_x" => numeric_value(peak_element, "electric_field_x"),
         "electrostatic_field_peak_y" => numeric_value(peak_element, "electric_field_y"),
         "electrostatic_field_peak_element_id" => Map.get(peak_element, "id"),
         "electrostatic_peak_flux_density" =>
           numeric_value(peak_element, "electric_flux_density_magnitude"),
         "electrostatic_peak_flux_density_x" =>
           numeric_value(peak_element, "electric_flux_density_x"),
         "electrostatic_peak_flux_density_y" =>
           numeric_value(peak_element, "electric_flux_density_y"),
         "electrostatic_peak_field_id" => Map.get(peak_element, "id"),
         "electrostatic_potential_max" => numeric_value(payload, "max_potential"),
         "max_potential" => numeric_value(payload, "max_potential"),
         "max_electric_field" => numeric_value(payload, "max_electric_field"),
         "max_flux_density" => numeric_value(payload, "max_flux_density")
       }}
    else
      _ -> {:error, :invalid_electrostatic_peak_result}
    end
  end

  def extract_electrostatic_peak_field(_payload, _config),
    do: {:error, :invalid_electrostatic_peak_result}

  def extract_magnetostatic_peak_field(payload, _config) when is_map(payload) do
    with {:ok, elements} <- fetch_list(payload, "elements"),
         {:ok, peak_element} <- peak_by_field(elements, "magnetic_field_strength_magnitude") do
      {:ok,
       %{
         "peak_element_id" => Map.get(peak_element, "id"),
         "peak_magnetic_field_strength" =>
           numeric_value(peak_element, "magnetic_field_strength_magnitude"),
         "peak_flux_density" => numeric_value(peak_element, "magnetic_flux_density_magnitude"),
         "peak_average_vector_potential" =>
           numeric_value(peak_element, "average_vector_potential"),
         "peak_magnetic_field_strength_x" =>
           numeric_value(peak_element, "magnetic_field_strength_x"),
         "peak_magnetic_field_strength_y" =>
           numeric_value(peak_element, "magnetic_field_strength_y"),
         "peak_flux_density_x" => numeric_value(peak_element, "magnetic_flux_density_x"),
         "peak_flux_density_y" => numeric_value(peak_element, "magnetic_flux_density_y"),
         "peak_vector_potential_gradient_x" =>
           numeric_value(peak_element, "vector_potential_gradient_x"),
         "peak_vector_potential_gradient_y" =>
           numeric_value(peak_element, "vector_potential_gradient_y"),
         "peak_vector_potential_gradient_magnitude" =>
           magnitude2(
             numeric_value(peak_element, "vector_potential_gradient_x"),
             numeric_value(peak_element, "vector_potential_gradient_y")
           ),
         "peak_stored_energy" => numeric_value(peak_element, "stored_energy"),
         "magnetostatic_peak_field" =>
           numeric_value(peak_element, "magnetic_field_strength_magnitude"),
         "magnetostatic_peak_average_vector_potential" =>
           numeric_value(peak_element, "average_vector_potential"),
         "magnetostatic_peak_field_x" => numeric_value(peak_element, "magnetic_field_strength_x"),
         "magnetostatic_peak_field_y" => numeric_value(peak_element, "magnetic_field_strength_y"),
         "magnetostatic_field_peak_magnitude" =>
           numeric_value(peak_element, "magnetic_field_strength_magnitude"),
         "magnetostatic_field_peak_x" => numeric_value(peak_element, "magnetic_field_strength_x"),
         "magnetostatic_field_peak_y" => numeric_value(peak_element, "magnetic_field_strength_y"),
         "magnetostatic_field_peak_element_id" => Map.get(peak_element, "id"),
         "magnetostatic_peak_flux_density" =>
           numeric_value(peak_element, "magnetic_flux_density_magnitude"),
         "magnetostatic_peak_flux_density_x" =>
           numeric_value(peak_element, "magnetic_flux_density_x"),
         "magnetostatic_peak_flux_density_y" =>
           numeric_value(peak_element, "magnetic_flux_density_y"),
         "magnetostatic_flux_peak_magnitude" =>
           numeric_value(peak_element, "magnetic_flux_density_magnitude"),
         "magnetostatic_flux_peak_x" => numeric_value(peak_element, "magnetic_flux_density_x"),
         "magnetostatic_flux_peak_y" => numeric_value(peak_element, "magnetic_flux_density_y"),
         "magnetostatic_flux_peak_element_id" => Map.get(peak_element, "id"),
         "magnetostatic_peak_stored_energy" => numeric_value(peak_element, "stored_energy"),
         "magnetostatic_peak_field_id" => Map.get(peak_element, "id"),
         "max_vector_potential" => numeric_value(payload, "max_vector_potential"),
         "max_magnetic_field_strength" => numeric_value(payload, "max_magnetic_field_strength"),
         "max_flux_density" => numeric_value(payload, "max_flux_density"),
         "total_stored_energy" => numeric_value(payload, "total_stored_energy")
       }}
    else
      _ -> {:error, :invalid_magnetostatic_peak_result}
    end
  end

  def extract_magnetostatic_peak_field(_payload, _config),
    do: {:error, :invalid_magnetostatic_peak_result}

  def extract_heat_peak_flux(payload, _config) when is_map(payload) do
    with {:ok, elements} <- fetch_list(payload, "elements"),
         {:ok, peak_element} <- peak_by_field(elements, "heat_flux_magnitude") do
      {:ok,
       %{
         "peak_element_id" => Map.get(peak_element, "id"),
         "peak_heat_flux" => numeric_value(peak_element, "heat_flux_magnitude"),
         "peak_average_temperature" => numeric_value(peak_element, "average_temperature"),
         "peak_heat_flux_x" => numeric_value(peak_element, "heat_flux_x"),
         "peak_heat_flux_y" => numeric_value(peak_element, "heat_flux_y"),
         "peak_temperature_gradient_x" => numeric_value(peak_element, "temperature_gradient_x"),
         "peak_temperature_gradient_y" => numeric_value(peak_element, "temperature_gradient_y"),
         "peak_temperature_gradient_magnitude" =>
           magnitude2(
             numeric_value(peak_element, "temperature_gradient_x"),
             numeric_value(peak_element, "temperature_gradient_y")
           ),
         "thermal_peak_flux" => numeric_value(peak_element, "heat_flux_magnitude"),
         "thermal_peak_average_temperature" => numeric_value(peak_element, "average_temperature"),
         "thermal_peak_flux_x" => numeric_value(peak_element, "heat_flux_x"),
         "thermal_peak_flux_y" => numeric_value(peak_element, "heat_flux_y"),
         "thermal_peak_temperature_gradient_magnitude" =>
           magnitude2(
             numeric_value(peak_element, "temperature_gradient_x"),
             numeric_value(peak_element, "temperature_gradient_y")
           ),
         "thermal_flux_peak_magnitude" => numeric_value(peak_element, "heat_flux_magnitude"),
         "thermal_flux_peak_x" => numeric_value(peak_element, "heat_flux_x"),
         "thermal_flux_peak_y" => numeric_value(peak_element, "heat_flux_y"),
         "thermal_flux_peak_element_id" => Map.get(peak_element, "id"),
         "thermal_peak_flux_id" => Map.get(peak_element, "id"),
         "thermal_temperature_max" => numeric_value(payload, "max_temperature"),
         "max_temperature" => numeric_value(payload, "max_temperature"),
         "max_heat_flux" => numeric_value(payload, "max_heat_flux")
       }}
    else
      _ -> {:error, :invalid_heat_peak_result}
    end
  end

  def extract_heat_peak_flux(_payload, _config), do: {:error, :invalid_heat_peak_result}

  def extract_thermo_peak_response(payload, _config) when is_map(payload) do
    with {:ok, nodes} <- fetch_list(payload, "nodes"),
         {:ok, elements} <- fetch_list(payload, "elements"),
         {:ok, peak_node} <- peak_by_field(nodes, "displacement_magnitude"),
         {:ok, peak_element} <- peak_by_field(elements, "von_mises") do
      {:ok,
       %{
         "peak_node_id" => Map.get(peak_node, "id"),
         "peak_displacement" => numeric_value(peak_node, "displacement_magnitude"),
         "peak_displacement_x" => numeric_value(peak_node, "ux"),
         "peak_displacement_y" => numeric_value(peak_node, "uy"),
         "peak_node_temperature_delta" => numeric_value(peak_node, "temperature_delta"),
         "peak_element_id" => Map.get(peak_element, "id"),
         "peak_von_mises" => numeric_value(peak_element, "von_mises"),
         "peak_stress_x" => numeric_value(peak_element, "stress_x"),
         "peak_stress_y" => numeric_value(peak_element, "stress_y"),
         "peak_tau_xy" => numeric_value(peak_element, "tau_xy"),
         "peak_element_temperature_delta" =>
           numeric_value(peak_element, "average_temperature_delta"),
         "peak_thermal_strain" => numeric_value(peak_element, "thermal_strain"),
         "peak_mechanical_strain_x" => numeric_value(peak_element, "mechanical_strain_x"),
         "peak_mechanical_strain_y" => numeric_value(peak_element, "mechanical_strain_y"),
         "peak_total_strain_x" => numeric_value(peak_element, "total_strain_x"),
         "peak_total_strain_y" => numeric_value(peak_element, "total_strain_y"),
         "peak_gamma_xy" => numeric_value(peak_element, "gamma_xy"),
         "peak_principal_stress_1" => numeric_value(peak_element, "principal_stress_1"),
         "peak_principal_stress_2" => numeric_value(peak_element, "principal_stress_2"),
         "peak_max_in_plane_shear" => numeric_value(peak_element, "max_in_plane_shear"),
         "thermo_peak_displacement" => numeric_value(peak_node, "displacement_magnitude"),
         "thermo_peak_displacement_x" => numeric_value(peak_node, "ux"),
         "thermo_peak_displacement_y" => numeric_value(peak_node, "uy"),
         "thermo_displacement_peak_magnitude" =>
           numeric_value(peak_node, "displacement_magnitude"),
         "thermo_displacement_peak_x" => numeric_value(peak_node, "ux"),
         "thermo_displacement_peak_y" => numeric_value(peak_node, "uy"),
         "thermo_displacement_peak_element_id" => Map.get(peak_node, "id"),
         "thermo_peak_displacement_id" => Map.get(peak_node, "id"),
         "thermo_peak_stress" => numeric_value(peak_element, "von_mises"),
         "thermo_stress_peak" => numeric_value(peak_element, "von_mises"),
         "thermo_peak_stress_id" => Map.get(peak_element, "id"),
         "thermo_stress_peak_element_id" => Map.get(peak_element, "id"),
         "thermo_peak_thermal_strain" => numeric_value(peak_element, "thermal_strain"),
         "thermo_peak_mechanical_strain_x" => numeric_value(peak_element, "mechanical_strain_x"),
         "thermo_peak_mechanical_strain_y" => numeric_value(peak_element, "mechanical_strain_y"),
         "thermo_peak_total_strain_x" => numeric_value(peak_element, "total_strain_x"),
         "thermo_peak_total_strain_y" => numeric_value(peak_element, "total_strain_y"),
         "thermo_peak_gamma_xy" => numeric_value(peak_element, "gamma_xy"),
         "thermo_peak_principal_stress_1" => numeric_value(peak_element, "principal_stress_1"),
         "thermo_peak_principal_stress_2" => numeric_value(peak_element, "principal_stress_2"),
         "thermo_peak_max_in_plane_shear" => numeric_value(peak_element, "max_in_plane_shear"),
         "thermo_temperature_delta_max" => numeric_value(payload, "max_temperature_delta"),
         "max_displacement" => numeric_value(payload, "max_displacement"),
         "max_stress" => numeric_value(payload, "max_stress"),
         "max_temperature_delta" => numeric_value(payload, "max_temperature_delta")
       }}
    else
      _ -> {:error, :invalid_thermo_peak_result}
    end
  end

  def extract_thermo_peak_response(_payload, _config), do: {:error, :invalid_thermo_peak_result}

  defp fetch_list(payload, key) when is_binary(key) do
    case Map.get(payload, key) do
      entries when is_list(entries) -> {:ok, Enum.filter(entries, &is_map/1)}
      _ -> {:error, :invalid_result_collection}
    end
  end

  defp peak_by_field(entries, field) do
    case Enum.reduce(entries, nil, fn entry, best ->
           case fetch_numeric_field(entry, field) do
             {:ok, value} ->
               cond do
                 is_nil(best) -> {entry, value}
                 value > elem(best, 1) -> {entry, value}
                 true -> best
               end

             :error ->
               best
           end
         end) do
      {entry, _value} -> {:ok, entry}
      nil -> {:error, :empty_peak_collection}
    end
  end

  defp numeric_value(entry, field) do
    case fetch_numeric_field(entry, field) do
      {:ok, value} -> value
      :error -> nil
    end
  end

  defp fetch_numeric_field(entry, field) when is_map(entry) and is_binary(field) do
    case Map.get(entry, field) do
      value when is_integer(value) -> {:ok, value * 1.0}
      value when is_float(value) -> {:ok, value}
      _ -> :error
    end
  end

  defp fetch_numeric_field(_entry, _field), do: :error

  defp magnitude2(nil, nil), do: nil

  defp magnitude2(x, y) do
    x_value = x || 0.0
    y_value = y || 0.0
    :math.sqrt(x_value * x_value + y_value * y_value)
  end
end
