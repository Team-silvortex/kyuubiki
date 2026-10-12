defmodule KyuubikiWeb.WorkflowSweepScore do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowSweepResultChecks, as: Result

  def run(payload, config) do
    Check.run("transform.score_parameter_sweep", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      Result.complete(payload)
      rows = Sweep.values(payload["rows"], "payload.rows")
      ids = Sweep.case_ids(rows)
      Sweep.require_count(payload, "row_count", length(rows))

      objectives =
        Sweep.values(config["objectives"], "config.objectives")
        |> Enum.with_index()
        |> Enum.map(&objective/1)

      fields = Enum.map(objectives, & &1.field)

      if length(Enum.uniq(fields)) != length(fields),
        do: Check.fail("config.objectives contains duplicate fields")

      scored = Enum.zip(rows, ids) |> Enum.map(fn {row, id} -> score_row(row, id, objectives) end)
      # A fixed numeric penalty cannot enforce a hard constraint at arbitrary metric scales.
      scored =
        Enum.sort_by(scored, &{!&1["objective_feasible"], -&1["objective_score"], &1["case_id"]})

      feasible = Enum.count(scored, & &1["objective_feasible"])

      %{
        "best" => if(feasible > 0, do: hd(scored)),
        "scored_rows" => scored,
        "scored_count" => length(scored),
        "feasible_count" => feasible,
        "selection_ready" => feasible > 0
      }
    end)
  end

  defp objective({raw, index}) do
    path = "config.objectives[#{index}]"
    raw = Check.object(raw, path)
    field = Sweep.text(raw, "field", nil, path) |> Result.metric_field(path)
    goal = Check.choice(raw, "goal", "min", ~w(min max target), path)
    weight = Check.number(Map.get(raw, "weight", 1.0), "#{path}.weight")
    if weight < 0, do: Check.fail("#{path}.weight must be nonnegative")
    target = Check.optional_number(raw, "target", path)
    minimum = Check.optional_number(raw, "min_allowed", path)
    maximum = Check.optional_number(raw, "max_allowed", path)
    if goal == "target" && is_nil(target), do: Check.fail("#{path}.target is required")

    if minimum != nil && maximum != nil && minimum > maximum,
      do: Check.fail("#{path}.min_allowed exceeds max_allowed")

    %{
      field: field,
      goal: goal,
      weight: weight,
      target: target,
      minimum: minimum,
      maximum: maximum
    }
  end

  defp score_row(row, id, objectives) do
    Result.success(row, "result_status", "result_error", "row #{id}")

    {breakdown, {total, feasible}} =
      Enum.map_reduce(objectives, {0.0, true}, fn objective, {total, feasible} ->
        path = "row #{id}.#{objective.field}"
        value = Check.number(row[objective.field], path)

        score =
          Check.calculate(path, fn ->
            case objective.goal do
              "min" -> -value * objective.weight
              "max" -> value * objective.weight
              "target" -> -abs(value - objective.target) * objective.weight
            end
          end)

        allowed =
          (is_nil(objective.minimum) || value >= objective.minimum) &&
            (is_nil(objective.maximum) || value <= objective.maximum)

        term = %{
          "field" => objective.field,
          "goal" => objective.goal,
          "weight" => objective.weight,
          "value" => value,
          "score" => score,
          "feasible" => allowed
        }

        term =
          Enum.reduce(
            [
              {"target", objective.target},
              {"min_allowed", objective.minimum},
              {"max_allowed", objective.maximum}
            ],
            term,
            fn
              {_key, nil}, acc -> acc
              {key, value}, acc -> Map.put(acc, key, value)
            end
          )

        {term,
         {Check.calculate("row #{id} objective total", fn -> total + score end),
          feasible && allowed}}
      end)

    score =
      Check.calculate("row #{id} objective penalty", fn ->
        if feasible, do: total, else: total - 1.0e12
      end)

    row
    |> Map.put("case_id", id)
    |> Map.put("objective_score", score)
    |> Map.put("objective_feasible", feasible)
    |> Map.put("objective_breakdown", breakdown)
  end
end
