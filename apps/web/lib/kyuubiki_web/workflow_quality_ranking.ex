defmodule KyuubikiWeb.WorkflowQualityRanking do
  @moduledoc false

  alias KyuubikiWeb.WorkflowQualityComposition, as: Quality
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def run(payload, config) do
    Check.run("transform.rank_quality_candidates", fn ->
      payload = Check.object(payload, "payload")
      if Map.has_key?(payload, "candidates"), do: Check.converged(payload, "payload")
      config = Quality.checked_config(config)

      objective_config =
        Quality.checked_config(Map.get(config, "objective", config), "config.objective")

      entries = entries(payload)

      {ranking, rejected} =
        Enum.reduce(entries, {[], []}, fn {id, candidate}, {ranking, rejected} ->
          case Quality.run(candidate, objective_config) do
            {:ok, objective} -> {[rank(id, candidate, objective) | ranking], rejected}
            {:error, reason} -> {ranking, [%{"candidate_id" => id, "error" => reason} | rejected]}
          end
        end)

      rejected = Enum.reverse(rejected)

      if ranking == [],
        do:
          Check.fail(
            "no valid candidates; rejected #{length(rejected)}: #{inspect(List.first(rejected))}"
          )

      ranking =
        ranking
        |> Enum.sort_by(&order_key/1)
        |> Enum.with_index(1)
        |> Enum.map(fn {entry, index} -> Map.put(entry, "rank", index) end)

      [best | _] = ranking

      %{
        "quality_candidate_ranking_contract" => "kyuubiki.quality_candidate_ranking/v1",
        "candidate_count" => length(ranking),
        "input_candidate_count" => length(entries),
        "rejected_candidate_count" => length(rejected),
        "ranking_complete" => rejected == [],
        "rejected_candidates" => rejected,
        "ready_candidate_count" => Enum.count(ranking, & &1["ready"]),
        "best_candidate_id" => best["candidate_id"],
        "best_candidate_ready" => best["ready"],
        "best_candidate_score" => best["score"],
        "ranking" => ranking,
        "ranking_summary" =>
          "Ranked #{length(ranking)}/#{length(entries)} candidates; rejected #{length(rejected)}. Best quality candidate #{best["candidate_id"]}: score=#{best["score"]}, ready=#{best["ready"]}."
      }
    end)
  end

  def order_key(entry),
    do: {if(entry["ready"], do: 0, else: 1), entry["score"], entry["candidate_id"]}

  defp entries(payload) do
    source = Map.get(payload, "candidates", payload)

    entries =
      cond do
        is_map(source) ->
          source |> Check.object("payload.candidates") |> Enum.sort_by(&elem(&1, 0))

        is_list(source) ->
          source
          |> Enum.with_index(1)
          |> Enum.map(fn {value, index} ->
            id =
              if is_map(value),
                do: Map.get(value, "id", "candidate_#{index}"),
                else: "candidate_#{index}"

            {id, value}
          end)

        true ->
          Check.fail("payload.candidates must be an object or array")
      end

    if entries == [], do: Check.fail("payload.candidates must not be empty")

    ids =
      Enum.map(entries, fn {id, _} ->
        if !is_binary(id) || String.trim(id) == "",
          do: Check.fail("payload.candidates contains an invalid candidate id")

        id
      end)

    if length(Enum.uniq(ids)) != length(ids),
      do: Check.fail("payload.candidates contains duplicate candidate ids")

    entries
  end

  defp rank(id, candidate, objective) do
    %{
      "candidate_id" => id,
      "candidate_label" => if(is_binary(candidate["label"]), do: candidate["label"], else: id),
      "metadata" => candidate["metadata"],
      "score" => objective["composite_quality_score"],
      "grade" => objective["composite_quality_grade"],
      "ready" => objective["composite_quality_ready"],
      "term_count" => objective["composite_quality_term_count"],
      "missing_metric_count" => objective["composite_quality_missing_metric_count"],
      "blocked_term_count" => objective["composite_quality_blocked_term_count"],
      "dominant_term" => objective["composite_quality_dominant_term"],
      "blocking_terms" => objective["composite_quality_blocking_terms"],
      "objective" => objective
    }
  end
end
