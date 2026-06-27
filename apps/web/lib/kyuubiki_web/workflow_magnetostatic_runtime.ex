defmodule KyuubikiWeb.WorkflowMagnetostaticRuntime do
  @moduledoc false

  def extract_magnetostatic_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "magnetostatic"))

      vector_potential_nodes =
        collect_numeric_entries(
          nodes,
          Map.get(config, "vector_potential_field", "vector_potential")
        )

      current_density_nodes =
        collect_numeric_entries(
          nodes,
          Map.get(config, "current_density_field", "current_density")
        )

      energy_density_peak =
        peak_scalar_entry(
          elements,
          Map.get(config, "energy_density_field", "energy_area_density")
        ) ||
          peak_scalar_entry(elements, "stored_energy")

      field_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "field_x_field", "magnetic_field_strength_x"),
          Map.get(config, "field_y_field", "magnetic_field_strength_y"),
          Map.get(config, "field_z_field"),
          Map.get(config, "field_magnitude_field", "magnetic_field_strength_magnitude")
        )

      flux_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "flux_x_field", "magnetic_flux_density_x"),
          Map.get(config, "flux_y_field", "magnetic_flux_density_y"),
          Map.get(config, "flux_z_field"),
          Map.get(config, "flux_magnitude_field", "magnetic_flux_density_magnitude")
        )

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> merge_diagnostic_contract("magnetostatic", "magnetostatic_result", prefix, [
          "vector_potential",
          "current_density",
          "energy_density",
          "field",
          "flux"
        ])
        |> maybe_merge_distribution_stats(prefix, "vector_potential", vector_potential_nodes)
        |> maybe_merge_distribution_stats(prefix, "current_density", current_density_nodes)
        |> maybe_merge_peak_scalar(prefix, "energy_density", energy_density_peak)
        |> maybe_merge_peak_vector(prefix, "field", field_peak)
        |> maybe_merge_peak_vector(prefix, "flux", flux_peak)

      if map_size(diagnostics) <= 6 do
        {:error, :empty_magnetostatic_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_magnetostatic_result}
    end
  end

  def extract_magnetostatic_result_diagnostics(_payload, _config),
    do: {:error, :invalid_magnetostatic_result}

  defp fetch_list(payload, key) when is_binary(key) do
    case Map.get(payload, key) do
      entries when is_list(entries) -> {:ok, entries}
      _ -> {:error, :invalid_result_collection}
    end
  end

  defp fetch_list(_payload, _key), do: {:error, :invalid_result_collection}

  defp collect_numeric_entries(entries, field) when is_list(entries) and is_binary(field) do
    entries
    |> Enum.filter(&is_map/1)
    |> Enum.flat_map(fn entry ->
      case fetch_numeric_field(entry, field) do
        {:ok, value} -> [{entry, value}]
        :error -> []
      end
    end)
  end

  defp collect_numeric_entries(_entries, _field), do: []

  defp peak_scalar_entry(entries, field) when is_list(entries) and is_binary(field) do
    entries
    |> collect_numeric_entries(field)
    |> Enum.max_by(&elem(&1, 1), fn -> nil end)
  end

  defp peak_scalar_entry(_entries, _field), do: nil

  defp peak_vector_entry(entries, x_field, y_field, z_field, magnitude_field) do
    entries
    |> Enum.filter(&is_map/1)
    |> Enum.flat_map(fn entry ->
      components =
        %{}
        |> maybe_put_component("x", entry, x_field)
        |> maybe_put_component("y", entry, y_field)
        |> maybe_put_component("z", entry, z_field)

      magnitude =
        case fetch_numeric_field(entry, magnitude_field) do
          {:ok, value} -> value
          :error -> vector_magnitude(components)
        end

      if magnitude == nil,
        do: [],
        else: [%{"entry" => entry, "magnitude" => magnitude, "components" => components}]
    end)
    |> Enum.max_by(& &1["magnitude"], fn -> nil end)
  end

  defp maybe_put_component(components, _axis, _entry, field) when not is_binary(field),
    do: components

  defp maybe_put_component(components, axis, entry, field) do
    case fetch_numeric_field(entry, field) do
      {:ok, value} -> Map.put(components, axis, value)
      :error -> components
    end
  end

  defp vector_magnitude(components) when map_size(components) >= 2 do
    components
    |> Map.values()
    |> Enum.reduce(0.0, &(&1 * &1 + &2))
    |> :math.sqrt()
  end

  defp vector_magnitude(_components), do: nil

  defp maybe_merge_distribution_stats(diagnostics, _prefix, _label, []), do: diagnostics

  defp maybe_merge_distribution_stats(diagnostics, prefix, label, entries) do
    values = Enum.map(entries, &elem(&1, 1))
    sum = Enum.sum(values)

    diagnostics
    |> Map.put("#{prefix}_#{label}_count", length(values))
    |> Map.put("#{prefix}_#{label}_min", Enum.min(values))
    |> Map.put("#{prefix}_#{label}_max", Enum.max(values))
    |> Map.put("#{prefix}_#{label}_mean", sum / length(values))
    |> Map.put("#{prefix}_#{label}_sum", sum)
    |> Map.put("#{prefix}_#{label}_span", Enum.max(values) - Enum.min(values))
  end

  defp maybe_merge_peak_scalar(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_scalar(diagnostics, prefix, label, {entry, value}) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak", value)
    |> Map.put("#{prefix}_peak_#{label}", value)
    |> Map.put("#{prefix}_peak_#{label}_id", Map.get(entry, "id"))
    |> Map.put("#{prefix}_#{label}_peak_element_id", Map.get(entry, "id"))
  end

  defp maybe_merge_peak_vector(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_vector(diagnostics, prefix, label, peak) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak_magnitude", peak["magnitude"])
    |> Map.put("#{prefix}_peak_#{label}", peak["magnitude"])
    |> Map.put("#{prefix}_peak_#{label}_id", get_in(peak, ["entry", "id"]))
    |> Map.put("#{prefix}_#{label}_peak_element_id", get_in(peak, ["entry", "id"]))
    |> maybe_put_vector_components(prefix, label, peak["components"])
  end

  defp maybe_put_vector_components(diagnostics, prefix, label, components) do
    Enum.reduce(components, diagnostics, fn {axis, value}, acc ->
      Map.put(acc, "#{prefix}_#{label}_peak_#{axis}", value)
    end)
  end

  defp merge_diagnostic_contract(diagnostics, domain, subject, prefix, metric_groups) do
    diagnostics
    |> Map.put("diagnostic_contract", "kyuubiki.workflow_diagnostics/v1")
    |> Map.put("diagnostic_domain", domain)
    |> Map.put("diagnostic_subject", subject)
    |> Map.put("diagnostic_prefix", prefix)
    |> Map.put("diagnostic_node_count", Map.get(diagnostics, "#{prefix}_node_count", 0))
    |> Map.put("diagnostic_element_count", Map.get(diagnostics, "#{prefix}_element_count", 0))
    |> Map.put("diagnostic_metric_groups", metric_groups)
  end

  defp fetch_numeric_field(entry, field) when is_map(entry) and is_binary(field) do
    case Map.get(entry, field) do
      value when is_number(value) -> {:ok, value * 1.0}
      _ -> :error
    end
  end

  defp fetch_numeric_field(_entry, _field), do: :error

  defp normalize_prefix(prefix) when is_binary(prefix) and prefix != "" do
    prefix
    |> String.trim()
    |> String.replace(~r/[^a-zA-Z0-9]+/u, "_")
    |> String.trim("_")
    |> case do
      "" -> "magnetostatic"
      normalized -> normalized
    end
  end

  defp normalize_prefix(_prefix), do: "magnetostatic"
end
