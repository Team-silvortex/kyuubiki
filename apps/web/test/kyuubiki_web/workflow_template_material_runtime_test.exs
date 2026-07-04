defmodule KyuubikiWeb.WorkflowTemplateMaterialRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorRuntime
  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "runs material margin template through graph runner and exports JSON" do
    assert {:ok, result} =
             run_template("workflow.material-margin-summary-json", %{
               "summary_input" => %{
                 "max_stress" => 300_000_000.0,
                 "max_temperature" => 320.0
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "summary_input",
             "evaluate_margin",
             "export_json",
             "json_output"
           ]

    assert result["performance"]["completed_node_count"] == 4
    assert dataset_value_emitted?(result, "material_margin")
    assert dataset_value_emitted?(result, "summary_json")

    assert summary["material_status"] == "fail"
    assert summary["material_critical_metric"] == "max_stress"
    assert summary["material_violation_count"] == 1
    assert summary["material_failure_index"] == 1.2
  end

  test "runs material candidate ranking template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-candidate-ranking-json", %{
               "candidates_input" => %{
                 "candidates" => %{
                   "aluminum" => %{
                     "material_status" => "pass",
                     "material_violation_count" => 0,
                     "material_failure_index" => 0.78,
                     "material_safety_factor" => 1.28
                   },
                   "titanium" => %{
                     "material_status" => "pass",
                     "material_violation_count" => 0,
                     "material_failure_index" => 0.52,
                     "material_safety_factor" => 1.92
                   },
                   "polymer" => %{
                     "material_status" => "fail",
                     "material_violation_count" => 1,
                     "material_failure_index" => 1.4,
                     "material_safety_factor" => 0.71,
                     "material_critical_metric" => "max_temperature"
                   }
                 }
               }
             })

    summary = exported_summary(result)

    assert dataset_value_emitted?(result, "material_ranking")
    assert summary["material_candidate_count"] == 3
    assert summary["material_feasible_count"] == 2
    assert summary["material_best_candidate_id"] == "titanium"
    assert summary["material_failure_reasons"] == %{"max_temperature" => 1}
  end

  test "runs material weighted score template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-candidate-score-json", %{
               "candidates_input" => %{
                 "candidates" => %{
                   "aluminum" => %{
                     "mass" => 1.8,
                     "cost" => 4.0,
                     "material_safety_factor" => 1.3,
                     "material_status" => "pass"
                   },
                   "titanium" => %{
                     "mass" => 2.4,
                     "cost" => 10.0,
                     "material_safety_factor" => 2.0,
                     "material_status" => "pass"
                   },
                   "polymer" => %{
                     "mass" => 1.0,
                     "cost" => 2.0,
                     "material_safety_factor" => 0.7,
                     "material_status" => "fail"
                   }
                 }
               }
             })

    summary = exported_summary(result)

    assert dataset_value_emitted?(result, "material_score")
    assert summary["material_score_candidate_count"] == 3
    assert summary["material_score_feasible_count"] == 2
    assert summary["material_score_best_candidate_id"] == "titanium"
    assert hd(summary["material_score_rankings"])["candidate_id"] == "titanium"
  end

  test "runs material fatigue life template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-fatigue-life-json", %{
               "candidates_input" => %{
                 "candidates" => %{
                   "aluminum" => %{
                     "stress_amplitude" => 95.0,
                     "mean_stress" => 20.0,
                     "ultimate_strength" => 310.0,
                     "material_status" => "pass"
                   },
                   "polymer" => %{
                     "stress_amplitude" => 160.0,
                     "material_status" => "pass"
                   }
                 }
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "candidates_input",
             "estimate_fatigue",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_fatigue")
    assert summary["material_fatigue_candidate_count"] == 2
    assert summary["material_fatigue_pass_count"] == 1
    assert summary["material_fatigue_best_candidate_id"] == "aluminum"
  end

  test "runs material experiment plan template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-experiment-plan-json", %{
               "candidates_input" => material_candidate_input()
             })

    summary = exported_summary(result)
    plan = summary["material_experiment_plan"]

    assert result["completed_nodes"] == [
             "candidates_input",
             "score_candidates",
             "plan_experiments",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_score")
    assert dataset_value_emitted?(result, "material_experiment_plan")
    assert summary["material_experiment_plan_count"] == 2
    assert summary["material_experiment_primary_candidate_id"] == "titanium"
    assert hd(plan)["candidate_id"] == "titanium"
    assert Enum.at(plan, 1)["candidate_id"] == "aluminum"
  end

  test "runs material experiment result analysis template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-experiment-result-analysis-json", %{
               "results_input" => %{
                 "results" => [
                   %{
                     "experiment_id" => "coupon-1",
                     "candidate_id" => "titanium",
                     "priority" => 1,
                     "expected_score" => 0.58,
                     "observed_score" => 0.72,
                     "passed" => true
                   },
                   %{
                     "experiment_id" => "coupon-2",
                     "candidate_id" => "aluminum",
                     "priority" => 2,
                     "expected_score" => 0.52,
                     "observed_score" => 0.49,
                     "passed" => true
                   }
                 ]
               }
             })

    summary = exported_summary(result)

    assert result["completed_nodes"] == [
             "results_input",
             "analyze_results",
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_experiment_result_analysis")
    assert summary["material_experiment_result_count"] == 2
    assert summary["material_experiment_validated_count"] == 2
    assert summary["material_experiment_best_candidate_id"] == "titanium"
    assert hd(summary["material_experiment_result_rankings"])["candidate_id"] == "titanium"
  end

  test "runs material iteration decision template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-iteration-decision-json", %{
               "results_input" => %{
                 "results" => [
                   %{
                     "experiment_id" => "coupon-1",
                     "candidate_id" => "titanium",
                     "priority" => 1,
                     "expected_score" => 0.58,
                     "observed_score" => 0.72,
                     "passed" => true
                   },
                   %{
                     "experiment_id" => "coupon-2",
                     "candidate_id" => "aluminum",
                     "priority" => 2,
                     "expected_score" => 0.52,
                     "observed_score" => 0.49,
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
             "export_json",
             "json_output"
           ]

    assert dataset_value_emitted?(result, "material_experiment_result_analysis")
    assert dataset_value_emitted?(result, "material_iteration_decision")
    assert summary["material_iteration_decision"] == "stop"
    assert summary["material_iteration_next_action"] == "accept_candidate"
    assert summary["material_iteration_best_candidate_id"] == "titanium"
  end

  test "runs material next-round request template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-next-round-request-json", %{
               "results_input" => %{
                 "results" => [
                   %{
                     "experiment_id" => "coupon-1",
                     "candidate_id" => "aluminum",
                     "priority" => 1,
                     "expected_score" => 0.58,
                     "observed_score" => 0.62,
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

    assert dataset_value_emitted?(result, "material_iteration_decision")
    assert dataset_value_emitted?(result, "material_next_round_request")
    assert summary["material_next_round_enabled"] == true
    assert summary["material_next_round_action"] == "run_more_experiments"
    assert summary["material_next_round_seed_candidate_id"] == "aluminum"
  end

  test "runs material exploration snapshot template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-exploration-snapshot-json", %{
               "results_input" => %{
                 "results" => [
                   %{
                     "experiment_id" => "coupon-1",
                     "candidate_id" => "aluminum",
                     "priority" => 1,
                     "expected_score" => 0.58,
                     "observed_score" => 0.62,
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

    assert dataset_value_emitted?(result, "material_next_round_request")
    assert dataset_value_emitted?(result, "material_exploration_snapshot")

    assert summary["material_exploration_snapshot_schema"] ==
             "kyuubiki.material_exploration_snapshot/v1"

    assert summary["material_exploration_status"] == "active"
    assert summary["material_exploration_seed_candidate_id"] == "aluminum"
  end

  test "runs material Pareto frontier template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-pareto-frontier-json", %{
               "candidates_input" => %{
                 "candidates" => %{
                   "light" => %{
                     "mass" => 1.0,
                     "material_safety_factor" => 1.2,
                     "material_status" => "pass"
                   },
                   "strong" => %{
                     "mass" => 2.0,
                     "material_safety_factor" => 1.8,
                     "material_status" => "pass"
                   },
                   "dominated" => %{
                     "mass" => 3.0,
                     "material_safety_factor" => 1.1,
                     "material_status" => "pass"
                   },
                   "failed" => %{
                     "mass" => 0.5,
                     "material_safety_factor" => 2.0,
                     "material_status" => "fail"
                   }
                 }
               }
             })

    summary = exported_summary(result)
    frontier_ids = Enum.map(summary["material_pareto_frontier"], & &1["candidate_id"])

    assert dataset_value_emitted?(result, "material_pareto")
    assert summary["material_pareto_candidate_count"] == 4
    assert summary["material_pareto_feasible_count"] == 3
    assert summary["material_pareto_frontier_count"] == 2
    assert "light" in frontier_ids
    assert "strong" in frontier_ids
    assert Enum.any?(summary["material_pareto_dominated"], &(&1["candidate_id"] == "failed"))
  end

  test "runs material study envelope ranking template through graph runner" do
    assert {:ok, result} =
             run_template("workflow.material-study-envelope-ranking-json", %{
               "material_rows" => %{
                 "rows" => [
                   %{
                     "case_id" => "cool_stiff",
                     "summaries" => %{
                       "thermal" => %{"max_temperature" => 90.0},
                       "structural" => %{"max_stress" => 180.0}
                     }
                   },
                   %{
                     "case_id" => "hot_light",
                     "summaries" => %{
                       "thermal" => %{"max_temperature" => 130.0},
                       "structural" => %{"max_stress" => 120.0}
                     }
                   }
                 ]
               }
             })

    summary = exported_summary(result)

    assert dataset_value_emitted?(result, "material_envelopes")
    assert dataset_value_emitted?(result, "material_envelope_ranking")
    assert dataset_value_emitted?(result, "material_envelope_pareto")
    assert dataset_value_emitted?(result, "material_envelope_decision_bundle")

    assert summary["bundle_source_count"] == 2
    assert MapSet.new(summary["bundle_sources"]) == MapSet.new(["ranking", "pareto"])
    assert summary["bundle_payloads"]["ranking"]["material_best_candidate_id"] == "cool_stiff"

    assert summary["bundle_payloads"]["pareto"]["material_pareto_best_candidate_id"] ==
             "cool_stiff"
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

  defp material_candidate_input do
    %{
      "candidates" => %{
        "aluminum" => %{
          "mass" => 1.8,
          "cost" => 4.0,
          "material_safety_factor" => 1.3,
          "material_status" => "pass"
        },
        "titanium" => %{
          "mass" => 2.4,
          "cost" => 10.0,
          "material_safety_factor" => 2.0,
          "material_status" => "pass"
        },
        "polymer" => %{
          "mass" => 1.0,
          "cost" => 2.0,
          "material_safety_factor" => 0.7,
          "material_status" => "fail"
        }
      }
    }
  end
end
