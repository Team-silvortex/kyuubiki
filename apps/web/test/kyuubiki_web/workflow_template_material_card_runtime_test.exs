defmodule KyuubikiWeb.WorkflowTemplateMaterialCardRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorRuntime
  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "runs material card preflight template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-card-preflight-json", %{
               "material_card_input" => material_card()
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "material_card_input",
             "validate_card",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_card")
    assert dataset_value_emitted?(result, "material_card_preflight")
    assert summary["material_card_preflight_status"] == "pass"
    assert summary["material_card_id"] == "al-6061-t6"
    assert summary["material_card_reliability_envelope"]["trust_level"] == "review_ready"
  end

  test "runs material card batch preflight template through graph runner" do
    bad_card =
      material_card()
      |> Map.put("material_id", "bad-polymer")
      |> put_in(["parameters", "youngs_modulus", "unit"], "MPa")

    assert {:ok, result} =
             run_template("workflow.material-card-batch-preflight-json", %{
               "material_cards_input" => %{
                 "material_cards" => %{"aluminum" => material_card(), "polymer" => bad_card}
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "material_cards_input",
             "validate_cards",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_cards")
    assert dataset_value_emitted?(result, "material_card_batch_preflight")
    assert summary["material_card_batch_count"] == 2
    assert summary["material_card_batch_usable_count"] == 1
    assert summary["material_card_batch_issue_counts"] == %{"unit_mismatch" => 1}
  end

  test "runs material card screening score template through graph runner" do
    copper =
      material_card()
      |> Map.put("material_id", "cu-c110")
      |> Map.put("display_name", "Copper C110")
      |> put_in(["parameters", "youngs_modulus", "value"], 117.0e9)
      |> put_in(["parameters", "thermal_conductivity", "value"], 390.0)

    assert {:ok, result} =
             run_template("workflow.material-card-screening-score-json", %{
               "material_cards_input" => %{
                 "material_cards" => %{"aluminum" => material_card(), "copper" => copper}
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "material_cards_input",
             "validate_cards",
             "build_candidates",
             "score_candidates",
             "explain_screening",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_card_batch_preflight")
    assert dataset_value_emitted?(result, "material_card_candidate_summaries")
    assert dataset_value_emitted?(result, "material_score")
    assert dataset_value_emitted?(result, "material_card_screening_report")

    assert summary["material_card_screening_report_schema"] ==
             "kyuubiki.material-card-screening-report/v1"

    assert summary["material_screening_candidate_count"] == 2
    assert summary["material_screening_best_candidate_id"] == "copper"

    best = hd(summary["material_screening_rankings"])
    assert best["candidate_id"] == "copper"

    assert Enum.any?(
             best["criteria_breakdown"],
             &(&1["field"] == "material_param_thermal_conductivity")
           )
  end

  test "runs material card screening experiment plan template through graph runner" do
    copper =
      material_card()
      |> Map.put("material_id", "copper")
      |> Map.put("display_name", "Copper")
      |> put_in(["parameters", "youngs_modulus", "value"], 110.0e9)
      |> put_in(["parameters", "thermal_conductivity", "value"], 401.0)

    assert {:ok, result} =
             run_template("workflow.material-card-screening-experiment-plan-json", %{
               "material_cards_input" => %{
                 "material_cards" => %{"aluminum" => material_card(), "copper" => copper}
               }
             })

    summary = exported_summary(result)
    plan = summary["material_experiment_plan"]

    assert result["completed_nodes"] == [
             "material_cards_input",
             "validate_cards",
             "build_candidates",
             "score_candidates",
             "explain_screening",
             "plan_experiments",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_card_screening_report")
    assert dataset_value_emitted?(result, "material_experiment_plan")
    assert summary["material_experiment_plan_count"] == 2
    assert summary["material_experiment_primary_candidate_id"] == "copper"
    assert hd(plan)["candidate_id"] == "copper"
  end

  test "runs material card next-round request template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-card-next-round-request-json", %{
               "results_input" => %{
                 "results" => [
                   %{
                     "experiment_id" => "material-card-screening-1",
                     "candidate_id" => "aluminum",
                     "priority" => 1,
                     "expected_score" => 0.72,
                     "observed_score" => 0.74,
                     "passed" => true
                   }
                 ]
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "results_input",
             "analyze_results",
             "decide_iteration",
             "prepare_next_round",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_experiment_result_analysis")
    assert dataset_value_emitted?(result, "material_iteration_decision")
    assert dataset_value_emitted?(result, "material_card_next_round_request")
    assert summary["material_next_round_enabled"] == true
    assert summary["material_next_round_seed_candidate_id"] == "aluminum"

    assert summary["material_next_round_constraints"]["candidate_schema"] ==
             "kyuubiki.material-card/v1"
  end

  test "runs material card exploration snapshot template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-card-exploration-snapshot-json", %{
               "results_input" => %{
                 "results" => [
                   %{
                     "experiment_id" => "material-card-screening-1",
                     "candidate_id" => "aluminum",
                     "priority" => 1,
                     "expected_score" => 0.72,
                     "observed_score" => 0.74,
                     "passed" => true
                   }
                 ]
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "results_input",
             "analyze_results",
             "decide_iteration",
             "prepare_next_round",
             "build_snapshot",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_card_next_round_request")
    assert dataset_value_emitted?(result, "material_card_exploration_snapshot")

    assert summary["material_exploration_snapshot_schema"] ==
             "kyuubiki.material_exploration_snapshot/v1"

    assert summary["material_exploration_metadata"]["candidate_schema"] ==
             "kyuubiki.material-card/v1"

    assert summary["material_exploration_checkpoint"]["constraints"]["candidate_schema"] ==
             "kyuubiki.material-card/v1"
  end

  defp run_template(workflow_id, input_artifacts) do
    with {:ok, graph} <- WorkflowTemplateCatalog.graph_by_id(workflow_id) do
      WorkflowGraphRunner.run(graph, input_artifacts,
        dataset_contract: graph["dataset_contract"],
        execute_solve: &WorkflowOperatorRuntime.run_solve_operator/3,
        execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
        execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
        execute_export: &WorkflowOperatorRuntime.run_export_operator/3
      )
    end
  end

  defp exported_summary(result) do
    assert %{"format" => "json", "content" => content} =
             result["artifacts"]["json_output.json"]

    Jason.decode!(content)
  end

  defp dataset_value_emitted?(result, dataset_value) do
    Enum.any?(result["dataset_lineage"], &(&1["dataset_value"] == dataset_value))
  end

  defp material_card do
    %{
      "schema_version" => "kyuubiki.material-card/v1",
      "material_id" => "al-6061-t6",
      "display_name" => "Aluminum 6061-T6",
      "unit_system" => "si",
      "provenance" => %{
        "source_id" => "internal-screening-db",
        "source_label" => "Internal screening database"
      },
      "confidence" => %{"level" => "measured"},
      "applicability" => %{"temperature_range_c" => [-40.0, 150.0]},
      "parameters" => %{
        "youngs_modulus" => %{"kind" => "scalar", "value" => 69.0e9, "unit" => "Pa"},
        "thermal_conductivity" => %{
          "kind" => "scalar",
          "value" => 167.0,
          "unit" => "W/(m*K)"
        }
      }
    }
  end
end
