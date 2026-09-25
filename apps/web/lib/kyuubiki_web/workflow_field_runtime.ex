defmodule KyuubikiWeb.WorkflowFieldRuntime do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def statistics(payload, config) do
    extract("extract.field_statistics", payload, config, "nodes", fn samples, config, prefix ->
      percentiles = percentiles(config)
      values = Enum.map(samples, &elem(&1, 0))
      count = length(values)

      sum =
        Enum.reduce(values, 0.0, fn value, sum ->
          Check.calculate("#{prefix}_sum", fn -> sum + value end)
        end)

      mean = sum / count
      root_count = :math.sqrt(count)

      stddev =
        Enum.reduce(values, 0.0, fn value, acc ->
          delta = scaled_deviation(value, mean, root_count)
          Check.hypot(acc, delta, "#{prefix}_stddev")
        end)

      summary = %{
        "#{prefix}_count" => count,
        "#{prefix}_min" => Enum.min(values),
        "#{prefix}_max" => Enum.max(values),
        "#{prefix}_sum" => sum,
        "#{prefix}_mean" => mean,
        "#{prefix}_stddev" => stddev
      }

      if percentiles == [] do
        summary
      else
        sorted = values |> Enum.sort() |> List.to_tuple()

        Enum.reduce(percentiles, summary, fn p, output ->
          Map.put(output, "#{prefix}_#{percentile_key(p)}", interpolate(sorted, p))
        end)
      end
    end)
  end

  def hotspots(payload, config) do
    extract("extract.field_hotspots", payload, config, "elements", fn samples, config, prefix ->
      explicit = Check.optional_number(config, "threshold", "config")
      percentile = config |> Map.get("percentile", 90) |> percentile("config.percentile")

      limit =
        config |> Map.get("sample_limit", 8) |> Check.unsigned("config.sample_limit") |> min(32)

      order = Check.choice(config, "sample_sort", "value_desc", ["value_desc", "value_asc"])

      threshold =
        if is_nil(explicit) do
          samples
          |> Enum.map(&elem(&1, 0))
          |> Enum.sort()
          |> List.to_tuple()
          |> interpolate(percentile)
        else
          explicit
        end

      hotspots =
        samples
        |> Enum.filter(fn {value, _} -> value >= threshold end)
        |> Enum.sort_by(
          fn {value, _} -> {value, zero_rank(value)} end,
          if(order == "value_asc", do: :asc, else: :desc)
        )

      if hotspots == [], do: Check.fail("did not find any values meeting the threshold")

      mean =
        hotspots
        |> Enum.with_index(1)
        |> Enum.reduce(0.0, fn
          {{value, _}, 1}, _ ->
            value

          {{value, _}, count}, mean ->
            Check.calculate("#{prefix}_hotspot_mean", fn -> update_mean(value, mean, count) end)
        end)

      %{
        "#{prefix}_threshold" => threshold,
        "#{prefix}_hotspot_mean" => mean,
        "#{prefix}_hotspot_max" => hotspots |> Enum.map(&elem(&1, 0)) |> Enum.max(),
        "#{prefix}_hotspot_count" => length(hotspots),
        "#{prefix}_hotspot_fraction" => length(hotspots) / length(samples),
        "#{prefix}_sample_sort" => order,
        "#{prefix}_hotspot_ids" =>
          Enum.flat_map(hotspots, fn {_, row} ->
            case Map.fetch(row, "id") do
              {:ok, id} -> [id]
              :error -> []
            end
          end),
        "#{prefix}_hotspot_samples" => hotspots |> Enum.take(limit) |> Enum.map(&elem(&1, 1))
      }
    end)
  end

  defp extract(operator, payload, config, default_source, reduce) do
    Check.run(operator, fn ->
      payload = Check.object(payload, "payload")
      Check.converged(payload, "payload")
      config = Check.config(config)
      source = Check.text(config, "source", "config", default_source)
      field = Check.text(config, "field", "config") || Check.fail("requires config.field")
      prefix = Check.text(config, "output_prefix", "config", field)
      rows = payload |> Map.get(source) |> Check.list("payload.#{source}")
      if rows == [], do: Check.fail("payload.#{source} must contain samples")

      samples =
        rows
        |> Enum.with_index()
        |> Enum.map(fn {row, index} ->
          path = "payload.#{source}[#{index}]"
          row = Check.object(row, path)
          {Check.number(row[field], "#{path}.#{field}"), row}
        end)

      reduce.(samples, config, prefix)
      |> Map.merge(%{"source_collection" => source, "source_field" => field})
    end)
  end

  defp percentile(value, path) do
    value = Check.number(value, path)
    if value < 0 or value > 100, do: Check.fail("#{path} must be between 0 and 100")
    if value == 0, do: 0.0, else: value
  end

  defp percentiles(config) do
    {values, _seen} =
      config
      |> Map.get("percentiles", [])
      |> Check.list("config.percentiles")
      |> Enum.with_index()
      |> Enum.map_reduce(MapSet.new(), fn {value, index}, seen ->
        path = "config.percentiles[#{index}]"
        value = percentile(value, path)
        key = percentile_key(value)
        if MapSet.member?(seen, key), do: Check.fail("#{path} duplicates a percentile output")
        {value, MapSet.put(seen, key)}
      end)

    values
  end

  # Retain all significant digits and expand scientific notation to match Rust keys.
  defp percentile_key(value) do
    text =
      case String.split(Float.to_string(value), "e") do
        [plain] ->
          String.trim_trailing(plain, ".0")

        [mantissa, exponent] ->
          [whole, fraction] = String.split(mantissa, ".")
          digits = whole <> fraction
          position = byte_size(whole) + String.to_integer(exponent)

          if position <= 0 do
            "0." <> String.duplicate("0", -position) <> String.trim_trailing(digits, "0")
          else
            {left, right} = String.split_at(digits, position)
            left <> "." <> String.trim_trailing(right, "0")
          end
      end

    "p" <> String.replace(text, ".", "_")
  end

  defp interpolate(sorted, percentile) do
    position = percentile / 100.0 * (tuple_size(sorted) - 1)
    low = elem(sorted, floor(position))
    high = elem(sorted, ceil(position))
    weight = position - floor(position)

    Check.calculate("percentile", fn ->
      cond do
        low == high -> low
        low < 0 == high < 0 -> low + (high - low) * weight
        true -> low * (1.0 - weight) + high * weight
      end
    end)
  end

  defp scaled_deviation(value, mean, root_count) do
    (value - mean) / root_count
  rescue
    ArithmeticError -> Check.calculate("stddev", fn -> value / root_count - mean / root_count end)
  end

  defp update_mean(value, mean, count) do
    mean + (value - mean) / count
  rescue
    ArithmeticError -> mean * ((count - 1.0) / count) + value / count
  end

  defp zero_rank(value) when value == 0 do
    <<sign::1, _::63>> = <<value::float>>
    1 - sign
  end

  defp zero_rank(_value), do: 1
end
