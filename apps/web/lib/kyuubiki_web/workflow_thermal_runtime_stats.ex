defmodule KyuubikiWeb.WorkflowThermalRuntimeStats do
  @moduledoc false

  def normalize_prefix(prefix) when is_binary(prefix) do
    trimmed = String.trim(prefix)
    if trimmed == "", do: "summary", else: trimmed
  end

  def merge_diagnostic_contract(summary, domain, subject, prefix, metric_groups) do
    summary
    |> Map.put("diagnostic_contract", "kyuubiki.workflow_diagnostics/v1")
    |> Map.put("diagnostic_domain", domain)
    |> Map.put("diagnostic_subject", subject)
    |> Map.put("diagnostic_prefix", prefix)
    |> Map.put("diagnostic_node_count", Map.get(summary, "#{prefix}_node_count", 0))
    |> Map.put("diagnostic_element_count", Map.get(summary, "#{prefix}_element_count", 0))
    |> Map.put("diagnostic_metric_groups", metric_groups)
  end

  def maybe_merge_temperature_stats(summary, _prefix, []), do: summary

  def maybe_merge_temperature_stats(summary, prefix, temperature_nodes) do
    merge_node_range_stats(summary, prefix, "temperature", temperature_nodes)
  end

  def maybe_merge_load_stats(summary, _prefix, []), do: summary

  def maybe_merge_load_stats(summary, prefix, heat_load_nodes) do
    values = Enum.map(heat_load_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(heat_load_nodes, &abs(elem(&1, 1)))
    loaded_count = Enum.count(values, &(&1 != 0.0))

    summary
    |> Map.put("#{prefix}_heat_load_count", length(values))
    |> Map.put("#{prefix}_loaded_node_count", loaded_count)
    |> Map.put("#{prefix}_total_heat_load", Enum.sum(values))
    |> Map.put("#{prefix}_heat_load_mean", Enum.sum(values) / length(values))
    |> Map.put("#{prefix}_peak_heat_load", peak_value)
    |> Map.put("#{prefix}_peak_heat_load_node_id", peak_id)
  end

  def maybe_merge_temperature_delta_stats(summary, _prefix, []), do: summary

  def maybe_merge_temperature_delta_stats(summary, prefix, delta_nodes) do
    values = Enum.map(delta_nodes, &elem(&1, 1))

    summary
    |> merge_node_range_stats(prefix, "temperature_delta", delta_nodes)
    |> Map.put("#{prefix}_heated_node_count", Enum.count(values, &(&1 != 0.0)))
  end

  def maybe_merge_peak_vector(summary, _prefix, _name, nil), do: summary

  def maybe_merge_peak_vector(summary, prefix, name, {id, magnitude, components}) do
    Enum.reduce(components, summary, fn {axis, value}, acc ->
      Map.put(acc, "#{prefix}_peak_#{name}_#{axis}", value)
    end)
    |> Map.put("#{prefix}_peak_#{name}_magnitude", magnitude)
    |> put_peak_identity(prefix, name, id)
  end

  def maybe_merge_peak_scalar(summary, _prefix, _name, nil), do: summary

  def maybe_merge_peak_scalar(summary, prefix, name, {id, value}) do
    put_peak_value(summary, prefix, name, id, value)
  end

  def maybe_merge_peak_magnitude(summary, _prefix, _name, nil), do: summary

  def maybe_merge_peak_magnitude(summary, prefix, name, {id, value, components}) do
    Enum.reduce(components, summary, fn {axis, component}, acc ->
      Map.put(acc, "#{prefix}_peak_#{name}_#{axis}", component)
    end)
    |> put_peak_value(prefix, name, id, value)
  end

  def maybe_merge_peak_magnitude(summary, prefix, name, {id, value}) do
    put_peak_value(summary, prefix, name, id, value)
  end

  defp merge_node_range_stats(summary, prefix, name, node_values) do
    values = Enum.map(node_values, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(node_values, &elem(&1, 1))
    {min_id, min_value} = Enum.min_by(node_values, &elem(&1, 1))

    summary
    |> Map.put("#{prefix}_#{name}_min", min_value)
    |> Map.put("#{prefix}_#{name}_min_node_id", min_id)
    |> Map.put("#{prefix}_#{name}_max", peak_value)
    |> Map.put("#{prefix}_#{name}_max_node_id", peak_id)
    |> Map.put("#{prefix}_#{name}_span", peak_value - min_value)
    |> Map.put("#{prefix}_#{name}_mean", Enum.sum(values) / length(values))
  end

  defp put_peak_value(summary, prefix, name, id, value) do
    summary
    |> Map.put("#{prefix}_peak_#{name}", value)
    |> put_peak_identity(prefix, name, id)
  end

  defp put_peak_identity(summary, prefix, name, id) do
    summary
    |> Map.put("#{prefix}_peak_#{name}_id", id)
    |> Map.put("#{prefix}_peak_#{name}_element_id", id)
  end
end
