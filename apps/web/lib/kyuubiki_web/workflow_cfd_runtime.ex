defmodule KyuubikiWeb.WorkflowCfdRuntime do
  @moduledoc false

  def extract_stokes_flow_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    nodes = Map.get(payload, "nodes", [])
    elements = Map.get(payload, "elements", [])

    with true <- is_list(nodes) and is_list(elements),
         false <- nodes == [] and elements == [] do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "cfd"))
      velocity_values = numeric_values(nodes, "velocity_magnitude")
      pressure_values = numeric_values(nodes, "pressure")
      divergence_values = numeric_values(elements, "divergence_error")
      reynolds_values = numeric_values(elements, "reynolds_number")
      dissipation_values = numeric_values(elements, "viscous_dissipation")

      summary =
        %{
          "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
          "diagnostic_domain" => "fluid",
          "diagnostic_subject" => "stokes_flow_result",
          "diagnostic_prefix" => prefix,
          "diagnostic_node_count" => length(nodes),
          "diagnostic_element_count" => length(elements)
        }
        |> merge_min_max("#{prefix}_velocity", velocity_values)
        |> merge_min_max("#{prefix}_pressure", pressure_values)
        |> merge_peak("#{prefix}_divergence_error", elements, "divergence_error")
        |> merge_peak("#{prefix}_reynolds_number", elements, "reynolds_number")
        |> merge_peak("#{prefix}_viscous_dissipation", elements, "viscous_dissipation")
        |> Map.put("#{prefix}_velocity_mean", mean_or_zero(velocity_values))
        |> Map.put("#{prefix}_pressure_mean", mean_or_zero(pressure_values))
        |> Map.put("#{prefix}_divergence_error_mean", mean_or_zero(divergence_values))
        |> Map.put("#{prefix}_reynolds_number_mean", mean_or_zero(reynolds_values))
        |> Map.put("#{prefix}_viscous_dissipation_total", Enum.sum(dissipation_values))

      {:ok, summary}
    else
      _ -> {:error, :invalid_stokes_flow_result}
    end
  end

  def extract_stokes_flow_result_diagnostics(_payload, _config),
    do: {:error, :invalid_stokes_flow_result}

  defp merge_min_max(summary, key, []),
    do: summary |> Map.put("#{key}_min", 0.0) |> Map.put("#{key}_max", 0.0)

  defp merge_min_max(summary, key, values),
    do:
      summary
      |> Map.put("#{key}_min", Enum.min(values))
      |> Map.put("#{key}_max", Enum.max(values))

  defp merge_peak(summary, key, elements, field) do
    case peak_element(elements, field) do
      nil ->
        summary
        |> Map.put("#{key}_peak", 0.0)
        |> Map.put("#{key}_peak_element_id", nil)

      {element, value} ->
        summary
        |> Map.put("#{key}_peak", value)
        |> Map.put("#{key}_peak_element_id", Map.get(element, "id"))
    end
  end

  defp peak_element(elements, field) do
    elements
    |> Enum.filter(&is_map/1)
    |> Enum.filter(&number?(Map.get(&1, field)))
    |> Enum.max_by(fn element -> abs(Map.fetch!(element, field)) end, fn -> nil end)
    |> case do
      nil -> nil
      element -> {element, Map.fetch!(element, field)}
    end
  end

  defp numeric_values(values, field) do
    values
    |> Enum.filter(&is_map/1)
    |> Enum.map(&Map.get(&1, field))
    |> Enum.filter(&number?/1)
  end

  defp mean_or_zero([]), do: 0.0
  defp mean_or_zero(values), do: Enum.sum(values) / length(values)

  defp number?(value), do: is_integer(value) or is_float(value)

  defp normalize_prefix(prefix) when is_binary(prefix) do
    case String.trim(prefix) do
      "" -> "cfd"
      value -> value
    end
  end

  defp normalize_prefix(_prefix), do: "cfd"
end
