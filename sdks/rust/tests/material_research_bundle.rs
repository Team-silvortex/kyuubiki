use kyuubiki_headless_sdk::{MaterialResearchBundle, validate_material_research_bundle};
use serde_json::Value;

fn fixture() -> MaterialResearchBundle {
    serde_json::from_str(include_str!(
        "../../../schemas/examples.material-research-bundle.json"
    ))
    .expect("fixture should decode")
}

#[test]
fn validates_shared_material_research_bundle_fixture() {
    let bundle = fixture();

    validate_material_research_bundle(&bundle).expect("fixture should validate");

    assert_eq!(
        bundle.schema_version,
        "kyuubiki.material-research-bundle/v1"
    );
    assert_eq!(bundle.study, "heat-spreader");
    assert_eq!(
        bundle.summary.winner_candidate_id,
        "pyrolytic_graphite_in_plane"
    );
}

#[test]
fn rejects_bad_retained_artifact_schema() {
    let mut bundle = fixture();
    bundle.chain["schema_version"] = Value::String("wrong".into());

    let error = validate_material_research_bundle(&bundle)
        .expect_err("bad chain schema should fail")
        .to_string();

    assert!(error.contains("chain.schema_version"));
}

#[test]
fn rejects_bad_checksum_shape() {
    let mut bundle = fixture();
    bundle.artifact_checksums.chain_sha256 = "not-a-digest".into();

    let error = bundle
        .validate()
        .expect_err("bad checksum shape should fail")
        .to_string();

    assert!(error.contains("chain_sha256"));
}

#[test]
fn rejects_summary_plan_decision_mismatch() {
    let mut bundle = fixture();
    bundle.next_round_execution_plan["decision"] = Value::String("repair_validation".into());

    let error = bundle
        .validate()
        .expect_err("summary and plan decision mismatch should fail")
        .to_string();

    assert!(error.contains("next_round_execution_plan.decision"));
}

#[test]
fn rejects_mock_execution_authority() {
    let mut bundle = fixture();
    bundle.execution_trace["authority"]["initial"]["mock_execution"] = Value::Bool(true);

    let error = bundle
        .validate()
        .expect_err("mock execution must fail")
        .to_string();

    assert!(error.contains("mock_execution"));
}

#[test]
fn rejects_inconsistent_research_ranking() {
    let mut bundle = fixture();
    bundle.research_evidence["candidate_count"] = Value::from(99);

    let error = bundle
        .validate()
        .expect_err("candidate count drift must fail")
        .to_string();

    assert!(error.contains("candidate_count"));
}

#[test]
fn rejects_validation_gate_drift() {
    let mut bundle = fixture();
    bundle.validation_evidence["violated_quality_gate_ids"] = Value::Array(Vec::new());

    let error = bundle
        .validate()
        .expect_err("validation gate drift must fail")
        .to_string();

    assert!(error.contains("violated_quality_gate_ids"));
}

#[test]
fn rejects_missing_screening_boundary() {
    let mut bundle = fixture();
    bundle.validation_evidence["validation_readiness"]["blocking_reasons"] =
        serde_json::json!(["violated_quality_gates"]);

    let error = bundle
        .validate()
        .expect_err("external validation boundary must fail")
        .to_string();

    assert!(error.contains("external_validation_required"));
}

#[test]
fn decoding_requires_retained_evidence() {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../schemas/examples.material-research-bundle.json"
    ))
    .expect("fixture should decode as json");
    value
        .as_object_mut()
        .expect("fixture must be an object")
        .remove("research_evidence");

    let error = serde_json::from_value::<MaterialResearchBundle>(value)
        .expect_err("missing research evidence must not decode")
        .to_string();

    assert!(error.contains("research_evidence"));
}
