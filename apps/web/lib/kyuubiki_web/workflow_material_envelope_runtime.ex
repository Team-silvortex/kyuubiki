defmodule KyuubikiWeb.WorkflowMaterialEnvelopeRuntime do
  @moduledoc false

  def compose_material_study_envelope(payload, config) when is_map(payload) and is_map(config) do
    case material_envelope_entries(payload) do
      {:ok, nil} -> compose_single_material_study_envelope(payload, config)
      {:ok, []} -> {:error, :missing_material_envelope_candidates}
      {:ok, entries} -> compose_material_study_envelope_batch(entries, config)
      {:error, _reason} = error -> error
    end
  end

  def compose_material_study_envelope(_payload, _config),
    do: {:error, :invalid_material_envelope_payload}

  defp compose_material_study_envelope_batch(entries, config) do
    prefix = Map.get(config, "output_prefix", "material_envelope")

    with {:ok, envelopes} <-
           entries
           |> Enum.map(fn {candidate_id, row} ->
             with {:ok, envelope} <-
                    compose_single_material_study_envelope(
                      Map.put_new(row, "candidate_id", candidate_id),
                      config
                    ) do
               {:ok, {candidate_id, envelope}}
             end
           end)
           |> collect_ok() do
      envelope_rows = Enum.map(envelopes, fn {_id, envelope} -> envelope end)
      best = best_material_envelope(envelope_rows, prefix)

      {:ok,
       %{
         "#{prefix}_batch_contract" => "kyuubiki.material_study_envelope_batch/v1",
         "#{prefix}_candidate_count" => length(envelopes),
         "#{prefix}_best_candidate_id" => Map.get(best, "#{prefix}_candidate_id", ""),
         "candidates" => Map.new(envelopes),
         "envelopes" => envelope_rows
       }}
    end
  end

  defp compose_single_material_study_envelope(payload, config) do
    prefix = Map.get(config, "output_prefix", "material_envelope")

    with {:ok, specs} <- material_envelope_metric_specs(config) do
      metrics = material_envelope_metrics(payload, specs)

      if metrics == [] do
        {:error, :missing_material_envelope_metrics}
      else
        critical = Enum.max_by(metrics, & &1.failure_index)
        score = Enum.reduce(metrics, 0.0, &(&1.failure_index * &1.weight + &2))
        violations = Enum.count(metrics, &(&1.failure_index > 1.0))
        domains = metrics |> Enum.map(& &1.source) |> Enum.uniq() |> Enum.sort()

        {:ok,
         %{
           "#{prefix}_contract" => "kyuubiki.material_study_envelope/v1",
           "#{prefix}_candidate_id" => material_envelope_candidate_id(payload, config),
           "#{prefix}_domain_count" => length(domains),
           "#{prefix}_domains" => domains,
           "#{prefix}_metric_count" => length(metrics),
           "#{prefix}_violation_count" => violations,
           "#{prefix}_status" => if(violations == 0, do: "pass", else: "fail"),
           "#{prefix}_score" => score,
           "#{prefix}_failure_index" => critical.failure_index,
           "#{prefix}_safety_factor" => 1.0 / critical.failure_index,
           "#{prefix}_critical_metric" => "#{critical.source}.#{critical.alias}",
           "#{prefix}_critical_actual" => critical.actual,
           "#{prefix}_critical_limit" => critical.limit,
           "#{prefix}_metrics" => Enum.map(metrics, &material_envelope_metric_summary/1)
         }}
      end
    end
  end

  defp material_envelope_metrics(payload, specs) do
    Enum.flat_map(specs, fn spec ->
      with actual when is_number(actual) <- material_envelope_metric_value(payload, spec),
           limit when is_number(limit) <- spec.limit,
           true <- limit > 0.0 do
        failure_index = material_envelope_failure_index(actual, limit, spec.direction)
        [spec |> Map.put(:actual, actual) |> Map.put(:failure_index, failure_index)]
      else
        _ -> []
      end
    end)
  end

  defp material_envelope_entries(%{"rows" => rows}) when is_list(rows) do
    rows
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, []}, fn {row, index}, {:ok, acc} ->
      if is_map(row),
        do: {:cont, {:ok, [{entry_candidate_id(row, index), row} | acc]}},
        else: {:halt, {:error, :invalid_material_envelope_row}}
    end)
    |> reverse_ok()
  end

  defp material_envelope_entries(%{"candidates" => candidates}) when is_map(candidates) do
    candidates
    |> Enum.reduce_while({:ok, []}, fn {candidate_id, candidate}, {:ok, acc} ->
      if is_map(candidate),
        do: {:cont, {:ok, [{candidate_id, candidate} | acc]}},
        else: {:halt, {:error, :invalid_material_envelope_candidate}}
    end)
    |> reverse_ok()
  end

  defp material_envelope_entries(_payload), do: {:ok, nil}

  defp best_material_envelope(envelopes, prefix) do
    envelopes
    |> Enum.filter(&(Map.get(&1, "#{prefix}_status") == "pass"))
    |> Enum.sort_by(&Map.get(&1, "#{prefix}_score", :infinity))
    |> List.first()
    |> case do
      nil ->
        envelopes
        |> Enum.sort_by(&Map.get(&1, "#{prefix}_failure_index", :infinity))
        |> List.first(%{})

      envelope ->
        envelope
    end
  end

  defp material_envelope_metric_specs(%{"metrics" => metrics})
       when is_list(metrics) and metrics != [] do
    metrics
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, []}, fn {metric, index}, {:ok, acc} ->
      field = Map.get(metric, "field")

      if is_binary(field) do
        {:cont, {:ok, [metric_spec(metric, field) | acc]}}
      else
        {:halt, {:error, {:invalid_material_envelope_metric, index}}}
      end
    end)
    |> reverse_ok()
  end

  defp material_envelope_metric_specs(%{"metrics" => []}),
    do: {:error, :empty_material_envelope_metrics}

  defp material_envelope_metric_specs(_config), do: {:ok, default_material_envelope_specs()}

  defp metric_spec(metric, field) do
    %{
      source: Map.get(metric, "source", "summary"),
      field: field,
      alias: Map.get(metric, "alias", field),
      limit: Map.get(metric, "limit") || Map.get(metric, "max") || Map.get(metric, "min"),
      direction: Map.get(metric, "direction", "max"),
      weight: material_envelope_weight(metric)
    }
  end

  defp default_material_envelope_specs do
    [
      spec("thermal", "max_temperature", "temperature", 120.0, 2.0),
      spec("thermal", "max_heat_flux", "heat_flux", 30.0, 1.5),
      spec("structural", "max_stress", "stress", 250.0, 2.0),
      spec("structural", "max_displacement", "displacement", 0.01, 1.0),
      spec("electrostatic", "max_electric_field", "electric_field", 5.0, 1.5),
      spec("magnetostatic", "max_flux_density", "flux_density", 2.0, 1.5),
      spec("cfd", "fluid_reynolds_peak", "reynolds", 2000.0, 1.0)
    ]
  end

  defp spec(source, field, alias, limit, weight),
    do: %{
      source: source,
      field: field,
      alias: alias,
      limit: limit,
      direction: "max",
      weight: weight
    }

  defp material_envelope_weight(%{"weight" => weight}) when is_number(weight) and weight >= 0.0,
    do: weight * 1.0

  defp material_envelope_weight(_metric), do: 1.0

  defp material_envelope_metric_value(payload, spec) do
    source =
      get_in(payload, ["summaries", spec.source]) ||
        Map.get(payload, spec.source) ||
        if(spec.source == "summary", do: payload)

    lookup_number(source || %{}, spec.field) ||
      lookup_number(payload, "#{spec.source}.#{spec.field}") ||
      lookup_number(payload, spec.field)
  end

  defp lookup_number(payload, field) when is_map(payload) do
    case get_in(payload, String.split(field, ".")) ||
           get_in(payload, ["summary" | String.split(field, ".")]) do
      value when is_number(value) -> value * 1.0
      _ -> nil
    end
  end

  defp material_envelope_failure_index(actual, limit, "min"), do: limit / max(actual, 1.0e-12)
  defp material_envelope_failure_index(actual, limit, "abs"), do: abs(actual) / limit
  defp material_envelope_failure_index(actual, limit, _direction), do: actual / limit

  defp material_envelope_candidate_id(payload, config),
    do:
      Map.get(config, "candidate_id") || Map.get(payload, "candidate_id") ||
        Map.get(payload, "id") || "candidate"

  defp entry_candidate_id(row, index),
    do:
      Map.get(row, "candidate_id") || Map.get(row, "case_id") || Map.get(row, "id") ||
        "candidate_#{index + 1}"

  defp material_envelope_metric_summary(metric) do
    %{
      "source" => metric.source,
      "field" => metric.field,
      "alias" => metric.alias,
      "actual" => metric.actual,
      "limit" => metric.limit,
      "direction" => metric.direction,
      "weight" => metric.weight,
      "failure_index" => metric.failure_index,
      "safety_factor" => 1.0 / metric.failure_index,
      "weighted_score" => metric.failure_index * metric.weight,
      "status" => if(metric.failure_index <= 1.0, do: "pass", else: "fail")
    }
  end

  defp collect_ok(values) do
    Enum.reduce_while(values, {:ok, []}, fn
      {:ok, value}, {:ok, acc} -> {:cont, {:ok, [value | acc]}}
      {:error, _reason} = error, _acc -> {:halt, error}
    end)
    |> reverse_ok()
  end

  defp reverse_ok({:ok, values}), do: {:ok, Enum.reverse(values)}
  defp reverse_ok({:error, _reason} = error), do: error
end
