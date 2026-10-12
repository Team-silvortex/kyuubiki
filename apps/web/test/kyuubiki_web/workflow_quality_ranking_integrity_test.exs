defmodule KyuubikiWeb.WorkflowQualityRankingIntegrityTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-ranking-integrity.json",
              __DIR__
            )
  for fixture <- @fixtures |> File.read!() |> Jason.decode!() do
    @fixture fixture
    test "shared ranking integrity: #{fixture["id"]}" do
      result =
        Runtime.run_transform_operator(
          "transform.prepare_quality_next_round_request",
          @fixture["payload"],
          @fixture["config"]
        )

      if @fixture["error"] do
        assert {:error, _} = result
      else
        assert {:ok, result} = result
        for {key, value} <- @fixture["expected"], do: assert(result[key] == value)
      end
    end
  end

  test "generated rankings survive JSON exchange and reject noncanonical permutations" do
    permutations = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]]

    for {scores, readiness, expected} <- [
          {[1.125, 1.125, 1.125], [true, true, true], ~w(a b z)},
          {[2.25, 1.125, 0], [true, true, true], ~w(b a z)},
          {[0, 2.25, 1.125], [false, true, true], ~w(b a z)}
        ],
        permutation <- permutations do
      candidates =
        Enum.with_index(~w(z a b))
        |> Enum.map(fn {id, index} ->
          %{
            "id" => id,
            "qualities" => %{
              "thermal" => %{
                "thermal_quality_score" => Enum.at(scores, index),
                "thermal_quality_ready" => Enum.at(readiness, index)
              }
            }
          }
        end)

      assert {:ok, ranked} =
               Runtime.run_transform_operator(
                 "transform.rank_quality_candidates",
                 %{"candidates" => Enum.map(permutation, &Enum.at(candidates, &1))},
                 %{"not_ready_penalty" => 0}
               )

      assert Enum.map(ranked["ranking"], & &1["candidate_id"]) == expected
      exchanged = ranked |> Jason.encode!() |> Jason.decode!()

      assert {:ok, _} =
               Runtime.run_transform_operator(
                 "transform.prepare_quality_next_round_request",
                 exchanged,
                 %{}
               )

      rows =
        Enum.map(permutation, &Enum.at(exchanged["ranking"], &1))
        |> Enum.with_index(1)
        |> Enum.map(fn {row, index} -> Map.put(row, "rank", index) end)

      shuffled =
        Map.merge(exchanged, %{
          "ranking" => rows,
          "best_candidate_id" => hd(rows)["candidate_id"],
          "best_candidate_score" => hd(rows)["score"],
          "best_candidate_ready" => hd(rows)["ready"]
        })

      result =
        Runtime.run_transform_operator(
          "transform.prepare_quality_next_round_request",
          shuffled,
          %{}
        )

      if permutation == [0, 1, 2],
        do: assert(match?({:ok, _}, result)),
        else: assert(match?({:error, _}, result))
    end
  end
end
