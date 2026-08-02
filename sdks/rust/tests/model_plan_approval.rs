use kyuubiki_headless_sdk::{
    HeadlessModelRisk, ModelCollaborationSession, ModelWorkflowProposal, build_model_headless_plan,
    build_model_plan_approval_request, compute_model_headless_plan_digest,
};
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn shared_plan_builds_digest_bound_approval_request() {
    let schemas = schemas();
    let session: ModelCollaborationSession =
        read_json(&schemas, "examples.model-collaboration-session.json");
    let proposal: ModelWorkflowProposal =
        read_json(&schemas, "examples.model-workflow-proposal.json");
    let plan = build_model_headless_plan(&session, &proposal).expect("plan");
    let request = build_model_plan_approval_request(&plan).expect("approval request");
    let actual = serde_json::to_value(request).expect("serialize request");
    let mut expected: Value = read_json(&schemas, "examples.model-plan-approval-request.json");
    expected
        .as_object_mut()
        .expect("fixture object")
        .remove("$schema");

    assert_eq!(actual, expected);
    assert_eq!(
        compute_model_headless_plan_digest(&plan).expect("digest"),
        "sha256:22e040653a1fc2274201a86f3ffaff67e896cedb5754e6fee01fb0528704d18d"
    );
}

#[test]
fn plan_digest_changes_with_nested_payload() {
    let schemas = schemas();
    let session: ModelCollaborationSession =
        read_json(&schemas, "examples.model-collaboration-session.json");
    let proposal: ModelWorkflowProposal =
        read_json(&schemas, "examples.model-workflow-proposal.json");
    let mut plan = build_model_headless_plan(&session, &proposal).expect("plan");
    let before = compute_model_headless_plan_digest(&plan).expect("digest");
    plan.steps[1].payload["input_artifacts"]["material_rows"]["rows"][0]["case_id"] =
        Value::String("changed".to_string());
    let after = compute_model_headless_plan_digest(&plan).expect("changed digest");
    assert_ne!(before, after);
}

#[test]
fn approval_request_rejects_inconsistent_gated_risk() {
    let schemas = schemas();
    let session: ModelCollaborationSession =
        read_json(&schemas, "examples.model-collaboration-session.json");
    let proposal: ModelWorkflowProposal =
        read_json(&schemas, "examples.model-workflow-proposal.json");
    let mut plan = build_model_headless_plan(&session, &proposal).expect("plan");
    plan.steps[1].risk = HeadlessModelRisk::Normal;

    let error = build_model_plan_approval_request(&plan).expect_err("risk consistency");
    assert!(error.to_string().contains("has invalid risk"));
}

fn schemas() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

fn read_json<T: serde::de::DeserializeOwned>(schemas: &std::path::Path, name: &str) -> T {
    serde_json::from_slice(&std::fs::read(schemas.join(name)).expect("read fixture"))
        .expect("parse fixture")
}
