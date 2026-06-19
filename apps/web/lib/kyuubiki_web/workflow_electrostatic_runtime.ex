defmodule KyuubikiWeb.WorkflowElectrostaticRuntime do
  @moduledoc false

  def extract_electrostatic_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "electrostatic"))
      potential_field = Map.get(config, "potential_field", "potential")
      charge_density_field = Map.get(config, "charge_density_field", "charge_density")
      energy_density_field = Map.get(config, "energy_density_field", "energy_density")
      charge_density_entries =
        resolve_charge_density_entries(
          config,
          nodes,
          elements
        )

      potential_nodes = collect_numeric_entries(nodes, potential_field)
      charge_density_values = collect_numeric_entries(charge_density_entries, charge_density_field)
      energy_density_peak = peak_scalar_entry(elements, energy_density_field)

      field_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "field_x_field", "electric_field_x"),
          Map.get(config, "field_y_field", "electric_field_y"),
          Map.get(config, "field_z_field"),
          Map.get(config, "field_magnitude_field", "electric_field_magnitude")
        )

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> merge_diagnostic_contract("electrostatic", "electrostatic_result", prefix, [
          "potential",
          "charge_density",
          "energy_density",
          "field"
        ])
        |> maybe_merge_range_stats(prefix, "potential", potential_nodes)
        |> maybe_merge_distribution_stats(prefix, "charge_density", charge_density_values)
        |> maybe_merge_peak_scalar(prefix, "energy_density", energy_density_peak)
        |> maybe_merge_peak_vector(prefix, "field", field_peak)

      if map_size(diagnostics) <= 2 do
        {:error, :empty_electrostatic_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_electrostatic_result}
    end
  end

  defp fetch_list(payload, key) when is_binary(key) do
    case Map.get(payload, key) do
      entries when is_list(entries) -> {:ok, entries}
      _ -> {:error, :invalid_result_collection}
    end
  end

  defp fetch_list(_payload, _key), do: {:error, :invalid_result_collection}

  defp resolve_charge_density_entries(config, nodes, elements) do
    case Map.get(config, "charge_density_source", "nodes") do
      "elements" -> elements
      "element" -> elements
      _ -> nodes
    end
  end

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
    |> Enum.filter(&is_map/1)
    |> Enum.reduce(nil, fn entry, best ->
      case fetch_numeric_field(entry, field) do
        {:ok, value} ->
          candidate = %{"entry" => entry, "value" => value}

          cond do
            is_nil(best) -> candidate
            value > best["value"] -> candidate
            true -> best
          end

        :error ->
          best
      end
    end)
  end

  defp peak_scalar_entry(_entries, _field), do: nil

  defp peak_vector_entry(entries, x_field, y_field, z_field, magnitude_field)
       when is_list(entries) and is_binary(magnitude_field) do
    entries
    |> Enum.filter(&is_map/1)
    |> Enum.reduce(nil, fn entry, best ->
      case resolve_vector_magnitude(entry, x_field, y_field, z_field, magnitude_field) do
        {:ok, magnitude, components} ->
          candidate = %{
            "entry" => entry,
            "magnitude" => magnitude,
            "components" => components
          }

          cond do
            is_nil(best) -> candidate
            magnitude > best["magnitude"] -> candidate
            true -> best
          end

        :error ->
          best
      end
    end)
  end

  defp peak_vector_entry(_entries, _x_field, _y_field, _z_field, _magnitude_field), do: nil

  defp resolve_vector_magnitude(entry, x_field, y_field, z_field, magnitude_field) do
    case fetch_numeric_field(entry, magnitude_field) do
      {:ok, magnitude} ->
        {:ok, magnitude, vector_components(entry, x_field, y_field, z_field)}

      :error ->
        with {:ok, x} <- fetch_numeric_field(entry, x_field),
             {:ok, y} <- fetch_numeric_field(entry, y_field) do
          components =
            vector_components(entry, x_field, y_field, z_field)
            |> Map.put_new("x", x)
            |> Map.put_new("y", y)

          magnitude =
            x * x + y * y + square_optional_component(Map.get(components, "z"))
            |> :math.sqrt()

          {:ok, magnitude, components}
        else
          _ -> :error
        end
    end
  end

  defp vector_components(entry, x_field, y_field, z_field) do
    %{}
    |> maybe_put_component("x", entry, x_field)
    |> maybe_put_component("y", entry, y_field)
    |> maybe_put_component("z", entry, z_field)
  end

  defp maybe_put_component(components, _axis, _entry, field) when not is_binary(field), do: components

  defp maybe_put_component(components, axis, entry, field) do
    case fetch_numeric_field(entry, field) do
      {:ok, value} -> Map.put(components, axis, value)
      :error -> components
    end
  end

  defp square_optional_component(nil), do: 0.0
  defp square_optional_component(value), do: value * value

  defp maybe_merge_range_stats(diagnostics, _prefix, _label, []), do: diagnostics

  defp maybe_merge_range_stats(diagnostics, prefix, label, entries) do
    values = Enum.map(entries, &elem(&1, 1))
    mean = Enum.sum(values) / length(values)

    diagnostics
    |> Map.put("#{prefix}_#{label}_min", Enum.min(values))
    |> Map.put("#{prefix}_#{label}_max", Enum.max(values))
    |> Map.put("#{prefix}_#{label}_mean", mean)
    |> Map.put("#{prefix}_#{label}_span", Enum.max(values) - Enum.min(values))
  end

  defp maybe_merge_distribution_stats(diagnostics, _prefix, _label, []), do: diagnostics

  defp maybe_merge_distribution_stats(diagnostics, prefix, label, entries) do
    values = Enum.map(entries, &elem(&1, 1))

    diagnostics
    |> Map.put("#{prefix}_#{label}_count", length(values))
    |> Map.put("#{prefix}_#{label}_min", Enum.min(values))
    |> Map.put("#{prefix}_#{label}_max", Enum.max(values))
    |> Map.put("#{prefix}_#{label}_mean", Enum.sum(values) / length(values))
    |> Map.put("#{prefix}_#{label}_sum", Enum.sum(values))
  end

  defp maybe_merge_peak_scalar(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_scalar(diagnostics, prefix, label, peak) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak", peak["value"])
    |> Map.put("#{prefix}_#{label}_peak_element_id", get_in(peak, ["entry", "id"]))
  end

  defp maybe_merge_peak_vector(diagnostics, _prefix, _label, nil), do: diagnostics

  defp maybe_merge_peak_vector(diagnostics, prefix, label, peak) do
    diagnostics
    |> Map.put("#{prefix}_#{label}_peak_magnitude", peak["magnitude"])
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
      "" -> "electrostatic"
      normalized -> normalized
    end
  end

  defp normalize_prefix(_prefix), do: "electrostatic"
end
