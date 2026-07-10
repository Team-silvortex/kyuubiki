use crate::{
    HeadlessWorkflowStep, MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION,
    MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION, MATERIAL_EXPLORATION_SCHEMA_VERSION,
    MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION,
    build_material_exploration_next_round_execution_plan,
    build_material_exploration_next_round_plan, build_material_exploration_run,
    build_material_exploration_run_for_iteration, build_material_study_execution_plan,
    material_exploration_steps,
};
use serde_json::{Value, json};

#[test]
fn exposes_candidate_solve_steps_for_each_material_study() {
    for study in [
        "heat-spreader",
        "dielectric-screening",
        "thermo-shield",
        "structural-panel",
        "composite-thermo-electric-panel",
    ] {
        let steps = material_exploration_steps(study).expect("material steps");
        let solve_count = steps
            .iter()
            .filter(|step| step.action.starts_with("solve_"))
            .count();
        assert_eq!(solve_count, 3);
    }
}

#[test]
fn builds_material_study_execution_plan_without_running_solver() {
    let plan =
        build_material_study_execution_plan("composite-thermo-electric-panel").expect("plan");

    assert_eq!(
        plan.schema_version,
        MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION
    );
    assert_eq!(plan.study_id, "material_composite_thermo_electric_panel");
    assert_eq!(plan.step_count, 3);
    assert_eq!(plan.solve_step_count, 3);
    assert_eq!(plan.candidate_count, 3);
    assert!(
        plan.candidate_ids
            .contains(&"copper_ptfe_glass_epoxy".to_string())
    );
    assert!(
        plan.actions
            .iter()
            .all(|action| action == "solve_composite_thermo_electric_panel")
    );
    assert!(
        plan.recommended_command
            .contains("composite-thermo-electric-panel")
    );
    assert_eq!(plan.steps.len(), plan.step_count);
}

#[test]
fn builds_stable_exploration_run_contract() {
    let run = build_material_exploration_run(
        "dielectric-screening",
        "unit-test",
        vec![
            json!({ "max_electric_field": 42.0e6, "max_flux_density": 1.2e-3 }),
            json!({ "max_electric_field": 38.0e6, "max_flux_density": 3.3e-3 }),
            json!({ "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }),
        ],
    )
    .expect("exploration run");

    assert_eq!(run.schema_version, MATERIAL_EXPLORATION_SCHEMA_VERSION);
    assert_eq!(run.mode, "unit-test");
    assert_eq!(run.iteration, 1);
    assert_eq!(run.candidate_count, 3);
    assert_eq!(
        run.report["winner_candidate_id"].as_str(),
        Some("polyimide_film")
    );
    assert_eq!(run.next_round.decision, "expand_around_winner");
    assert_eq!(run.next_round.iteration, 2);
    assert!(
        run.next_round
            .focus_candidate_ids
            .contains(&"polyimide_film".to_string())
    );
    assert!(
        run.next_round
            .actions
            .contains(&"run_next_quality_batch".to_string())
    );
}

#[test]
fn builds_next_round_from_explicit_iteration() {
    let run = build_material_exploration_run_for_iteration(
        "dielectric-screening",
        "unit-test",
        vec![
            json!({ "max_electric_field": 42.0e6, "max_flux_density": 1.2e-3 }),
            json!({ "max_electric_field": 38.0e6, "max_flux_density": 3.3e-3 }),
            json!({ "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }),
        ],
        2,
    )
    .expect("exploration run");

    assert_eq!(run.iteration, 2);
    assert_eq!(run.next_round.iteration, 3);
}

#[test]
fn next_round_plan_repairs_incomplete_reports_before_expansion() {
    let report = json!({
        "winner_candidate_id": "candidate-a",
        "warnings": ["candidate-b is missing max_stress_pa"],
        "candidates": [
            { "candidate_id": "candidate-a", "rank": 1, "score": 0.8 },
            { "candidate_id": "candidate-b", "rank": 2, "score": 0.6 }
        ],
        "reliability": {
            "quality_gates": [
                { "id": "gate.result_completeness", "status": "violate" }
            ]
        }
    });

    let plan = build_material_exploration_next_round_plan(&report, 3);

    assert_eq!(
        plan.schema_version,
        MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION
    );
    assert_eq!(plan.iteration, 4);
    assert_eq!(plan.decision, "repair_or_rerun");
    assert_eq!(plan.focus_candidate_ids, vec!["candidate-a", "candidate-b"]);
    assert!(
        plan.actions
            .contains(&"rerun_incomplete_candidates".to_string())
    );
    assert!(
        plan.rationale
            .iter()
            .any(|line| line.contains("gate.result_completeness"))
    );
}

#[test]
fn next_round_plan_mitigates_design_risk_without_missing_metrics() {
    let report = json!({
        "winner_candidate_id": "candidate-a",
        "warnings": ["candidate-a exceeds prototype interface risk threshold: 0.910"],
        "candidates": [
            { "candidate_id": "candidate-a", "rank": 1, "score": 0.8 },
            { "candidate_id": "candidate-b", "rank": 2, "score": 0.6 }
        ],
        "reliability": {
            "quality_gates": [
                { "id": "gate.interface_risk.prototype", "status": "violate" }
            ]
        }
    });

    let plan = build_material_exploration_next_round_plan(&report, 1);

    assert_eq!(plan.decision, "mitigate_design_risk");
    assert_eq!(plan.focus_candidate_ids, vec!["candidate-a", "candidate-b"]);
    assert!(
        plan.actions
            .contains(&"generate_lower_risk_neighbor_candidates".to_string())
    );
    assert!(
        plan.rationale
            .iter()
            .any(|line| line.contains("gate.interface_risk.prototype"))
    );
}

#[test]
fn next_round_plan_prefers_reliability_summary_blocking_gate_ids() {
    let report = json!({
        "winner_candidate_id": "candidate-a",
        "warnings": [],
        "candidates": [
            { "candidate_id": "candidate-a", "rank": 1, "score": 0.8 }
        ],
        "reliability": {
            "summary": {
                "decision": "blocked_by_quality_gates",
                "blocking_gate_ids": ["gate.interface_risk.prototype"]
            },
            "quality_gates": [
                { "id": "gate.result_completeness", "status": "observe" },
                { "id": "gate.interface_risk.prototype", "status": "violate" }
            ]
        }
    });

    let plan = build_material_exploration_next_round_plan(&report, 1);

    assert_eq!(plan.decision, "mitigate_design_risk");
    assert!(
        plan.rationale
            .iter()
            .any(|line| line.contains("gate.interface_risk.prototype"))
    );
    assert!(
        !plan
            .rationale
            .iter()
            .any(|line| line.contains("gate.result_completeness"))
    );
}

#[test]
fn next_round_execution_plan_filters_repair_reruns_to_focus_candidates() {
    let exploration = json!({
        "schema_version": MATERIAL_EXPLORATION_SCHEMA_VERSION,
        "study": "material_dielectric_screening",
        "next_round": {
            "iteration": 2,
            "decision": "repair_or_rerun",
            "focus_candidate_ids": ["polyimide_film"],
            "actions": ["rerun_incomplete_candidates"]
        }
    });

    let plan =
        build_material_exploration_next_round_execution_plan(&exploration).expect("next plan");

    assert_eq!(
        plan.schema_version,
        MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION
    );
    assert_eq!(plan.decision, "repair_or_rerun");
    assert_eq!(plan.runnable_step_count, 1);
    assert!(
        plan.steps
            .iter()
            .all(|step| candidate_id_for_step(step) == Some("polyimide_film"))
    );
}

#[test]
fn next_round_execution_plan_exports_machine_readable_optimization_objectives() {
    let exploration = json!({
        "schema_version": MATERIAL_EXPLORATION_SCHEMA_VERSION,
        "study": "material_dielectric_screening",
        "report": {
            "winner_candidate_id": "polyimide_film",
            "metric_specs": [
                {
                    "id": "max_electric_field",
                    "label": "Max electric field",
                    "unit": "V/m",
                    "objective": "minimize",
                    "weight": 0.5
                },
                {
                    "id": "max_flux_density",
                    "label": "Max flux density",
                    "unit": "Wb/m^2",
                    "objective": "observe",
                    "weight": 0.0
                }
            ],
            "reliability": {
                "quality_gates": [
                    { "id": "gate.breakdown_margin", "status": "violate" }
                ]
            }
        },
        "next_round": {
            "iteration": 2,
            "decision": "mitigate_design_risk",
            "focus_candidate_ids": ["polyimide_film"],
            "actions": ["generate_lower_risk_neighbor_candidates"]
        }
    });

    let plan =
        build_material_exploration_next_round_execution_plan(&exploration).expect("next plan");

    assert_eq!(
        plan.optimization_objectives["schema_version"].as_str(),
        Some("kyuubiki.material-next-round-optimization-objectives/v1")
    );
    assert_eq!(
        plan.optimization_objectives["mode"].as_str(),
        Some("risk_constrained_search")
    );
    assert_eq!(
        plan.optimization_objectives["winner_candidate_id"].as_str(),
        Some("polyimide_film")
    );
    assert_eq!(
        plan.optimization_objectives["primary_metric_ids"][0].as_str(),
        Some("max_electric_field")
    );
    assert_eq!(
        plan.optimization_objectives["violated_quality_gate_ids"][0].as_str(),
        Some("gate.breakdown_margin")
    );
}

#[test]
fn next_round_execution_plan_expands_with_full_study_steps() {
    let exploration = json!({
        "schema_version": MATERIAL_EXPLORATION_SCHEMA_VERSION,
        "study": "material_structural_panel_screening",
        "next_round": {
            "iteration": 2,
            "decision": "expand_around_winner",
            "focus_candidate_ids": ["carbon_fiber_quasi_iso"],
            "actions": ["generate_neighbor_candidates", "run_next_quality_batch"]
        }
    });

    let plan =
        build_material_exploration_next_round_execution_plan(&exploration).expect("next plan");

    assert_eq!(plan.decision, "expand_around_winner");
    assert_eq!(plan.runnable_step_count, 9);
    assert_eq!(plan.steps.len(), 9);
}

fn candidate_id_for_step(step: &HeadlessWorkflowStep) -> Option<&str> {
    step.payload
        .get("research")
        .and_then(|research| research.get("candidate_id"))
        .and_then(Value::as_str)
}
