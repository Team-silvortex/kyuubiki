defmodule KyuubikiWeb.WorkflowReportingRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowSummaryRuntime
  alias KyuubikiWeb.WorkflowElectrostaticRuntime
  alias KyuubikiWeb.WorkflowThermalRuntime

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

  def extract_thermal_result_diagnostics(payload, config),
    do: WorkflowThermalRuntime.extract_thermal_result_diagnostics(payload, config)

  def extract_electrostatic_result_diagnostics(payload, config),
    do: WorkflowElectrostaticRuntime.extract_electrostatic_result_diagnostics(payload, config)

  def extract_thermo_result_diagnostics(payload, config),
    do: WorkflowThermalRuntime.extract_thermo_result_diagnostics(payload, config)

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

  def export_diagnostics_bundle_markdown(payload, config)
      when is_map(payload) and is_map(config) do
    lines =
      [
        "# #{Map.get(config, "title", "Workflow Diagnostics Bundle")}",
        "",
        "- Contract: #{Map.get(payload, "bundle_contract", "unknown")}",
        "- Sources: #{Map.get(payload, "bundle_source_count", 0)}",
        "- Domains: #{format_list(Map.get(payload, "bundle_domains", []))}",
        "- Subjects: #{format_list(Map.get(payload, "bundle_subjects", []))}",
        "- Total Nodes: #{Map.get(payload, "bundle_total_node_count", 0)}",
        "- Total Elements: #{Map.get(payload, "bundle_total_element_count", 0)}"
      ] ++
        build_bundle_metric_groups_section(payload) ++
        build_bundle_highlights_section(payload) ++
        build_bundle_items_section(payload, config) ++
        build_bundle_guard_section(payload, config)

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

  defp build_bundle_metric_groups_section(payload) do
    case Map.get(payload, "bundle_metric_groups", []) do
      groups when is_list(groups) and groups != [] ->
        ["", "## Metric Groups", "", "- " <> Enum.join(groups, ", ")]

      _ ->
        []
    end
  end

  defp build_bundle_highlights_section(payload) do
    case Map.get(payload, "report_highlights", []) do
      highlights when is_list(highlights) and highlights != [] ->
        ["", "## Key Highlights"] ++
          Enum.flat_map(highlights, fn
            %{} = highlight ->
              marker =
                if Map.get(highlight, "attention", false) do
                  "attention"
                else
                  "info"
                end

              [
                "- [#{marker}] #{Map.get(highlight, "label", Map.get(highlight, "id", "unknown"))}: #{markdown_value(Map.get(highlight, "value"))}"
              ]

            _ ->
              []
          end)

      _ ->
        []
    end
  end

  defp build_bundle_items_section(payload, config) do
    case Map.get(payload, "bundle_items", []) do
      items when is_list(items) and items != [] ->
        max_items = min(Map.get(config, "item_count", 8), 24)

        ["", "## Diagnostics Sources"] ++
          Enum.flat_map(Enum.take(items, max_items), fn
            %{} = item ->
              [
                "",
                "### #{Map.get(item, "source", "unknown")}",
                "- Domain: #{Map.get(item, "domain", "unknown")}",
                "- Subject: #{Map.get(item, "subject", "unknown")}",
                "- Prefix: #{Map.get(item, "prefix", "unknown")}",
                "- Nodes: #{Map.get(item, "node_count", 0)}",
                "- Elements: #{Map.get(item, "element_count", 0)}",
                "- Metric Groups: #{format_list(Map.get(item, "metric_groups", []))}"
              ]

            _ ->
              []
          end)

      _ ->
        []
    end
  end

  defp build_bundle_guard_section(payload, config) do
    guard_payload =
      case Map.get(payload, "guard_payload") || Map.get(config, "guard_payload") do
        %{} = explicit -> explicit
        _ -> payload
      end

    if Map.get(guard_payload, "guard_status") do
      [
        "",
        "## Guard Decision",
        "",
        "- Status: #{Map.get(guard_payload, "guard_status")}",
        "- Passed: #{markdown_value(Map.get(guard_payload, "guard_passed"))}",
        "- Recommendation: #{Map.get(guard_payload, "guard_recommendation", "continue")}",
        "- Summary: #{Map.get(guard_payload, "guard_summary", "No guard summary.")}"
      ] ++ build_bundle_guard_triggers(guard_payload)
    else
      []
    end
  end

  defp build_bundle_guard_triggers(guard_payload) do
    case Map.get(guard_payload, "guard_triggers", []) do
      triggers when is_list(triggers) and triggers != [] ->
        ["", "### Guard Triggers"] ++
          Enum.flat_map(triggers, fn
            %{} = trigger ->
              [
                "- #{Map.get(trigger, "source", "bundle")}.#{Map.get(trigger, "label", Map.get(trigger, "field", "unknown"))}: #{markdown_value(Map.get(trigger, "value"))} #{Map.get(trigger, "comparison", "gte")} #{markdown_value(Map.get(trigger, "threshold"))} (#{Map.get(trigger, "severity", "warn")})"
              ]

            _ ->
              []
          end)

      _ ->
        []
    end
  end

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

  defp format_list(entries) when is_list(entries) do
    entries
    |> Enum.filter(&is_binary/1)
    |> case do
      [] -> "none"
      values -> Enum.join(values, ", ")
    end
  end

  defp format_list(_entries), do: "none"

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
