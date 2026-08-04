use crate::{build_materialized_candidate_report, build_materialized_candidate_steps};
use serde_json::json;

#[test]
fn heat_spreader_materialization_builds_real_solver_steps_and_ranked_report() {
    let plan = heat_spreader_plan();
    let steps = build_materialized_candidate_steps(&plan).expect("materialized steps");

    assert_eq!(steps.len(), 2);
    assert!(
        steps
            .iter()
            .all(|step| step.action == "solve_heat_plane_quad_2d")
    );
    assert_eq!(
        steps[0].payload["research"]["candidate_id"],
        "pyrolytic_graphite_in_plane__generate_conservative_neighbor"
    );
    assert_eq!(
        steps[0].payload["model"]["elements"][0]["conductivity"],
        1575.0
    );

    let report = build_materialized_candidate_report(
        &plan,
        &[
            json!({ "max_temperature": 68.0, "max_heat_flux": 2.1e6 }),
            json!({ "max_temperature": 74.0, "max_heat_flux": 0.3e6 }),
        ],
    )
    .expect("materialized report");

    assert_eq!(
        report["schema_version"],
        "kyuubiki.materialized-heat-spreader-report/v1"
    );
    assert_eq!(report["candidates"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        report["winner_candidate_id"],
        "pyrolytic_graphite_in_plane__generate_conservative_neighbor"
    );
}

#[test]
fn materialized_rerun_rejects_mixed_studies_before_dispatch() {
    let mut plan = heat_spreader_plan();
    plan["materialized_candidates"][1]["study"] = json!("material_composite_thermo_electric_panel");

    let error = build_materialized_candidate_steps(&plan).expect_err("mixed studies must fail");

    assert!(error.contains("does not match"));
}

#[test]
fn heat_spreader_materialization_rejects_non_rerun_status() {
    let mut plan = heat_spreader_plan();
    plan["materialized_candidates"][0]["status"] = json!("draft_requires_review");

    let error = build_materialized_candidate_steps(&plan).expect_err("status must fail closed");

    assert!(error.contains("incompatible status"));
}

fn heat_spreader_plan() -> serde_json::Value {
    json!({
        "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
        "status": "ready_for_solver_rerun",
        "materialized_candidate_count": 2,
        "materialized_candidates": [
            heat_candidate("pyrolytic_graphite_in_plane", "draft-pyro"),
            heat_candidate("aluminum_6061", "draft-aluminum")
        ]
    })
}

fn heat_candidate(source_candidate_id: &str, source_draft_id: &str) -> serde_json::Value {
    json!({
        "schema_version": "kyuubiki.materialized-candidate-spec/v1",
        "candidate_id": format!("{source_candidate_id}__generate_conservative_neighbor"),
        "source_candidate_id": source_candidate_id,
        "source_draft_id": source_draft_id,
        "strategy": "generate_conservative_neighbor",
        "study": "material_heat_spreader_screening",
        "required_result_schema": "kyuubiki.material-result-payload/v1",
        "status": "requires_solver_rerun"
    })
}
