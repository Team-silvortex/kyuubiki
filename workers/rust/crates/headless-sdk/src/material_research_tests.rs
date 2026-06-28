use crate::{MaterialResearchReport, build_heat_spreader_screening_report};
use serde_json::json;

#[test]
fn heat_spreader_report_ranks_by_result_and_material_metrics() {
    let report = build_heat_spreader_screening_report(&[
        json!({ "result": { "max_temperature": 82.0, "max_heat_flux": 900.0 } }),
        json!({ "result": { "result": { "max_temperature": 64.0, "max_heat_flux": 1400.0 } } }),
        json!({ "max_temperature": 58.0, "max_heat_flux": 1800.0 }),
    ])
    .expect("report should build");

    assert_eq!(
        report.schema_version,
        "kyuubiki.material-research-report/v1"
    );
    assert_eq!(report.candidates.len(), 3);
    assert_eq!(
        report.winner_candidate_id.as_deref(),
        Some("pyrolytic_graphite_in_plane")
    );
    assert_eq!(
        report.optimization.id,
        "material.heat_spreader_screening.optimization.v1"
    );
    assert_eq!(
        report.reliability.schema_version,
        "kyuubiki.material-reliability-envelope/v1"
    );
    assert_eq!(report.reliability.posture, "screening_only");
    assert_eq!(report.reliability.evidence_refs.len(), 3);
    assert!(
        report
            .reliability
            .model_assumptions
            .iter()
            .any(|assumption| assumption.id == "model.material")
    );
    assert!(
        report
            .reliability
            .quality_gates
            .iter()
            .any(|gate| gate.id == "gate.result_completeness" && gate.status == "pass")
    );
    assert!(
        report
            .optimization
            .score_formula
            .contains("peak_temperature_c:min")
    );
    assert_eq!(report.candidates[0].rank, 1);
    assert!(report.candidates[0].score > report.candidates[1].score);
    assert_eq!(report.candidates[0].optimization_terms.len(), 3);
    assert_eq!(
        report.candidates[0].material_card_id,
        "kyuubiki.material_card.pyrolytic_graphite_in_plane.v1"
    );
    assert_eq!(report.candidates[0].material_card_confidence, "low");
    assert!(
        report.candidates[0]
            .optimization_terms
            .iter()
            .any(|term| term.metric_id == "areal_mass_kg_m2" && term.weighted_score > 0.0)
    );
    assert_eq!(report.candidates[2].candidate_id, "aluminum_6061");
}

#[test]
fn heat_spreader_report_keeps_missing_metric_warnings_visible() {
    let report = build_heat_spreader_screening_report(&[
        json!({ "result": { "kind": "simulated_result" } }),
        json!({ "result": { "max_temperature": 64.0 } }),
        json!({ "result": { "max_temperature": 58.0, "max_heat_flux": 1800.0 } }),
    ])
    .expect("report should tolerate incomplete early results");

    assert!(warning_count(&report) >= 3);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("aluminum_6061 is missing peak_temperature_c"))
    );
    assert!(
        report
            .reliability
            .quality_gates
            .iter()
            .any(|gate| gate.id == "gate.result_completeness" && gate.status == "violate")
    );
}

#[test]
fn heat_spreader_report_rejects_candidate_result_count_mismatch() {
    let error =
        build_heat_spreader_screening_report(&[]).expect_err("mismatched result count should fail");
    assert!(error.contains("expects 3 result payloads"));
}

fn warning_count(report: &MaterialResearchReport) -> usize {
    report.warnings.len()
}
