defmodule KyuubikiWeb.WorkflowMaterialCardCandidateRuntime do
  @moduledoc false

  def build_material_card_candidate_summaries(payload, config)
      when is_map(payload) and is_map(config) do
    with reports when reports != [] <- Map.get(payload, "material_card_batch_reports", []),
         {:ok, candidates} <- build_candidates(reports, config) do
      {:ok,
       %{
         "material_card_candidate_schema" => "kyuubiki.material-card-candidates/v1",
         "material_card_candidate_count" => map_size(candidates),
         "material_card_candidate_usable_count" =>
           Enum.count(candidates, fn {_id, summary} -> summary["material_status"] == "pass" end),
         "material_card_candidate_parameter_fields" => parameter_fields(candidates),
         "candidates" => candidates
       }}
    else
      [] -> {:error, :missing_material_card_batch_reports}
      {:error, _reason} = error -> error
    end
  end

  def build_material_card_candidate_summaries(_payload, _config),
    do: {:error, :invalid_material_card_candidate_payload}

  defp build_candidates(reports, config) do
    reports
    |> Enum.reduce_while({:ok, %{}}, fn report, {:ok, acc} ->
      candidate_id = Map.get(report, "candidate_id") || Map.get(report, "material_card_id")

      if is_binary(candidate_id) and candidate_id != "" do
        {:cont, {:ok, Map.put(acc, candidate_id, candidate_summary(report, config))}}
      else
        {:halt, {:error, :invalid_material_card_candidate_id}}
      end
    end)
  end

  defp candidate_summary(report, config) do
    trust_level =
      get_in(report, ["material_card_reliability_envelope", "trust_level"]) || "unknown"

    status = Map.get(report, "material_card_preflight_status", "fail")

    base = %{
      "material_card_id" => Map.get(report, "material_card_id", ""),
      "material_card_display_name" => Map.get(report, "material_card_display_name", ""),
      "material_status" => if(status in ["pass", "warn"], do: "pass", else: "fail"),
      "material_safety_factor" => trust_score(trust_level),
      "material_failure_index" => failure_index(status),
      "material_preflight_status" => status,
      "material_issue_count" => Map.get(report, "material_card_issue_count", 0),
      "material_error_count" => Map.get(report, "material_card_error_count", 0),
      "material_warning_count" => Map.get(report, "material_card_warning_count", 0),
      "material_trust_level" => trust_level,
      "material_trust_score" => trust_score(trust_level),
      "material_confidence_level" => Map.get(report, "material_card_confidence_level", "unknown"),
      "material_parameter_count" => Map.get(report, "material_card_parameter_count", 0),
      "material_cost_score" => Map.get(config, "default_cost_score", 1.0),
      "material_mass_score" => Map.get(config, "default_mass_score", 1.0)
    }

    Map.merge(base, numeric_parameter_fields(report))
  end

  defp trust_score("review_ready"), do: 1.0
  defp trust_score("screening_only"), do: 0.7
  defp trust_score("blocked"), do: 0.0
  defp trust_score(_trust_level), do: 0.4

  defp failure_index("fail"), do: 2.0
  defp failure_index("warn"), do: 0.9
  defp failure_index(_status), do: 0.5

  defp numeric_parameter_fields(%{"material_card_parameters" => parameters})
       when is_map(parameters) do
    parameters
    |> Enum.flat_map(fn {name, parameter} ->
      case Map.get(parameter, "value") do
        value when is_number(value) ->
          [
            {"material_param_#{name}", value * 1.0},
            {"material_param_#{name}_unit", Map.get(parameter, "unit", "")}
          ]

        _ ->
          []
      end
    end)
    |> Map.new()
  end

  defp numeric_parameter_fields(_report), do: %{}

  defp parameter_fields(candidates) do
    candidates
    |> Enum.flat_map(fn {_id, summary} ->
      summary
      |> Map.keys()
      |> Enum.filter(&String.starts_with?(&1, "material_param_"))
      |> Enum.reject(&String.ends_with?(&1, "_unit"))
    end)
    |> Enum.uniq()
    |> Enum.sort()
  end
end
