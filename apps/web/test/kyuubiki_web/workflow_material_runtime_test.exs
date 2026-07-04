defmodule KyuubikiWeb.WorkflowMaterialRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes material study envelope transform" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.compose_material_study_envelope")

    assert operator["family"] == "material_study_envelope"
    assert "envelope" in operator["capability_tags"]
    assert "headless_safe" in operator["capability_tags"]
  end

  test "evaluates material margins from solver summary limits" do
    assert {:ok, margin} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_material_margins",
               %{"max_stress" => 225.0, "max_temperature" => 80.0},
               %{
                 "limits" => %{
                   "max_stress" => %{"limit" => 200.0},
                   "max_temperature" => %{"limit" => 120.0}
                 }
               }
             )

    assert margin["material_constraint_count"] == 2
    assert margin["material_violation_count"] == 1
    assert margin["material_status"] == "fail"
    assert margin["material_critical_metric"] == "max_stress"
    assert margin["material_failure_index"] == 1.125
  end

  test "composes material study envelope from multi-domain summaries" do
    assert {:ok, envelope} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_material_study_envelope",
               %{
                 "candidate_id" => "cool_stiff",
                 "summaries" => %{
                   "thermal" => %{"max_temperature" => 90.0},
                   "structural" => %{"max_stress" => 180.0},
                   "electrostatic" => %{"max_electric_field" => 3.0}
                 }
               },
               %{}
             )

    assert envelope["material_envelope_contract"] == "kyuubiki.material_study_envelope/v1"
    assert envelope["material_envelope_candidate_id"] == "cool_stiff"
    assert envelope["material_envelope_status"] == "pass"
    assert envelope["material_envelope_metric_count"] == 3
    assert envelope["material_envelope_domain_count"] == 3
    assert envelope["material_envelope_critical_metric"] == "thermal.temperature"
  end

  test "composes material envelope batch for ranking and Pareto operators" do
    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_material_study_envelope",
               %{
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
               },
               %{}
             )

    assert batch["material_envelope_batch_contract"] ==
             "kyuubiki.material_study_envelope_batch/v1"

    assert batch["material_envelope_candidate_count"] == 2
    assert batch["material_envelope_best_candidate_id"] == "cool_stiff"

    assert {:ok, ranking} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.rank_material_candidates",
               batch,
               %{"margin_prefix" => "material_envelope"}
             )

    assert ranking["material_best_candidate_id"] == "cool_stiff"

    assert {:ok, pareto} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.extract_material_pareto_frontier",
               %{"candidates" => batch["candidates"]},
               %{
                 "feasible_field" => "material_envelope_status",
                 "objectives" => [
                   %{"field" => "material_envelope_score", "goal" => "min"},
                   %{"field" => "material_envelope_safety_factor", "goal" => "max"}
                 ]
               }
             )

    assert pareto["material_pareto_candidate_count"] == 2
    assert pareto["material_pareto_best_candidate_id"] == "cool_stiff"
  end

  test "ranks material candidates by feasibility and safety factor" do
    assert {:ok, ranking} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.rank_material_candidates",
               %{
                 "candidates" => %{
                   "aluminum" => %{
                     "material_status" => "pass",
                     "material_violation_count" => 0,
                     "material_failure_index" => 0.82,
                     "material_safety_factor" => 1.21,
                     "material_critical_metric" => "max_stress"
                   },
                   "titanium" => %{
                     "material_status" => "pass",
                     "material_violation_count" => 0,
                     "material_failure_index" => 0.55,
                     "material_safety_factor" => 1.81,
                     "material_critical_metric" => "max_temperature"
                   },
                   "polymer" => %{
                     "material_status" => "fail",
                     "material_violation_count" => 1,
                     "material_failure_index" => 1.4,
                     "material_safety_factor" => 0.71,
                     "material_critical_metric" => "max_temperature"
                   }
                 }
               },
               %{}
             )

    assert ranking["material_candidate_count"] == 3
    assert ranking["material_feasible_count"] == 2
    assert ranking["material_best_candidate_id"] == "titanium"
    assert ranking["material_failure_reasons"] == %{"max_temperature" => 1}
  end

  test "estimates material fatigue life for candidate stress amplitudes" do
    assert {:ok, fatigue} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.estimate_material_fatigue_life",
               %{
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
               },
               %{
                 "fatigue_strength" => 120.0,
                 "reference_cycles" => 1.0e6,
                 "slope_exponent" => 4.0,
                 "target_cycles" => 8.0e5
               }
             )

    assessments = fatigue["material_fatigue_assessments"]

    assert fatigue["material_fatigue_candidate_count"] == 2
    assert fatigue["material_fatigue_pass_count"] == 1
    assert fatigue["material_fatigue_best_candidate_id"] == "aluminum"
    assert hd(assessments)["fatigue_status"] == "pass"
    assert hd(assessments)["fatigue_correction"]["kind"] == "goodman"
    assert List.last(assessments)["candidate_id"] == "polymer"
    assert List.last(assessments)["fatigue_status"] == "fail"
  end

  test "evaluates material thermal shock risk for temperature cycle candidates" do
    assert {:ok, shock} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_material_thermal_shock",
               %{
                 "candidates" => %{
                   "alloy" => %{
                     "temperature_delta" => 160.0,
                     "thermal_expansion" => 1.2e-5,
                     "youngs_modulus" => 70.0e9,
                     "poisson_ratio" => 0.33,
                     "yield_strength" => 320.0e6,
                     "material_status" => "pass"
                   },
                   "ceramic" => %{
                     "temperature_delta" => 160.0,
                     "thermal_expansion" => 8.0e-6,
                     "youngs_modulus" => 300.0e9,
                     "poisson_ratio" => 0.22,
                     "tensile_strength" => 180.0e6,
                     "fracture_toughness" => 3.0e6,
                     "flaw_size" => 0.001,
                     "material_status" => "pass"
                   }
                 }
               },
               %{"constraint_factor" => 0.7}
             )

    assessments = shock["material_thermal_shock_assessments"]

    assert shock["material_thermal_shock_candidate_count"] == 2
    assert shock["material_thermal_shock_pass_count"] == 1
    assert shock["material_thermal_shock_best_candidate_id"] == "alloy"
    assert hd(assessments)["thermal_shock_status"] == "pass"
    assert List.last(assessments)["candidate_id"] == "ceramic"
    assert List.last(assessments)["thermal_shock_status"] == "fail"
    assert List.last(assessments)["thermal_shock_fracture_index"] > 0.0
  end

  test "scores material candidates with weighted optimization criteria" do
    assert {:ok, scored} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_material_candidates",
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
               },
               %{
                 "criteria" => [
                   %{"field" => "mass", "goal" => "min", "weight" => 0.35},
                   %{"field" => "cost", "goal" => "min", "weight" => 0.15},
                   %{"field" => "material_safety_factor", "goal" => "max", "weight" => 0.5}
                 ]
               }
             )

    rankings = scored["material_score_rankings"]

    assert scored["material_score_candidate_count"] == 3
    assert scored["material_score_feasible_count"] == 2
    assert scored["material_score_best_candidate_id"] == "titanium"
    assert hd(rankings)["candidate_id"] == "titanium"
    assert Enum.at(rankings, 1)["candidate_id"] == "aluminum"
    assert List.last(rankings)["candidate_id"] == "polymer"
    assert hd(rankings)["criteria_breakdown"] |> length() == 3
  end

  test "plans material experiments from scored candidate rankings" do
    assert {:ok, scored} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_material_candidates",
               material_candidate_payload(),
               material_score_config()
             )

    assert {:ok, plan} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.plan_material_experiments",
               scored,
               %{"top_k" => 2, "label" => "coupon"}
             )

    experiments = plan["material_experiment_plan"]

    assert plan["material_experiment_plan_count"] == 2
    assert plan["material_experiment_primary_candidate_id"] == "titanium"
    assert hd(experiments)["experiment_id"] == "coupon-1"
    assert hd(experiments)["candidate_id"] == "titanium"
    assert Enum.at(experiments, 1)["candidate_id"] == "aluminum"
  end

  test "analyzes observed material experiment results against expected scores" do
    assert {:ok, analysis} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.analyze_material_experiment_results",
               %{
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
                   },
                   %{
                     "experiment_id" => "coupon-3",
                     "candidate_id" => "polymer",
                     "priority" => 3,
                     "expected_score" => 0.4,
                     "observed_score" => 0.9,
                     "passed" => false
                   }
                 ]
               },
               %{}
             )

    rankings = analysis["material_experiment_result_rankings"]

    assert analysis["material_experiment_result_count"] == 3
    assert analysis["material_experiment_validated_count"] == 2
    assert analysis["material_experiment_best_candidate_id"] == "titanium"
    assert hd(rankings)["candidate_id"] == "titanium"
    assert_in_delta hd(rankings)["score_error"], 0.14, 1.0e-12
    assert List.last(rankings)["candidate_id"] == "polymer"
    assert List.last(rankings)["status"] == "rejected"
  end

  test "decides material exploration iteration from analyzed experiment results" do
    assert {:ok, analysis} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.analyze_material_experiment_results",
               %{
                 "results" => [
                   %{
                     "experiment_id" => "coupon-1",
                     "candidate_id" => "titanium",
                     "priority" => 1,
                     "expected_score" => 0.58,
                     "observed_score" => 0.72,
                     "passed" => true
                   }
                 ]
               },
               %{}
             )

    assert {:ok, decision} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.decide_material_iteration",
               analysis,
               %{"target_score" => 0.7, "min_validated" => 1}
             )

    assert decision["material_iteration_decision"] == "stop"
    assert decision["material_iteration_next_action"] == "accept_candidate"
    assert decision["material_iteration_best_candidate_id"] == "titanium"
    assert decision["material_iteration_best_observed_score"] == 0.72
  end

  test "continues material exploration when target score is not reached" do
    assert {:ok, decision} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.decide_material_iteration",
               %{
                 "material_experiment_validated_count" => 1,
                 "material_experiment_best_observed_score" => 0.62,
                 "material_experiment_result_rankings" => [
                   %{
                     "candidate_id" => "aluminum",
                     "observed_score" => 0.62,
                     "status" => "validated"
                   }
                 ]
               },
               %{"target_score" => 0.7, "current_round" => 2, "max_rounds" => 5}
             )

    assert decision["material_iteration_decision"] == "continue"
    assert decision["material_iteration_next_action"] == "run_more_experiments"
  end

  test "prepares next-round material orchestration request from iteration decision" do
    assert {:ok, request} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.prepare_material_next_round_request",
               %{
                 "material_iteration_decision" => "continue",
                 "material_iteration_next_action" => "run_more_experiments",
                 "material_iteration_best_candidate_id" => "aluminum",
                 "material_iteration_target_score" => 0.7,
                 "material_iteration_current_round" => 2
               },
               %{"requested_candidate_count" => 4}
             )

    assert request["material_next_round_enabled"] == true
    assert request["material_next_round_action"] == "run_more_experiments"
    assert request["material_next_round_index"] == 3
    assert request["material_next_round_requested_candidate_count"] == 4
    assert request["material_next_round_seed_candidate_id"] == "aluminum"
  end

  test "builds material exploration snapshot from next-round request" do
    assert {:ok, snapshot} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.build_material_exploration_snapshot",
               %{
                 "material_next_round_enabled" => true,
                 "material_next_round_action" => "run_more_experiments",
                 "material_next_round_index" => 3,
                 "material_next_round_requested_candidate_count" => 4,
                 "material_next_round_seed_candidate_id" => "aluminum",
                 "material_next_round_target_score" => 0.7,
                 "material_next_round_source_decision" => "continue"
               },
               %{"metadata" => %{"project_id" => "mat-study"}}
             )

    assert snapshot["material_exploration_snapshot_schema"] ==
             "kyuubiki.material_exploration_snapshot/v1"

    assert snapshot["material_exploration_status"] == "active"
    assert snapshot["material_exploration_round_index"] == 3.0
    assert snapshot["material_exploration_seed_candidate_id"] == "aluminum"
    assert snapshot["material_exploration_metadata"] == %{"project_id" => "mat-study"}
  end

  test "extracts material Pareto frontier for multi objective candidates" do
    assert {:ok, frontier} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.extract_material_pareto_frontier",
               %{
                 "candidates" => %{
                   "light" => %{"mass" => 1.0, "stiffness" => 8.0, "material_status" => "pass"},
                   "stiff" => %{"mass" => 2.0, "stiffness" => 12.0, "material_status" => "pass"},
                   "weak" => %{"mass" => 3.0, "stiffness" => 6.0, "material_status" => "pass"},
                   "failed" => %{"mass" => 0.5, "stiffness" => 20.0, "material_status" => "fail"}
                 }
               },
               %{
                 "objectives" => [
                   %{"field" => "mass", "goal" => "min"},
                   %{"field" => "stiffness", "goal" => "max"}
                 ]
               }
             )

    ids = Enum.map(frontier["material_pareto_frontier"], & &1["candidate_id"])

    assert frontier["material_pareto_candidate_count"] == 4
    assert frontier["material_pareto_feasible_count"] == 3
    assert frontier["material_pareto_frontier_count"] == 2
    assert "light" in ids
    assert "stiff" in ids
    assert Enum.any?(frontier["material_pareto_dominated"], &(&1["candidate_id"] == "weak"))
    assert Enum.any?(frontier["material_pareto_dominated"], &(&1["candidate_id"] == "failed"))
  end

  defp material_candidate_payload do
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

  defp material_score_config do
    %{
      "criteria" => [
        %{"field" => "mass", "goal" => "min", "weight" => 0.35},
        %{"field" => "cost", "goal" => "min", "weight" => 0.15},
        %{"field" => "material_safety_factor", "goal" => "max", "weight" => 0.5}
      ]
    }
  end
end
