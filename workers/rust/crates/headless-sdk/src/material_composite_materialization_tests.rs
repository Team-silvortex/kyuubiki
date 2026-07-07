use crate::{
    build_composite_materialized_candidate_report, build_composite_materialized_candidate_steps,
    build_material_exploration_next_round_plan,
};
use serde_json::json;

#[test]
fn materialized_composite_candidates_become_solver_steps() {
    let plan = json!({
        "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
        "status": "ready_for_solver_rerun",
        "materialized_candidates": [
            materialized_candidate(
                "copper_ptfe_glass_epoxy__add_compliant_interlayer",
                "draft-a",
                "add_compliant_interlayer"
            ),
            materialized_candidate(
                "copper_ptfe_glass_epoxy__replace_high_cte_dielectric",
                "draft-b",
                "replace_high_cte_dielectric"
            )
        ]
    });

    let steps = build_composite_materialized_candidate_steps(&plan).unwrap();

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].action, "solve_composite_thermo_electric_panel");
    let first_research = &steps[0].payload["research"];
    assert_eq!(
        first_research["candidate_id"],
        "copper_ptfe_glass_epoxy__add_compliant_interlayer"
    );
    assert_eq!(first_research["source_draft_id"], "draft-a");
    assert_eq!(
        first_research["materialization_status"],
        "prototype_materialized_requires_solver_rerun"
    );
    assert!(steps[0].payload.get("electrostatic_model").is_some());
    assert!(steps[0].payload.get("heat_model").is_some());
    assert!(steps[0].payload.get("thermal_model").is_some());
    assert_eq!(
        steps[1].payload["research"]["materials"]["dielectric"],
        "polyimide"
    );
}

#[test]
fn materialized_composite_steps_reject_unknown_sources() {
    let plan = json!({
        "materialized_candidates": [{
            "schema_version": "kyuubiki.materialized-candidate-spec/v1",
            "candidate_id": "ghost__add_compliant_interlayer",
            "source_draft_id": "draft-ghost",
            "source_candidate_id": "ghost",
            "strategy": "add_compliant_interlayer",
            "study": "material_composite_thermo_electric_panel",
            "status": "requires_solver_rerun"
        }]
    });

    let error = build_composite_materialized_candidate_steps(&plan).unwrap_err();

    assert!(error.contains("unknown source"));
}

#[test]
fn materialized_composite_results_build_next_round_ready_report() {
    let plan = json!({
        "materialized_candidates": [
            materialized_candidate(
                "copper_ptfe_glass_epoxy__add_compliant_interlayer",
                "draft-a",
                "add_compliant_interlayer"
            ),
            materialized_candidate(
                "copper_ptfe_glass_epoxy__replace_high_cte_dielectric",
                "draft-b",
                "replace_high_cte_dielectric"
            )
        ]
    });
    let steps = build_composite_materialized_candidate_steps(&plan).unwrap();
    let report = build_composite_materialized_candidate_report(&[
        composite_result(&steps[0].payload["research"], 2.1e6, 120.0, 180.0e6),
        composite_result(&steps[1].payload["research"], 1.8e6, 118.0, 160.0e6),
    ])
    .unwrap();

    assert_eq!(
        report["schema_version"],
        "kyuubiki.composite-materialized-candidate-report/v1"
    );
    assert_eq!(report["candidate_count"], 2);
    assert_eq!(report["candidates"][0]["source_draft_id"], json!("draft-b"));
    assert!(
        report["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.len() >= 4)
    );

    let next_round = build_material_exploration_next_round_plan(&report, 2);

    assert_eq!(next_round.iteration, 3);
    assert!(!next_round.focus_candidate_ids.is_empty());
}

fn materialized_candidate(candidate_id: &str, draft_id: &str, strategy: &str) -> serde_json::Value {
    json!({
        "schema_version": "kyuubiki.materialized-candidate-spec/v1",
        "candidate_id": candidate_id,
        "source_draft_id": draft_id,
        "source_candidate_id": "copper_ptfe_glass_epoxy",
        "strategy": strategy,
        "study": "material_composite_thermo_electric_panel",
        "required_result_schema": "kyuubiki.material-composite-report/v1",
        "status": "requires_solver_rerun"
    })
}

fn composite_result(
    research: &serde_json::Value,
    max_electric_field: f64,
    max_temperature: f64,
    max_stress: f64,
) -> serde_json::Value {
    json!({
        "schema_version": "kyuubiki.composite-thermo-electric-panel-result/v1",
        "research": research,
        "electrostatic": { "max_electric_field": max_electric_field },
        "heat": { "max_temperature": max_temperature },
        "thermal": { "max_stress": max_stress }
    })
}
