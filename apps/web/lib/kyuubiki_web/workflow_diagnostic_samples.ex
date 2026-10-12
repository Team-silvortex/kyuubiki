defmodule KyuubikiWeb.WorkflowDiagnosticSamples do
  @moduledoc false

  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def extract(payload, config, domain, groups, populate) do
    Check.run("extract.#{domain}_result_diagnostics", fn ->
      payload = Check.object(payload, "payload")
      Check.converged(payload, "payload")
      config = Check.config(config)

      prefix =
        config
        |> Check.text("output_prefix", "config", domain)
        |> String.replace(~r/[^a-zA-Z0-9]/u, "_")
        |> String.trim("_")

      if prefix == "",
        do: Check.fail("config.output_prefix must contain an ASCII letter or digit")

      nodes = samples(payload, config, "node_source", "nodes")
      elements = samples(payload, config, "element_source", "elements")

      summary = %{
        "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
        "diagnostic_domain" => domain,
        "diagnostic_subject" => "#{domain}_result",
        "diagnostic_prefix" => prefix,
        "diagnostic_metric_groups" => groups,
        "diagnostic_node_count" => length(elem(nodes, 1)),
        "diagnostic_element_count" => length(elem(elements, 1)),
        "#{prefix}_node_count" => length(elem(nodes, 1)),
        "#{prefix}_element_count" => length(elem(elements, 1))
      }

      context = %{prefix: prefix, nodes: nodes, elements: elements, summary: summary, measured: 0}
      context = populate.(context, config)
      if context.measured == 0, do: Check.fail("did not find any diagnostic fields")
      context.summary
    end)
  end

  def field(config, key, defaults) do
    key = if is_list(key), do: Enum.find(key, &Map.has_key?(config, &1)) || hd(key), else: key

    case Check.text(config, key, "config") do
      nil -> %{names: defaults, required: false}
      name -> %{names: [name], required: true}
    end
  end

  def distribution(context, source, label, field, opts \\ []) do
    name = "#{context.prefix}_#{label}"

    initial = %{
      count: 0,
      nonzero_count: 0,
      min: nil,
      max: nil,
      sum: 0.0,
      min_id: nil,
      max_id: nil,
      peak: nil
    }

    observe = fn value, id, stats ->
      %{
        count: stats.count + 1,
        nonzero_count: stats.nonzero_count + if(value == 0, do: 0, else: 1),
        min: if(stats.min == nil, do: value, else: min(stats.min, value)),
        max: if(stats.max == nil, do: value, else: max(stats.max, value)),
        sum: Check.calculate("#{name}_sum", fn -> stats.sum + value end),
        min_id: if(stats.min == nil or value < stats.min, do: identity(id), else: stats.min_id),
        max_id: if(stats.max == nil or value > stats.max, do: identity(id), else: stats.max_id),
        peak:
          if(stats.peak == nil or abs(value) >= abs(stats.peak.value),
            do: %{id: identity(id), value: value},
            else: stats.peak
          )
      }
    end

    stats =
      scan(context[source], label(field), field.required, initial, &read(&1, field, &2), observe)

    if stats.count == 0 do
      context
    else
      stats =
        stats
        |> Map.put(:mean, stats.sum / stats.count)
        |> Map.put(:span, Check.calculate("#{name}_span", fn -> stats.max - stats.min end))

      values =
        stats
        |> Map.take(~w(count nonzero_count min max sum mean span)a)
        |> Map.new(fn {key, value} -> {"#{name}_#{key}", value} end)

      values =
        if Keyword.get(opts, :node_ids, false) do
          Map.merge(values, %{
            "#{name}_min_node_id" => stats.min_id,
            "#{name}_max_node_id" => stats.max_id
          })
        else
          values
        end

      values =
        if Keyword.get(opts, :load_peak, false) do
          Map.merge(values, %{
            "#{context.prefix}_peak_#{label}" => stats.peak.value,
            "#{context.prefix}_peak_#{label}_node_id" => stats.peak.id
          })
        else
          values
        end

      measured(context, values)
    end
  end

  def scalar_peak(context, label, field, fallback \\ nil) do
    observe = fn value, id, best ->
      if best == nil or value >= best.value, do: %{id: id, value: value}, else: best
    end

    peak =
      scan(context.elements, label(field), field.required, nil, &read(&1, field, &2), observe)

    cond do
      peak != nil ->
        scalar_result(context, label, peak)

      fallback != nil ->
        scalar_peak(context, label, fallback)

      true ->
        context
    end
  end

  def vector_peak(context, label, fields, source \\ :elements) do
    required = Enum.any?(fields, fn {_axis, field} -> field.required end)

    peak =
      scan(context[source], label, required, nil, &vector(&1, fields, &2), fn sample, id, best ->
        if best == nil or sample.magnitude >= best.magnitude,
          do: Map.put(sample, :id, identity(id)),
          else: best
      end)

    if peak == nil do
      context
    else
      prefix = context.prefix

      values =
        peak_identity(prefix, label, peak.id, peak.magnitude)
        |> Map.put("#{prefix}_#{label}_peak_magnitude", peak.magnitude)
        |> Map.put("#{prefix}_peak_#{label}_magnitude", peak.magnitude)

      values =
        Enum.reduce(peak.components, values, fn {axis, value}, values ->
          values
          |> Map.put("#{prefix}_#{label}_peak_#{axis}", value)
          |> Map.put("#{prefix}_peak_#{label}_#{axis}", value)
        end)

      measured(context, values)
    end
  end

  def component_peak(context, label, names) do
    peak =
      Enum.reduce(names, nil, fn name, best ->
        field = %{names: [name], required: false}

        scan(context.elements, name, false, best, &read(&1, field, &2), fn value, id, best ->
          if best == nil or abs(value) >= abs(best.value), do: %{id: id, value: value}, else: best
        end)
      end)

    if peak == nil, do: context, else: scalar_result(context, label, peak)
  end

  def payload_scalar_peak(context, payload, label, field) do
    case Map.fetch(payload, field) do
      :error ->
        context

      {:ok, value} ->
        scalar_result(context, label, %{id: field, value: Check.number(value, "payload.#{field}")})
    end
  end

  def has_metric?(context, name), do: Map.has_key?(context.summary, "#{context.prefix}_#{name}")

  def alias_metric(context, source, target) do
    case Map.fetch(context.summary, "#{context.prefix}_#{source}") do
      :error ->
        context

      {:ok, value} ->
        %{context | summary: Map.put(context.summary, "#{context.prefix}_#{target}", value)}
    end
  end

  defp scalar_result(context, label, peak) do
    values =
      peak_identity(context.prefix, label, identity(peak.id), peak.value)
      |> Map.put("#{context.prefix}_#{label}_peak", peak.value)

    values =
      if peak.id == :missing do
        Map.put(
          values,
          "#{context.prefix}_peak_#{label}_id",
          Map.get(context, :scalar_id_fallback)
        )
      else
        values
      end

    measured(context, values)
  end

  defp identity(:missing), do: nil
  defp identity(value), do: value

  defp samples(payload, config, key, default) do
    source = Check.text(config, key, "config", default)
    {source, Check.list(payload[source], "payload.#{source}")}
  end

  # Optional groups may be wholly absent, never partially measured. Retain only
  # reduction state and the first missing index, not filtered copies of all rows.
  defp scan({source, rows}, label, required, initial, read, observe) do
    {count, missing, result, _index} =
      Enum.reduce(rows, {0, nil, initial, 0}, fn row, {count, missing, result, index} ->
        path = "payload.#{source}[#{index}]"
        row = Check.object(row, path)

        case read.(row, path) do
          nil ->
            {count, missing || index, result, index + 1}

          value ->
            {count + 1, missing, observe.(value, Map.get(row, "id", :missing), result), index + 1}
        end
      end)

    if count > 0 or required do
      if missing != nil, do: missing("payload.#{source}[#{missing}]", label)
      if count == 0, do: Check.fail("payload.#{source} has no samples for #{label}")
    end

    result
  end

  defp read(row, field, path) do
    value =
      Enum.reduce_while(field.names, nil, fn name, _acc ->
        case Map.fetch(row, name) do
          :error -> {:cont, nil}
          {:ok, value} -> {:halt, Check.number(value, "#{path}.#{name}")}
        end
      end)

    if value == nil and field.required, do: missing(path, label(field))
    value
  end

  defp vector(row, fields, path) do
    magnitude = read(row, fields.magnitude, path)

    components =
      Enum.reduce([:x, :y, :z], %{}, fn axis, components ->
        case read(row, fields[axis], path) do
          nil -> components
          value -> Map.put(components, axis, value)
        end
      end)

    cond do
      magnitude != nil ->
        if magnitude < 0 do
          selected = Enum.find(fields.magnitude.names, &Map.has_key?(row, &1))
          Check.fail("#{path}.#{selected} must be nonnegative")
        end

        %{magnitude: magnitude, components: components}

      map_size(components) == 0 ->
        nil

      true ->
        x = components[:x] || missing(path, label(fields.x))
        y = components[:y] || missing(path, label(fields.y))
        norm = Check.hypot(x, y, "#{path} vector magnitude")

        norm =
          if Map.has_key?(components, :z),
            do: Check.hypot(norm, components.z, "#{path} vector magnitude"),
            else: norm

        %{magnitude: norm, components: components}
    end
  end

  defp label(field), do: List.first(field.names) || "metric"
  defp missing(path, field), do: Check.fail("#{path}.#{field} is missing a numeric sample")

  defp measured(context, values),
    do: %{context | summary: Map.merge(context.summary, values), measured: context.measured + 1}

  defp peak_identity(prefix, label, id, value),
    do: %{
      "#{prefix}_peak_#{label}" => value,
      "#{prefix}_peak_#{label}_id" => id,
      "#{prefix}_#{label}_peak_element_id" => id,
      "#{prefix}_peak_#{label}_element_id" => id
    }
end
