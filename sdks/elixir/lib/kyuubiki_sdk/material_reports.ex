defmodule KyuubikiSdk.MaterialReports do
  @moduledoc "Material-study catalog and headless result report helpers."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.MaterialReportCatalog

  def material_study_catalog do
    {:ok, Enum.map(MaterialReportCatalog.study_order(), &MaterialReportCatalog.catalog_entry/1)}
  end

  def describe_material_study(study) when is_binary(study) do
    case resolve_study_id(study) do
      nil -> {:error, Error.validation(["unsupported material report study: #{study}"])}
      study_id -> {:ok, MaterialReportCatalog.catalog_entry(study_id)}
    end
  end

  def describe_material_study(_study),
    do: {:error, Error.validation(["material report study must be a string"])}

  def extract_material_result_payloads(payload) when is_list(payload) do
    if Enum.all?(payload, &is_map/1) do
      {:ok, payload}
    else
      {:error, Error.validation(["material result payload entries must be objects"])}
    end
  end

  def extract_material_result_payloads(
        %{"schema_version" => "kyuubiki.headless-execution-run/v1"} = payload
      ) do
    results =
      payload
      |> Map.get("steps", [])
      |> Enum.flat_map(fn
        %{"action" => "result_fetch", "result_preview" => preview} = step when is_map(preview) ->
          if Map.get(step, "status") in ["blocked", "failed"] do
            []
          else
            [result_from_preview(preview)]
          end

        _step ->
          []
      end)

    if results == [] do
      {:error,
       Error.validation([
         "headless execution run report does not contain successful result_fetch payloads"
       ])}
    else
      {:ok, results}
    end
  end

  def extract_material_result_payloads(%{"results" => results}) when is_list(results),
    do: extract_material_result_payloads(results)

  def extract_material_result_payloads(%{"result_payloads" => results}) when is_list(results),
    do: extract_material_result_payloads(results)

  def extract_material_result_payloads(_payload) do
    {:error,
     Error.validation([
       "material report input must be an array, include a results array, or be a headless execution run report"
     ])}
  end

  def build_material_report(study, result_payloads, opts \\ [])

  def build_material_report(study, result_payloads, opts)
      when is_binary(study) and is_list(result_payloads) do
    with {:ok, study_id} <- resolved_study_id(study),
         {:ok, payloads} <- extract_material_result_payloads(result_payloads),
         {:ok, _count} <- require_payload_count(study_id, payloads) do
      candidates = MaterialReportCatalog.candidates(study_id)
      metrics = MaterialReportCatalog.metric_specs(study_id)
      profile = Keyword.get(opts, :optimization) || default_optimization(metrics)

      rows =
        candidates
        |> Enum.zip(payloads)
        |> Enum.map(fn {candidate, payload} ->
          candidate_report(study_id, candidate, payload, metrics, profile)
        end)
        |> Enum.sort_by(& &1["score"], :desc)
        |> Enum.with_index(1)
        |> Enum.map(fn {row, rank} -> Map.put(row, "rank", rank) end)

      warnings =
        Enum.flat_map(rows, fn row ->
          Enum.map(row["missing_metrics"], fn metric ->
            "#{row["candidate_id"]} is missing #{metric}; ranking used remaining weighted metrics"
          end)
        end)

      descriptor = MaterialReportCatalog.descriptor(study_id)

      {:ok,
       %{
         "schema_version" => descriptor["schema_version"],
         "study" => MaterialReportCatalog.study_alias(study_id),
         "objective" => descriptor["objective"],
         "optimization" => profile,
         "reliability" => reliability_envelope(study_id, rows),
         "metric_specs" => metrics,
         "candidates" => rows,
         "winner_candidate_id" => rows |> List.first() |> then(&(&1 && &1["candidate_id"])),
         "warnings" => warnings
       }}
    end
  end

  def build_material_report(_study, _result_payloads, _opts),
    do:
      {:error,
       Error.validation(["material report study must be a string and payloads must be a list"])}

  def build_material_report_from_payload(study, payload, opts \\ []) do
    with {:ok, payloads} <- extract_material_result_payloads(payload) do
      build_material_report(study, payloads, opts)
    end
  end

  defp resolved_study_id(study) do
    case resolve_study_id(study) do
      nil -> {:error, Error.validation(["unsupported material report study: #{study}"])}
      study_id -> {:ok, study_id}
    end
  end

  defp resolve_study_id(study) do
    normalized = normalize_study_key(study)

    Enum.find(MaterialReportCatalog.study_order(), fn study_id ->
      descriptor = MaterialReportCatalog.descriptor(study_id)
      keys = [study_id | descriptor["aliases"]]
      Enum.any?(keys, &(normalize_study_key(&1) == normalized))
    end)
  end

  defp normalize_study_key(study),
    do: study |> String.trim() |> String.replace(["-", "."], "_") |> String.downcase()

  defp require_payload_count(study_id, payloads) do
    expected = length(MaterialReportCatalog.candidates(study_id))

    if length(payloads) == expected do
      {:ok, expected}
    else
      {:error,
       Error.validation([
         "#{study_id} expects #{expected} result payloads, received #{length(payloads)}"
       ])}
    end
  end

  defp result_from_preview(%{"result" => result}) when is_map(result), do: result
  defp result_from_preview(preview), do: preview

  defp candidate_report(study_id, candidate, payload, specs, optimization) do
    result = descend_result_payload(payload)
    metrics = study_metrics(study_id, candidate, result)

    missing =
      specs
      |> Enum.reject(&(&1["direction"] == "observe"))
      |> Enum.flat_map(fn spec -> if is_nil(metrics[spec["id"]]), do: [spec["id"]], else: [] end)

    terms = optimization_terms(specs, metrics, optimization)
    score = Enum.reduce(terms, 0.0, fn term, acc -> acc + term["score"] * term["weight"] end)

    %{
      "candidate_id" => candidate["id"],
      "candidate_label" => candidate["label"],
      "rank" => 0,
      "score" => score,
      "metrics" => metrics,
      "optimization_terms" => terms,
      "missing_metrics" => missing
    }
  end

  defp reliability_envelope(study_id, rows) do
    gates = [result_completeness_gate(study_id, rows)]

    %{
      "schema_version" => "kyuubiki.material-reliability-envelope/v1",
      "posture" => "screening_only",
      "material_card_version" => "kyuubiki.material-cards.#{study_id}.v1",
      "unit_system" => "SI",
      "quality_gates" => gates,
      "summary" => reliability_summary(gates),
      "limitations" => [
        "Elixir SDK material reports use screening fixtures and should be rerun through solver-backed studies before qualification."
      ]
    }
  end

  defp result_completeness_gate(study_id, rows) do
    complete_count = Enum.count(rows, &(&1["missing_metrics"] == []))
    expected_count = length(rows)
    status = if complete_count >= expected_count, do: "pass", else: "violate"

    %{
      "id" => "gate.result_completeness",
      "label" => "Result payload completeness",
      "metric_id" => "complete_candidate_count",
      "operator" => ">=",
      "limit" => expected_count,
      "actual_value" => complete_count,
      "status" => status,
      "description" =>
        "Every #{study_id} candidate should expose required result metrics before ranking is trusted."
    }
  end

  defp reliability_summary(gates) do
    pass_count = Enum.count(gates, &(&1["status"] == "pass"))
    violation_count = Enum.count(gates, &(&1["status"] == "violate"))
    unknown_count = Enum.count(gates, &(&1["status"] == "unknown"))
    observe_count = length(gates) - pass_count - violation_count - unknown_count

    blocking_gate_ids =
      gates
      |> Enum.filter(&(&1["status"] == "violate"))
      |> Enum.map(& &1["id"])

    decision =
      cond do
        violation_count > 0 -> "blocked_by_quality_gates"
        unknown_count > 0 -> "needs_more_evidence"
        observe_count > 0 -> "review_observations"
        true -> "ready_for_next_round"
      end

    %{
      "decision" => decision,
      "total_gate_count" => length(gates),
      "pass_count" => pass_count,
      "violation_count" => violation_count,
      "unknown_count" => unknown_count,
      "observe_count" => observe_count,
      "blocking_gate_ids" => blocking_gate_ids
    }
  end

  defp descend_result_payload(%{"result" => result}) when is_map(result), do: result
  defp descend_result_payload(payload), do: payload

  defp study_metrics("material_heat_spreader_screening", candidate, result) do
    thickness = thickness(result)

    %{
      "peak_temperature_c" => float(result["max_temperature"]),
      "peak_heat_flux_w_m2" => float(result["max_heat_flux"]),
      "areal_mass_kg_m2" => candidate["density_kg_m3"] * thickness,
      "conductivity_density_ratio" =>
        candidate["thermal_conductivity_w_mk"] / candidate["density_kg_m3"]
    }
  end

  defp study_metrics("material_dielectric_screening", candidate, result) do
    thickness = thickness(result)
    field = float(result["max_electric_field"])

    %{
      "max_electric_field_v_m" => field,
      "breakdown_safety_factor" =>
        if(is_number(field) and field > 0, do: candidate["breakdown_field_v_m"] / field),
      "dielectric_loss_proxy" =>
        candidate["relative_permittivity"] * candidate["dissipation_factor"],
      "areal_mass_kg_m2" => candidate["density_kg_m3"] * thickness,
      "max_flux_density_c_m2" => float(result["max_flux_density"])
    }
  end

  defp study_metrics("material_thermo_shield_screening", candidate, result) do
    %{
      "max_stress_pa" => float(result["max_stress"]),
      "max_displacement_m" => float(result["max_displacement"]),
      "areal_mass_kg_m2" => candidate["density_kg_m3"] * thickness(result),
      "max_temperature_delta_k" => float(result["max_temperature_delta"]),
      "thermal_expansion_1_k" => candidate["thermal_expansion_1_k"]
    }
  end

  defp study_metrics("material_composite_thermo_electric_panel", candidate, result) do
    field = path_float(result, ["electrostatic", "max_electric_field"])

    %{
      "max_electric_field_v_m" => field,
      "max_temperature_c" => path_float(result, ["heat", "max_temperature"]),
      "max_thermal_stress_pa" => path_float(result, ["thermal", "max_stress"]),
      "breakdown_safety_factor" =>
        if(is_number(field) and field > 0, do: candidate["breakdown_field_v_m"] / field),
      "interface_risk_score" => candidate["interface_risk_score"],
      "areal_mass_kg_m2" => candidate["areal_mass_kg_m2"]
    }
  end

  defp study_metrics("material_structural_panel_screening", candidate, result) do
    stress = float(result["max_stress"])

    %{
      "max_stress_pa" => stress,
      "max_displacement_m" => float(result["max_displacement"]),
      "areal_mass_kg_m2" => candidate["density_kg_m3"] * thickness(result),
      "specific_stiffness_m2_s2" => candidate["youngs_modulus_pa"] / candidate["density_kg_m3"],
      "yield_safety_factor" =>
        if(is_number(stress) and stress > 0, do: candidate["yield_strength_pa"] / stress)
    }
  end

  defp optimization_terms(specs, metrics, optimization) do
    weights =
      optimization
      |> Map.get("weights", [])
      |> Enum.reduce(%{}, fn
        %{"metric_id" => id, "weight" => weight}, acc when is_binary(id) ->
          Map.put(acc, id, float(weight) || 0.0)

        _weight, acc ->
          acc
      end)

    Enum.map(specs, fn spec ->
      value = metrics[spec["id"]]

      %{
        "metric_id" => spec["id"],
        "direction" => spec["direction"],
        "weight" => Map.get(weights, spec["id"], spec["default_weight"]),
        "value" => value,
        "score" => metric_score(value, spec["direction"])
      }
    end)
  end

  defp metric_score(nil, _direction), do: 0.0
  defp metric_score(_value, "observe"), do: 0.0
  defp metric_score(value, "maximize"), do: value / (1.0 + abs(value))
  defp metric_score(value, _direction), do: 1.0 / (1.0 + abs(value))

  defp default_optimization(specs) do
    %{
      "id" => "default",
      "weights" =>
        specs
        |> Enum.reject(&(&1["direction"] == "observe"))
        |> Enum.map(&%{"metric_id" => &1["id"], "weight" => &1["default_weight"]}),
      "constraints" => []
    }
  end

  defp thickness(result), do: float(result["thickness_m"]) || float(result["thickness"]) || 0.002
  defp float(value) when is_number(value), do: value * 1.0

  defp float(value) when is_binary(value) do
    case Float.parse(value) do
      {parsed, _rest} -> parsed
      :error -> nil
    end
  end

  defp float(_value), do: nil

  defp path_float(payload, path) do
    path
    |> Enum.reduce(payload, fn
      key, value when is_map(value) -> Map.get(value, key)
      _key, _value -> nil
    end)
    |> float()
  end
end
