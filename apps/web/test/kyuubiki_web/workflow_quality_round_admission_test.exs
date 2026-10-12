defmodule KyuubikiWeb.WorkflowQualityRoundAdmissionTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowQualityObjectiveRuntime, as: Runtime

  test "unready low-score candidates never stop research" do
    assert {:ok, next} = Runtime.prepare_quality_next_round_request(ranking(false), %{})
    assert next["action"] == "continue"

    assert {:ok, next} =
             Runtime.prepare_quality_next_round_request(ranking(false), %{"require_ready" => true})

    assert next["action"] == "replan"
  end

  test "incomplete ranking replans even when its surviving candidate meets target" do
    partial =
      Map.merge(ranking(true), %{
        "ranking_complete" => false,
        "rejected_candidate_count" => 1,
        "input_candidate_count" => 2,
        "rejected_candidates" => [%{"candidate_id" => "broken", "error" => "missing score"}]
      })

    assert {:ok, next} = Runtime.prepare_quality_next_round_request(partial, %{})
    assert next["action"] == "replan"
    refute next["source_ranking_complete"]
    assert next["selected_iteration_hint"]["action"] == "repair_rejected_candidates"
    assert length(next["source_rejected_candidates"]) == 1
  end

  test "blocking terms override readiness even without require_ready" do
    payload =
      put_in(ranking(true), ["ranking", Access.at(0), "blocking_terms"], [%{"field" => "flux"}])

    assert {:ok, next} = Runtime.prepare_quality_next_round_request(payload, %{})
    assert next["action"] == "replan"
    refute next["selected_candidate_ready"]
  end

  for {field, value} <- [
        {"candidate_count", 2},
        {"input_candidate_count", 2},
        {"rejected_candidate_count", 1},
        {"rejected_candidates", %{}},
        {"ranking_complete", "true"},
        {"best_candidate_score", 2},
        {"best_candidate_id", "someone_else"},
        {"best_candidate_ready", false}
      ] do
    test "rejects contradictory ranking #{field}" do
      assert {:error, reason} =
               Runtime.prepare_quality_next_round_request(
                 Map.put(ranking(true), unquote(field), unquote(Macro.escape(value))),
                 %{}
               )

      assert reason =~ unquote(field)
    end
  end

  for {field, value} <- [
        {"score", nil},
        {"score", -1},
        {"ready", "true"},
        {"candidate_id", ""},
        {"blocking_terms", %{}}
      ] do
    test "rejects malformed selected #{field}=#{inspect(value)}" do
      payload =
        put_in(
          ranking(true),
          ["ranking", Access.at(0), unquote(field)],
          unquote(Macro.escape(value))
        )

      assert {:error, reason} = Runtime.prepare_quality_next_round_request(payload, %{})
      assert is_binary(reason)
    end
  end

  for config <- [
        %{"target_score" => -1},
        %{"require_ready" => "true"},
        %{"max_candidates" => nil}
      ] do
    test "rejects malformed next round config #{inspect(config)}" do
      assert {:error, reason} =
               Runtime.prepare_quality_next_round_request(
                 ranking(true),
                 unquote(Macro.escape(config))
               )

      assert reason =~ "config"
    end
  end

  test "complete ready ranking can stop and preserves seed metadata" do
    payload = put_in(ranking(true), ["ranking", Access.at(0), "metadata"], %{"recipe" => 2})
    assert {:ok, next} = Runtime.prepare_quality_next_round_request(payload, %{})
    assert next["action"] == "stop"
    assert next["source_ranking_complete"]
    assert next["request_payload"]["seed_metadata"] == %{"recipe" => 2}
  end

  defp ranking(ready) do
    %{"ranking" => [%{"candidate_id" => "a", "score" => 0.0, "ready" => ready}]}
  end

  test "ranked candidate identity remains stable across next round admission" do
    candidate = %{
      "qualities" => %{
        "thermal" => %{"thermal_quality_score" => 0, "thermal_quality_ready" => true}
      }
    }

    assert {:ok, ranked} =
             Runtime.rank_quality_candidates(
               %{"candidates" => %{" a " => candidate, " b " => nil}},
               %{}
             )

    assert {:ok, next} = Runtime.prepare_quality_next_round_request(ranked, %{})
    assert next["selected_candidate_id"] == " a "
    assert next["action"] == "replan"
    assert hd(next["source_rejected_candidates"])["candidate_id"] == " b "
  end

  test "selected objective cannot hide nonconvergence or blockers" do
    payload =
      put_in(ranking(true), ["ranking", Access.at(0), "objective"], %{"converged" => false})

    assert {:error, reason} = Runtime.prepare_quality_next_round_request(payload, %{})
    assert reason =~ "payload.ranking[0].objective.converged"

    payload =
      put_in(payload, ["ranking", Access.at(0), "objective"], %{
        "composite_quality_blocking_terms" => [%{"field" => "flux"}]
      })

    payload = put_in(payload, ["ranking", Access.at(0), "blocking_terms"], [])
    assert {:ok, next} = Runtime.prepare_quality_next_round_request(payload, %{})
    assert next["action"] == "replan"
  end

  test "validation blockers carry an actionable field through ranking to next round" do
    validation = %{
      "validation_contract" => "kyuubiki.summary_tolerance_validation/v1",
      "validation_passed" => false,
      "validation_checked_field_count" => 1,
      "validation_failed_field_count" => 1,
      "validation_missing_field_count" => 0,
      "validation_failures" => [%{"field" => "temperature", "relative_error" => 0.02}]
    }

    assert {:ok, ranked} =
             Runtime.rank_quality_candidates(
               %{"candidates" => %{"a" => %{"validation" => validation}}},
               %{}
             )

    assert {:ok, next} = Runtime.prepare_quality_next_round_request(ranked, %{})
    assert next["action"] == "replan"
    assert next["selected_iteration_hint"]["action"] == "fix_validation_failure"
    assert next["selected_iteration_hint"]["focus_field"] == "temperature"
    assert next["request_payload"]["optimization_hint"] == next["selected_iteration_hint"]
  end

  test "dominant term keeps its optimization focus" do
    payload =
      put_in(ranking(true), ["ranking", Access.at(0), "dominant_term"], %{
        "source" => "thermal",
        "domain" => "thermal",
        "dominant_term" => %{"field" => "thermal_temperature_max"}
      })

    assert {:ok, next} = Runtime.prepare_quality_next_round_request(payload, %{})
    assert next["selected_iteration_hint"]["focus_field"] == "thermal_temperature_max"
    assert next["selected_iteration_hint"]["focus_domain"] == "thermal"
  end

  for {state, action} <- [{"pass", "stop"}, {"warn", "continue"}, {"block", "replan"}] do
    test "coupled readiness #{state} controls next round" do
      payload =
        Map.put(ranking(true), "coupled_readiness", %{"coupled_readiness_state" => unquote(state)})

      assert {:ok, next} = Runtime.prepare_quality_next_round_request(payload, %{})
      assert next["action"] == unquote(action)
    end
  end
end
