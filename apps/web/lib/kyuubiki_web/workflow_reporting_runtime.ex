defmodule KyuubikiWeb.WorkflowReportingRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowSummaryRuntime

  def extract_result_summary(payload, config) when is_map(payload) and is_map(config) do
    requested_fields =
      case Map.get(config, "fields") do
        fields when is_list(fields) -> Enum.filter(fields, &is_binary/1)
        _ -> nil
      end

    summary =
      if is_list(requested_fields) do
        Enum.reduce(requested_fields, %{}, fn field, acc ->
          case Map.fetch(payload, field) do
            {:ok, value} -> Map.put(acc, field, value)
            :error -> acc
          end
        end)
      else
        payload
        |> Enum.filter(fn {key, _value} -> String.starts_with?(key, "max_") end)
        |> Map.new()
      end

    if map_size(summary) == 0, do: {:error, :empty_summary}, else: {:ok, summary}
  end

  def export_summary_json(payload) when is_map(payload) do
    {:ok,
     %{
       "format" => "json",
       "content_type" => "application/json",
       "content" => Jason.encode!(payload)
     }}
  end

  def export_summary_csv(payload, config) when is_map(payload) and is_map(config) do
    requested_fields =
      case Map.get(config, "fields") do
        fields when is_list(fields) -> Enum.filter(fields, &is_binary/1)
        _ -> nil
      end

    rows =
      if is_list(requested_fields) do
        Enum.reduce(requested_fields, [["key", "value"]], fn field, acc ->
          case Map.fetch(payload, field) do
            {:ok, value} -> acc ++ [[field, value]]
            :error -> acc
          end
        end)
      else
        [["key", "value"]] ++ Enum.map(payload, fn {key, value} -> [key, value] end)
      end

    if length(rows) == 1 do
      {:error, :empty_export}
    else
      {:ok,
       %{
         "format" => "csv",
         "content_type" => "text/csv",
         "content" =>
           rows
           |> Enum.map_join("\n", fn row -> Enum.map_join(row, ",", &csv_escape/1) end)
           |> Kernel.<>("\n")
       }}
    end
  end

  def merge_summary_pair(payload, config),
    do: WorkflowSummaryRuntime.merge_summary_pair(payload, config)

  def compare_summary_pair(payload, config),
    do: WorkflowSummaryRuntime.compare_summary_pair(payload, config)

  def aggregate_summary_collection(payload, config),
    do: WorkflowSummaryRuntime.aggregate_summary_collection(payload, config)

  def normalize_summary_fields(payload, config),
    do: WorkflowSummaryRuntime.normalize_summary_fields(payload, config)

  def select_best_summary(payload, config),
    do: WorkflowSummaryRuntime.select_best_summary(payload, config)

  def extract_field_statistics(payload, config) when is_map(payload) and is_map(config) do
    source = Map.get(config, "source", "nodes")
    field = Map.get(config, "field")

    with true <- is_binary(field),
         {:ok, items} <- fetch_list(payload, source) do
      values =
        items
        |> Enum.filter(&is_map/1)
        |> Enum.flat_map(fn item ->
          case fetch_numeric_field(item, field) do
            {:ok, value} -> [value]
            :error -> []
          end
        end)

      if values == [] do
        {:error, :empty_statistics}
      else
        prefix = normalize_prefix(Map.get(config, "output_prefix", field))
        mean = Enum.sum(values) / length(values)

        summary =
          %{
            "#{prefix}_count" => length(values),
            "#{prefix}_min" => Enum.min(values),
            "#{prefix}_max" => Enum.max(values),
            "#{prefix}_sum" => Enum.sum(values),
            "#{prefix}_mean" => mean,
            "#{prefix}_stddev" => population_stddev(values, mean),
            "source_collection" => source,
            "source_field" => field
          }
          |> Map.merge(
            Map.new(normalize_percentiles(Map.get(config, "percentiles", [])), fn percentile ->
              {"#{prefix}_#{format_percentile_key(percentile)}",
               interpolate_percentile(values, percentile)}
            end)
          )

        {:ok, summary}
      end
    else
      _ -> {:error, :invalid_statistics_config}
    end
  end

  def extract_field_hotspots(payload, config) when is_map(payload) and is_map(config) do
    source = Map.get(config, "source", "elements")
    field = Map.get(config, "field")

    with true <- is_binary(field),
         {:ok, items} <- fetch_list(payload, source),
         {:ok, threshold} <- resolve_hotspot_threshold(items, field, config) do
      hotspots =
        items
        |> Enum.filter(&is_map/1)
        |> Enum.flat_map(fn item ->
          case fetch_numeric_field(item, field) do
            {:ok, value} when value >= threshold -> [{value, item}]
            _ -> []
          end
        end)
        |> sort_hotspots(Map.get(config, "sample_sort", "value_desc"))

      if hotspots == [] do
        {:error, :empty_hotspots}
      else
        values = Enum.map(hotspots, &elem(&1, 0))
        prefix = normalize_prefix(Map.get(config, "output_prefix", field))
        sample_limit = min(Map.get(config, "sample_limit", 8), 32)

        {:ok,
         %{
           "#{prefix}_threshold" => threshold,
           "#{prefix}_hotspot_count" => length(hotspots),
           "#{prefix}_hotspot_fraction" => length(hotspots) / max(length(items), 1),
           "#{prefix}_hotspot_mean" => Enum.sum(values) / length(values),
           "#{prefix}_hotspot_max" => Enum.max(values),
           "#{prefix}_sample_sort" => Map.get(config, "sample_sort", "value_desc"),
           "#{prefix}_hotspot_ids" =>
             hotspots
             |> Enum.map(&elem(&1, 1))
             |> Enum.map(&Map.get(&1, "id"))
             |> Enum.reject(&is_nil/1),
           "#{prefix}_hotspot_samples" =>
             hotspots
             |> Enum.take(sample_limit)
             |> Enum.map(&elem(&1, 1)),
           "source_collection" => source,
           "source_field" => field
         }}
      end
    else
      _ -> {:error, :invalid_hotspot_config}
    end
  end

  def extract_thermal_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "thermal"))
      temperature_field = Map.get(config, "temperature_field", "temperature")
      heat_load_field = Map.get(config, "heat_load_field", "heat_load")

      temperature_nodes = collect_numeric_entries(nodes, temperature_field)
      heat_load_nodes = collect_numeric_entries(nodes, heat_load_field)

      gradient_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "gradient_x_field", "temperature_gradient_x"),
          Map.get(config, "gradient_y_field", "temperature_gradient_y"),
          Map.get(config, "gradient_z_field")
        )

      flux_peak =
        peak_vector_entry(
          elements,
          Map.get(config, "flux_x_field", "heat_flux_x"),
          Map.get(config, "flux_y_field", "heat_flux_y"),
          Map.get(config, "flux_z_field")
        )

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> maybe_merge_temperature_stats(prefix, temperature_nodes)
        |> maybe_merge_load_stats(prefix, heat_load_nodes)
        |> maybe_merge_peak_vector(prefix, "gradient", gradient_peak)
        |> maybe_merge_peak_vector(prefix, "flux", flux_peak)

      if map_size(diagnostics) <= 2 do
        {:error, :empty_thermal_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_thermal_result}
    end
  end

  def extract_thermo_result_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, nodes} <- fetch_list(payload, Map.get(config, "node_source", "nodes")),
         {:ok, elements} <- fetch_list(payload, Map.get(config, "element_source", "elements")) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "thermo"))
      delta_field = Map.get(config, "temperature_delta_field", "temperature_delta")
      stress_field = Map.get(config, "stress_field", "von_mises_stress")

      delta_nodes = collect_numeric_entries(nodes, delta_field)
      displacement_peak = peak_displacement_entry(nodes, config)

      stress_peak = peak_scalar_entry(elements, stress_field)

      diagnostics =
        %{
          "#{prefix}_node_count" => length(Enum.filter(nodes, &is_map/1)),
          "#{prefix}_element_count" => length(Enum.filter(elements, &is_map/1))
        }
        |> maybe_merge_temperature_delta_stats(prefix, delta_nodes)
        |> maybe_merge_peak_magnitude(prefix, "displacement", displacement_peak)
        |> maybe_merge_peak_scalar(prefix, "stress", stress_peak)

      if map_size(diagnostics) <= 2 do
        {:error, :empty_thermo_diagnostics}
      else
        {:ok, diagnostics}
      end
    else
      _ -> {:error, :invalid_thermo_result}
    end
  end

  def export_alert_markdown(payload, config) when is_map(payload) and is_map(config) do
    fields =
      case Map.get(config, "fields") do
        entries when is_list(entries) -> Enum.filter(entries, &is_binary/1)
        _ -> nil
      end

    lines =
      [
        "# #{Map.get(config, "title", "Workflow Alert")}",
        "",
        "- Severity: #{resolve_alert_severity(payload, config)}",
        "- Summary: #{Map.get(config, "summary", "The workflow produced an alertable summary payload.")}"
      ] ++ alert_field_lines(payload, fields) ++ build_alert_samples_section(payload, config)

    {:ok,
     %{
       "format" => "markdown",
       "content_type" => "text/markdown",
       "content" => Enum.join(lines, "\n")
     }}
  end

  defp alert_field_lines(payload, fields) when is_list(fields) do
    Enum.flat_map(fields, fn field ->
      case Map.fetch(payload, field) do
        {:ok, value} -> ["- #{field}: #{markdown_value(value)}"]
        :error -> []
      end
    end)
  end

  defp alert_field_lines(payload, _fields),
    do: Enum.map(payload, fn {key, value} -> "- #{key}: #{markdown_value(value)}" end)

  defp fetch_list(map, key) when is_map(map) and is_binary(key) do
    case Map.get(map, key) do
      value when is_list(value) -> {:ok, value}
      _ -> :error
    end
  end

  defp fetch_numeric_field(map, field) when is_map(map) and is_binary(field) do
    case Map.get(map, field) do
      value when is_number(value) -> {:ok, value * 1.0}
      _ -> :error
    end
  end

  defp collect_numeric_entries(items, field) when is_list(items) and is_binary(field) do
    Enum.flat_map(items, fn
      %{} = item ->
        case fetch_numeric_field(item, field) do
          {:ok, value} -> [{Map.get(item, "id"), value}]
          :error -> []
        end

      _ ->
        []
    end)
  end

  defp peak_scalar_entry(items, field) when is_list(items) and is_binary(field) do
    items
    |> collect_numeric_entries(field)
    |> Enum.max_by(&elem(&1, 1), fn -> nil end)
  end

  defp peak_vector_entry(items, x_field, y_field, z_field) when is_list(items) and is_binary(x_field) do
    Enum.reduce(items, nil, fn
      %{} = item, best ->
        components =
          [x_field, y_field, z_field]
          |> Enum.filter(&is_binary/1)
          |> Enum.map(&Map.get(item, &1))

        if components != [] and Enum.all?(components, &is_number/1) do
          magnitude =
            components
            |> Enum.map(&(&1 * 1.0))
            |> Enum.map(&(&1 * &1))
            |> Enum.sum()
            |> :math.sqrt()

          candidate = {Map.get(item, "id"), magnitude}

          case best do
            nil -> candidate
            {_id, best_value} when magnitude > best_value -> candidate
            _ -> best
          end
        else
          best
        end
      _item, best ->
        best
    end)
  end

  defp maybe_merge_temperature_stats(summary, _prefix, []), do: summary

  defp maybe_merge_temperature_stats(summary, prefix, temperature_nodes) do
    values = Enum.map(temperature_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(temperature_nodes, &elem(&1, 1))
    {min_id, min_value} = Enum.min_by(temperature_nodes, &elem(&1, 1))

    summary
    |> Map.put("#{prefix}_temperature_min", min_value)
    |> Map.put("#{prefix}_temperature_min_node_id", min_id)
    |> Map.put("#{prefix}_temperature_max", peak_value)
    |> Map.put("#{prefix}_temperature_max_node_id", peak_id)
    |> Map.put("#{prefix}_temperature_span", peak_value - min_value)
    |> Map.put("#{prefix}_temperature_mean", Enum.sum(values) / length(values))
  end

  defp maybe_merge_load_stats(summary, _prefix, []), do: summary

  defp maybe_merge_load_stats(summary, prefix, heat_load_nodes) do
    values = Enum.map(heat_load_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(heat_load_nodes, &abs(elem(&1, 1)))
    loaded_count = Enum.count(values, &(&1 != 0.0))

    summary
    |> Map.put("#{prefix}_loaded_node_count", loaded_count)
    |> Map.put("#{prefix}_total_heat_load", Enum.sum(values))
    |> Map.put("#{prefix}_peak_heat_load", peak_value)
    |> Map.put("#{prefix}_peak_heat_load_node_id", peak_id)
  end

  defp maybe_merge_temperature_delta_stats(summary, _prefix, []), do: summary

  defp maybe_merge_temperature_delta_stats(summary, prefix, delta_nodes) do
    values = Enum.map(delta_nodes, &elem(&1, 1))
    {peak_id, peak_value} = Enum.max_by(delta_nodes, &elem(&1, 1))
    {min_id, min_value} = Enum.min_by(delta_nodes, &elem(&1, 1))

    summary
    |> Map.put("#{prefix}_temperature_delta_min", min_value)
    |> Map.put("#{prefix}_temperature_delta_min_node_id", min_id)
    |> Map.put("#{prefix}_temperature_delta_max", peak_value)
    |> Map.put("#{prefix}_temperature_delta_max_node_id", peak_id)
    |> Map.put("#{prefix}_temperature_delta_span", peak_value - min_value)
    |> Map.put("#{prefix}_temperature_delta_mean", Enum.sum(values) / length(values))
    |> Map.put("#{prefix}_heated_node_count", Enum.count(values, &(&1 != 0.0)))
  end

  defp maybe_merge_peak_vector(summary, _prefix, _name, nil), do: summary

  defp maybe_merge_peak_vector(summary, prefix, name, {id, magnitude}) do
    summary
    |> Map.put("#{prefix}_peak_#{name}_magnitude", magnitude)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
  end

  defp maybe_merge_peak_scalar(summary, _prefix, _name, nil), do: summary

  defp maybe_merge_peak_scalar(summary, prefix, name, {id, value}) do
    summary
    |> Map.put("#{prefix}_peak_#{name}", value)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
  end

  defp maybe_merge_peak_magnitude(summary, _prefix, _name, nil), do: summary

  defp maybe_merge_peak_magnitude(summary, prefix, name, {id, value}) do
    summary
    |> Map.put("#{prefix}_peak_#{name}", value)
    |> Map.put("#{prefix}_peak_#{name}_id", id)
  end

  defp peak_displacement_entry(nodes, config) do
    peak_vector_entry(
      nodes,
      Map.get(config, "displacement_x_field", "displacement_x"),
      Map.get(config, "displacement_y_field", "displacement_y"),
      Map.get(config, "displacement_z_field")
    ) || peak_scalar_entry(nodes, Map.get(config, "displacement_field", "displacement_magnitude"))
  end

  defp resolve_hotspot_threshold(items, field, config) do
    values =
      items
      |> Enum.filter(&is_map/1)
      |> Enum.flat_map(fn item ->
        case fetch_numeric_field(item, field) do
          {:ok, value} -> [value]
          :error -> []
        end
      end)

    cond do
      values == [] ->
        {:error, :missing_hotspot_values}

      is_number(Map.get(config, "threshold")) ->
        {:ok, Map.get(config, "threshold") * 1.0}

      is_number(Map.get(config, "percentile")) ->
        {:ok, interpolate_percentile(values, Map.get(config, "percentile") * 1.0)}

      true ->
        {:ok, Enum.max(values)}
    end
  end

  defp build_alert_samples_section(payload, config) do
    case Map.get(payload, Map.get(config, "sample_field", "field_hotspot_samples")) do
      samples when is_list(samples) and samples != [] ->
        sample_value_key = Map.get(config, "sample_value_key", "electric_field_magnitude")
        sample_id_key = Map.get(config, "sample_id_key", "id")
        sample_count = min(Map.get(config, "sample_count", 3), 16)

        ["", "## Sample Context"] ++
          Enum.flat_map(Enum.take(samples, sample_count), fn
            %{} = sample ->
              [
                "- #{markdown_value(Map.get(sample, sample_id_key, "unknown"))}: #{sample_value_key}=#{markdown_value(Map.get(sample, sample_value_key, "n/a"))}"
              ]

            _ ->
              []
          end)

      _ ->
        []
    end
  end

  defp resolve_alert_severity(payload, config) do
    cond do
      is_binary(Map.get(config, "severity")) and String.trim(Map.get(config, "severity")) != "" ->
        Map.get(config, "severity")

      is_binary(Map.get(config, "severity_path")) ->
        case resolve_path_value(payload, Map.get(config, "severity_path")) do
          value when is_binary(value) -> value
          _ -> "warning"
        end

      true ->
        "warning"
    end
  end

  defp resolve_path_value(payload, path) when is_map(payload) and is_binary(path) do
    path
    |> String.split(".", trim: true)
    |> Enum.reduce_while(payload, fn segment, current ->
      cond do
        is_map(current) -> {:cont, Map.get(current, segment)}
        is_list(current) -> {:cont, Enum.at(current, parse_index(segment))}
        true -> {:halt, nil}
      end
    end)
  end

  defp parse_index(segment) do
    case Integer.parse(segment) do
      {index, ""} -> index
      _ -> -1
    end
  end

  defp normalize_percentiles(percentiles) when is_list(percentiles) do
    percentiles
    |> Enum.filter(&is_number/1)
    |> Enum.map(&(&1 * 1.0))
    |> Enum.filter(&(&1 >= 0.0 and &1 <= 100.0))
  end

  defp normalize_percentiles(_percentiles), do: []

  defp population_stddev(values, mean) do
    values
    |> Enum.map(fn value -> (value - mean) * (value - mean) end)
    |> Enum.sum()
    |> Kernel./(length(values))
    |> :math.sqrt()
  end

  defp interpolate_percentile(values, percentile) do
    sorted = Enum.sort(values)

    if length(sorted) == 1 do
      hd(sorted)
    else
      position = percentile / 100.0 * (length(sorted) - 1)
      lower_index = floor(position)
      upper_index = ceil(position)
      lower_value = Enum.at(sorted, lower_index)
      upper_value = Enum.at(sorted, upper_index)

      if lower_index == upper_index do
        lower_value
      else
        weight = position - lower_index
        lower_value * (1.0 - weight) + upper_value * weight
      end
    end
  end

  defp format_percentile_key(percentile) do
    rounded_string =
      percentile
      |> Float.round(2)
      |> :erlang.float_to_binary(decimals: 2)
      |> String.trim_trailing("0")
      |> String.trim_trailing(".")

    "p#{String.replace(rounded_string, ".", "_")}"
  end

  defp sort_hotspots(hotspots, "value_asc"), do: Enum.sort_by(hotspots, &elem(&1, 0))
  defp sort_hotspots(hotspots, _sort), do: Enum.sort_by(hotspots, &(-elem(&1, 0)))

  defp normalize_prefix(prefix) when is_binary(prefix) do
    trimmed = String.trim(prefix)
    if trimmed == "", do: "summary", else: trimmed
  end

  defp markdown_value(value) when is_binary(value), do: value
  defp markdown_value(value) when is_number(value) or is_boolean(value), do: to_string(value)
  defp markdown_value(nil), do: "null"
  defp markdown_value(value), do: Jason.encode!(value)

  defp csv_escape(nil), do: ""

  defp csv_escape(value) when is_binary(value) do
    escaped = String.replace(value, "\"", "\"\"")
    if String.contains?(escaped, [",", "\"", "\n", "\r"]), do: ~s("#{escaped}"), else: escaped
  end

  defp csv_escape(value), do: value |> to_string() |> csv_escape()
end
