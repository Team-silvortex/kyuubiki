defmodule KyuubikiWeb.WorkflowSweepEvaluationTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowParameterSweepRuntime, as: Runtime

  for fields <- [
        [],
        nil,
        "mass",
        ["mass", 2],
        ["mass", "mass"],
        [" "],
        ["case_id"],
        ["parameters"],
        ["metadata"]
      ] do
    @fields fields
    test "summary rejects invalid selected fields #{inspect(fields)}" do
      assert {:error, _} = summary([%{"mass" => 1, "case_id" => "fake"}], %{"fields" => @fields})
    end
  end

  test "summary cannot overwrite provenance through automatic field selection" do
    for key <-
          ~w(id case_id caseId parameters metadata result_status result_error objective_score objective_feasible objective_breakdown) do
      assert {:error, _} = summary([%{"mass" => 1, key => "fake"}])
    end
  end

  test "missing null and mixed numeric fields cannot silently shrink evidence" do
    for bad <- [%{}, %{"mass" => nil}, %{"mass" => "2"}] do
      assert {:error, _} = summary([%{"mass" => 1}, bad])
      assert {:ok, diagnostic} = summary([%{"mass" => 1}, bad], %{"fail_on_missing" => false})
      refute diagnostic["summary_complete"]
      assert diagnostic["missing_field_count"] == 1
      assert [%{"case_id" => "case_1", "field" => "mass"}] = diagnostic["missing_fields"]
      assert {:error, _} = score(diagnostic)
    end
  end

  test "complete flags cannot hide missing or unmatched evidence" do
    for extra <- [
          %{"join_complete" => false},
          %{"join_complete" => "true"},
          %{"missing_summary_count" => 1},
          %{"unmatched_result_ids" => ["foreign"]},
          %{"rejected_result_count" => 1},
          %{"joined_summary_count" => 2}
        ] do
      assert {:error, _} =
               Runtime.summarize_parameter_sweep(
                 Map.merge(%{"cases" => [%{"summary" => %{"mass" => 1}}]}, extra),
                 %{}
               )
    end

    for extra <- [
          %{"summary_complete" => false},
          %{"summary_complete" => nil},
          %{"missing_field_count" => 1},
          %{"missing_fields" => ["mass"]},
          %{"missing_fields" => "none"},
          %{"row_count" => 2}
        ] do
      assert {:error, _} = score(Map.merge(%{"rows" => [%{"mass" => 1}]}, extra))
    end
  end

  test "summary rejects failed statuses malformed flags and corrupt selected aliases" do
    for extra <- [
          %{"result_status" => "failed"},
          %{"result_status" => "pending"},
          %{"result_status" => nil},
          %{"result_error" => "bad"},
          %{"summary" => nil}
        ] do
      assert {:error, _} =
               Runtime.summarize_parameter_sweep(
                 %{
                   "cases" => [
                     Map.merge(%{"summary" => %{"mass" => 1}, "result" => %{"mass" => 0}}, extra)
                   ]
                 },
                 %{}
               )
    end

    for config <- [
          %{"include_metadata" => nil},
          %{"include_parameters" => "false"},
          %{"fail_on_missing" => 0},
          %{"fields" => nil}
        ] do
      assert {:error, _} = summary([%{"mass" => 1}], config)
    end
  end

  test "finite signed statistics and text survive but overflowing totals reject" do
    assert {:ok, table} =
             summary([
               %{"mass" => -2, "note" => "cold"},
               %{"mass" => 0, "note" => "neutral"},
               %{"mass" => 2, "note" => "warm"}
             ])

    assert table["summary_complete"]

    assert table["numeric_columns"]["mass"] == %{
             "count" => 3,
             "sum" => 0.0,
             "mean" => 0.0,
             "min" => -2.0,
             "max" => 2.0
           }

    assert Enum.at(table["rows"], 1)["note"] == "neutral"
    assert {:error, reason} = summary([%{"mass" => 1.0e308}, %{"mass" => 1.0e308}])
    assert reason =~ "mass"
  end

  for objective <- [
        %{"weight" => -1},
        %{"weight" => nil},
        %{"min_allowed" => "0"},
        %{"max_allowed" => nil},
        %{"min_allowed" => 2, "max_allowed" => 1},
        %{"target" => "bad"},
        %{"goal" => "target"},
        %{"field" => " "}
      ] do
    @objective objective
    test "objective configuration cannot weaken constraints #{inspect(objective)}" do
      assert {:error, _} =
               score(%{"rows" => [%{"mass" => 1}]}, [Map.merge(%{"field" => "mass"}, @objective)])
    end
  end

  test "duplicate objectives and duplicate or conflicting row identities reject" do
    assert {:error, _} =
             score(%{"rows" => [%{"mass" => 1}]}, [%{"field" => "mass"}, %{"field" => "mass"}])

    for rows <- [
          [%{"case_id" => "a", "mass" => 1}, %{"case_id" => "a", "mass" => 2}],
          [%{"case_id" => "a", "id" => "b", "mass" => 1}],
          [%{"case_id" => nil, "mass" => 1}]
        ] do
      assert {:error, _} = score(%{"rows" => rows})
    end
  end

  test "feasible candidates outrank arbitrarily large infeasible scores" do
    rows = [%{"case_id" => "invalid", "mass" => 1.0e15}, %{"case_id" => "valid", "mass" => 1}]

    assert {:ok, ranked} =
             score(%{"rows" => rows}, [%{"field" => "mass", "goal" => "max", "max_allowed" => 2}])

    assert ranked["best"]["case_id"] == "valid"
    assert ranked["feasible_count"] == 1
    assert ranked["selection_ready"]
  end

  test "all infeasible rows remain diagnostics without an actionable best candidate" do
    assert {:ok, ranked} =
             score(%{"rows" => [%{"mass" => 4}]}, [%{"field" => "mass", "max_allowed" => 2}])

    assert ranked["best"] == nil
    assert ranked["feasible_count"] == 0
    assert ranked["scored_count"] == 1
    refute ranked["selection_ready"]
  end

  test "score overflow is contextual and never exports an invalid number" do
    for {rows, objectives} <- [
          {[%{"mass" => 1.0e308}], [%{"field" => "mass", "weight" => 2}]},
          {[%{"mass" => -1.0e308}],
           [%{"field" => "mass", "goal" => "target", "target" => 1.0e308}]},
          {[%{"mass" => 1.0e308, "heat" => 1.0e308}],
           [%{"field" => "mass"}, %{"field" => "heat"}]}
        ] do
      assert {:error, reason} = score(%{"rows" => rows}, objectives)
      assert is_binary(reason)
    end
  end

  test "ties are identity-stable and zero weight does not disable hard limits" do
    rows = [
      %{"case_id" => "z", "mass" => 2},
      %{"case_id" => "a", "mass" => 1},
      %{"case_id" => "bad", "mass" => 3}
    ]

    assert {:ok, ranked} =
             score(%{"rows" => rows}, [%{"field" => "mass", "weight" => 0, "max_allowed" => 2}])

    assert Enum.map(ranked["scored_rows"], & &1["case_id"]) == ["a", "z", "bad"]
    assert ranked["best"]["case_id"] == "a"
  end

  defp summary(summaries, config \\ %{}),
    do:
      Runtime.summarize_parameter_sweep(
        %{"cases" => Enum.map(summaries, &%{"summary" => &1})},
        config
      )

  defp score(payload, objectives \\ [%{"field" => "mass"}]),
    do: Runtime.score_parameter_sweep(payload, %{"objectives" => objectives})
end
