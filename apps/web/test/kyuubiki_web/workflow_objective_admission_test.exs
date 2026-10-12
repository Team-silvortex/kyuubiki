defmodule KyuubikiWeb.WorkflowObjectiveAdmissionTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowQualityObjectiveRuntime, as: Runtime

  for {name, payload} <- [
        {"nonobject root", []},
        {"nonobject qualities", %{"qualities" => []}},
        {"empty qualities", %{"qualities" => %{}}},
        {"nonobject summary", %{"qualities" => %{"thermal" => nil}}},
        {"missing score", %{"qualities" => %{"thermal" => %{}}}},
        {"parent nonconvergence", %{"converged" => false, "qualities" => %{}}}
      ] do
    test "rejects #{name} without raising" do
      assert {:error, reason} =
               Runtime.compose_quality_objective(unquote(Macro.escape(payload)), %{})

      assert is_binary(reason)
    end
  end

  for {field, value} <- [
        {"thermal_quality_score", -1},
        {"thermal_quality_score", "0"},
        {"thermal_quality_score", nil},
        {"thermal_quality_ready", "true"},
        {"thermal_quality_missing_metric_count", 0.5},
        {"thermal_quality_watch_count", -1},
        {"thermal_quality_blocking_terms", %{}},
        {"converged", false},
        {"stability_result", %{"converged" => false}},
        {"other_quality_score", 0}
      ] do
    test "rejects malformed summary #{field}=#{inspect(value)}" do
      summary = Map.put(quality(), unquote(field), unquote(Macro.escape(value)))
      assert {:error, reason} = Runtime.compose_quality_objective(%{"thermal" => summary}, %{})
      assert is_binary(reason)
    end
  end

  for config <- [
        [],
        %{"weights" => []},
        %{"weights" => %{"unused" => -1}},
        %{"weights" => %{"thermal" => nil}},
        %{"missing_metric_penalty" => -1},
        %{"not_ready_penalty" => "25"},
        %{"max_ready_score" => nil}
      ] do
    test "rejects malformed objective config #{inspect(config)}" do
      assert {:error, reason} =
               Runtime.compose_quality_objective(candidate(), unquote(Macro.escape(config)))

      assert reason =~ "config"
    end
  end

  test "metadata is not a quality source and defaults allow nil config" do
    payload = %{"id" => "c1", "metadata" => %{"recipe" => "a"}, "thermal" => quality()}
    assert {:ok, objective} = Runtime.compose_quality_objective(payload, nil)
    assert objective["composite_quality_term_count"] == 1
    assert objective["composite_quality_score"] == 1.0
  end

  test "blocking evidence overrides declared readiness and zero penalties" do
    summary = Map.put(quality(), "thermal_quality_blocking_terms", [%{"field" => "flux"}])

    assert {:ok, objective} =
             Runtime.compose_quality_objective(%{"thermal" => summary}, %{
               "weights" => %{"thermal" => 0},
               "not_ready_penalty" => 0
             })

    refute objective["composite_quality_ready"]
    assert objective["composite_quality_score"] == 0

    assert [%{"source" => "thermal", "source_blocking_terms" => [%{"field" => "flux"}]}] =
             objective["composite_quality_blocking_terms"]
  end

  test "overflow is a contract error, including aggregate counters" do
    huge = Map.put(quality(), "thermal_quality_score", 1.0e308)
    assert {:error, _} = Runtime.compose_quality_objective(%{"a" => huge, "b" => huge}, %{})

    assert {:error, _} =
             Runtime.compose_quality_objective(%{"a" => huge}, %{"weights" => %{"a" => 2}})

    count = Map.put(quality(), "thermal_quality_watch_count", 18_446_744_073_709_551_615)
    assert {:error, _} = Runtime.compose_quality_objective(%{"a" => count, "b" => count}, %{})
  end

  test "validation evidence participates without a domain quality score" do
    validation = %{
      "validation_contract" => "kyuubiki.summary_tolerance_validation/v1",
      "validation_passed" => true,
      "validation_checked_field_count" => 2,
      "validation_failed_field_count" => 0,
      "validation_missing_field_count" => 0,
      "validation_max_relative_error" => 0.001
    }

    assert {:ok, objective} =
             Runtime.compose_quality_objective(%{"validation" => validation}, %{})

    assert objective["composite_quality_ready"]
    assert objective["composite_quality_score"] == 1.0

    for key <- [
          "validation_checked_field_count",
          "validation_failed_field_count",
          "validation_missing_field_count"
        ] do
      assert {:ok, blocked} =
               Runtime.compose_quality_objective(
                 %{"validation" => Map.delete(validation, key)},
                 %{}
               )

      refute blocked["composite_quality_ready"]
    end
  end

  test "partial rankings account for every original candidate without renumbering" do
    assert {:ok, ranked} =
             Runtime.rank_quality_candidates(%{"candidates" => [nil, candidate()]}, %{})

    assert ranked["input_candidate_count"] == 2
    assert ranked["candidate_count"] == 1
    assert ranked["rejected_candidate_count"] == 1
    refute ranked["ranking_complete"]
    assert ranked["best_candidate_id"] == "candidate_2"
    assert [%{"candidate_id" => "candidate_1", "error" => error}] = ranked["rejected_candidates"]
    assert is_binary(error)
  end

  test "all rejected returns an actionable controlled error" do
    assert {:error, reason} =
             Runtime.rank_quality_candidates(%{"candidates" => %{"broken" => %{}}}, %{})

    assert reason =~ "broken"
  end

  for candidates <- [
        nil,
        [],
        %{},
        [%{"id" => "same"}, %{"id" => "same"}],
        [%{"id" => " "}],
        [%{"id" => 3}]
      ] do
    test "rejects invalid candidate collection #{inspect(candidates)}" do
      assert {:error, reason} =
               Runtime.rank_quality_candidates(
                 %{"candidates" => unquote(Macro.escape(candidates))},
                 %{}
               )

      assert is_binary(reason)
    end
  end

  test "nested objective configuration is checked even when no candidates survive" do
    assert {:error, reason} =
             Runtime.rank_quality_candidates(%{"candidates" => [candidate()]}, %{
               "objective" => []
             })

    assert reason =~ "config.objective"
  end

  test "ties are deterministic and candidate metadata survives ranking" do
    payload = %{
      "candidates" => %{
        "z" => candidate(),
        "a" => Map.put(candidate(), "metadata", %{"recipe" => 2})
      }
    }

    assert {:ok, ranked} = Runtime.rank_quality_candidates(payload, %{})
    assert Enum.map(ranked["ranking"], & &1["candidate_id"]) == ["a", "z"]
    assert hd(ranked["ranking"])["metadata"] == %{"recipe" => 2}
    assert ranked["ranking_complete"]
  end

  defp quality do
    %{
      "thermal_quality_score" => 1.0,
      "thermal_quality_ready" => true,
      "thermal_quality_missing_metric_count" => 0
    }
  end

  defp candidate, do: %{"qualities" => %{"thermal" => quality()}}
end
