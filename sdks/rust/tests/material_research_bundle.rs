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
