use crate::{
    HeadlessWorkflowStep, MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION,
    MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION, MATERIAL_EXPLORATION_SCHEMA_VERSION,
    build_material_exploration_next_round_execution_plan,
    build_material_exploration_next_round_plan, build_material_exploration_run,
    build_material_exploration_run_for_iteration, material_exploration_steps,
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
