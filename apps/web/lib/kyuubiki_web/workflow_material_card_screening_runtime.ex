defmodule KyuubikiWeb.WorkflowMaterialCardScreeningRuntime do
  @moduledoc false

  def explain_material_card_screening(payload, config) when is_map(payload) and is_map(config) do
    with rankings when rankings != [] <- ranking_entries(payload) do
      best = List.first(rankings, %{})

      {:ok,
       %{
         "material_card_screening_report_schema" => "kyuubiki.material-card-screening-report/v1",
         "material_screening_candidate_count" =>
           Map.get(payload, "material_score_candidate_count", length(rankings)),
         "material_screening_feasible_count" =>
           Map.get(payload, "material_score_feasible_count", feasible_count(rankings)),
         "material_screening_best_candidate_id" => Map.get(best, "candidate_id", ""),
         "material_screening_best_score" => numeric_field(best, "final_score"),
         "material_screening_parameter_fields" => parameter_fields(rankings),
         "material_screening_rankings" => build_rankings(rankings),
         "material_screening_policy" => screening_policy(payload, config)
       }}
    else
      [] -> {:error, :missing_material_score_rankings}
    end
  end

  def explain_material_card_screening(_payload, _config),
    do: {:error, :invalid_material_screening_report_payload}

  defp ranking_entries(%{"material_score_rankings" => rankings}) when is_list(rankings),
    do: Enum.filter(rankings, &is_map/1)

  defp ranking_entries(%{"rankings" => rankings}) when is_list(rankings),
    do: Enum.filter(rankings, &is_map/1)

  defp ranking_entries(_payload), do: []

  defp feasible_count(rankings),
    do: Enum.count(rankings, &(Map.get(&1, "feasible", true) == true))

  defp parameter_fields(rankings) do
    rankings
    |> Enum.flat_map(&Map.get(&1, "criteria_breakdown", []))
    |> Enum.map(&Map.get(&1, "field", ""))
    |> Enum.filter(&String.starts_with?(&1, "material_param_"))
    |> Enum.uniq()
    |> Enum.sort()
  end

  defp build_rankings(rankings) do
    rankings
    |> Enum.with_index(1)
    |> Enum.map(fn {ranking, rank} ->
      %{
        "rank" => rank,
        "candidate_id" => Map.get(ranking, "candidate_id", ""),
        "feasible" => Map.get(ranking, "feasible", true),
        "final_score" => numeric_field(ranking, "final_score"),
        "weighted_score" => numeric_field(ranking, "weighted_score"),
        "metrics" => Map.get(ranking, "metrics", %{}),
        "deciding_fields" => deciding_fields(ranking),
        "criteria_breakdown" => Map.get(ranking, "criteria_breakdown", []),
        "explanation" => explanation_text(ranking, rank)
      }
    end)
  end

  defp deciding_fields(ranking) do
    ranking
    |> Map.get("criteria_breakdown", [])
    |> Enum.filter(&is_map/1)
    |> Enum.sort_by(&{-numeric_field(&1, "weighted_score"), Map.get(&1, "field", "")})
    |> Enum.take(3)
    |> Enum.map(fn criterion ->
      %{
        "field" => Map.get(criterion, "field", ""),
        "goal" => Map.get(criterion, "goal", ""),
        "actual" => numeric_field(criterion, "actual"),
        "weighted_score" => numeric_field(criterion, "weighted_score")
      }
    end)
  end

  defp explanation_text(ranking, rank) do
    candidate_id = Map.get(ranking, "candidate_id", "")
    score = ranking |> numeric_field("final_score") |> Float.round(4)

    top_fields =
      ranking
      |> deciding_fields()
      |> Enum.map(& &1["field"])
      |> Enum.reject(&(&1 == ""))
      |> Enum.join(", ")

    feasible_note =
      if Map.get(ranking, "feasible", true), do: "feasible", else: "penalized as infeasible"

    "rank #{rank}: #{candidate_id} scored #{score}, #{feasible_note}; strongest signals: #{top_fields}"
  end

  defp screening_policy(payload, config) do
    %{
      "label" => Map.get(config, "label", "material-card-screening"),
      "source" => "material_score_rankings",
      "criteria" => Map.get(payload, "material_score_criteria", []),
      "explain_top_k" => config |> numeric_config("explain_top_k", 3) |> round() |> max(1)
    }
  end

  defp numeric_config(config, key, default) do
    case Map.get(config, key, default) do
      value when is_number(value) -> value * 1.0
      _ -> default * 1.0
    end
  end

  defp numeric_field(map, key) do
    case Map.get(map, key, 0.0) do
      value when is_number(value) -> value * 1.0
      _ -> 0.0
    end
  end
end
