use kyuubiki_headless_sdk::{
    MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION, ModelCollaborationSession, ModelResearchSdk,
    ModelWorkflowProposal, build_bootstrapped_model_headless_plan,
    inspect_model_research_bootstrap,
};
use serde_json::Value;
use std::{fs, path::Path};

fn repository_bootstrap() -> (std::path::PathBuf, Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bootstrap = serde_json::from_str(
        &fs::read_to_string(root.join("docs/model-research-bootstrap.json"))
            .expect("model research bootstrap"),
    )
    .expect("bootstrap JSON");
    (root, bootstrap)
}

fn first_research_fixtures(
    root: &Path,
    bootstrap: &Value,
) -> (ModelCollaborationSession, ModelWorkflowProposal) {
    let first = &bootstrap["first_research"];
    let session = serde_json::from_str(
        &fs::read_to_string(root.join(first["session_fixture"].as_str().unwrap()))
            .expect("session fixture"),
    )
    .expect("session JSON");
    let proposal = serde_json::from_str(
        &fs::read_to_string(root.join(first["proposal_fixture"].as_str().unwrap()))
            .expect("proposal fixture"),
    )
    .expect("proposal JSON");
    (session, proposal)
}

#[test]
fn repository_bootstrap_is_ready_for_all_official_sdks() {
    let (root, bootstrap) = repository_bootstrap();
    for sdk in [
        ModelResearchSdk::Rust,
        ModelResearchSdk::Python,
        ModelResearchSdk::Elixir,
    ] {
        let report =
            inspect_model_research_bootstrap(&bootstrap, sdk, |path| root.join(path).is_file());
        assert!(report.ready_for_planning, "{:?}", report.blockers);
        assert_eq!(
            report.schema_version,
            MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION
        );
        assert_eq!(report.execution_authority, "none_preflight_only");
        assert!(report.missing_resources.is_empty());
        assert!(report.selected_surface.is_some());
    }
}

#[test]
fn shared_rust_readiness_fixture_matches_generated_report() {
    let (root, bootstrap) = repository_bootstrap();
    let report = inspect_model_research_bootstrap(&bootstrap, ModelResearchSdk::Rust, |path| {
        root.join(path).is_file()
    });
    let fixture: Value = serde_json::from_str(
        &fs::read_to_string(root.join("schemas/examples.model-research-readiness-report.json"))
            .expect("readiness fixture"),
    )
    .expect("readiness fixture JSON");
    let mut actual = serde_json::to_value(report).expect("serialize readiness report");
    actual.as_object_mut().expect("report object").insert(
        "$schema".into(),
        Value::String("./model-research-readiness-report.schema.json".into()),
    );
    assert_eq!(actual, fixture);
}

#[test]
fn missing_or_unsafe_resources_block_planning_without_execution_authority() {
    let (root, bootstrap) = repository_bootstrap();
    let missing = inspect_model_research_bootstrap(&bootstrap, ModelResearchSdk::Rust, |path| {
        path != "llms.txt" && root.join(path).is_file()
    });
    assert!(!missing.ready_for_planning);
    assert_eq!(missing.missing_resources, ["llms.txt"]);
    assert_eq!(missing.execution_authority, "none_preflight_only");

    let mut unsafe_bootstrap = bootstrap;
    unsafe_bootstrap["required_documents"][0]["path"] = Value::String("../secret".into());
    let unsafe_report =
        inspect_model_research_bootstrap(&unsafe_bootstrap, ModelResearchSdk::Rust, |path| {
            root.join(path).is_file()
        });
    assert!(!unsafe_report.ready_for_planning);
    assert!(
        unsafe_report
            .blockers
            .iter()
            .any(|blocker| blocker.contains("safe project-relative path"))
    );

    let mut authority_bootstrap = unsafe_bootstrap;
    authority_bootstrap["required_documents"][0]["path"] =
        Value::String("docs/model-research-onboarding.html".into());
    authority_bootstrap["preflight"]["execution_authority"] = Value::String("model_owned".into());
    let authority_report =
        inspect_model_research_bootstrap(&authority_bootstrap, ModelResearchSdk::Rust, |path| {
            root.join(path).is_file()
        });
    assert!(!authority_report.ready_for_planning);
    assert!(
        authority_report
            .blockers
            .iter()
            .any(|blocker| blocker.contains("none_preflight_only"))
    );
}

#[test]
fn bootstrapped_readiness_builds_first_headless_plan() {
    let (root, bootstrap) = repository_bootstrap();
    let readiness = inspect_model_research_bootstrap(&bootstrap, ModelResearchSdk::Rust, |path| {
        root.join(path).is_file()
    });
    let (session, proposal) = first_research_fixtures(&root, &bootstrap);
    let plan = build_bootstrapped_model_headless_plan(&readiness, &session, &proposal)
        .expect("bootstrapped plan");

    assert!(plan.ok);
    assert!(!plan.ready_without_confirmation);
    assert_eq!(plan.workflow_id, readiness.workflow_id);
}

#[test]
fn blocked_or_mismatched_readiness_never_builds_plan() {
    let (root, bootstrap) = repository_bootstrap();
    let mut readiness =
        inspect_model_research_bootstrap(&bootstrap, ModelResearchSdk::Rust, |path| {
            root.join(path).is_file()
        });
    let (mut session, proposal) = first_research_fixtures(&root, &bootstrap);
    readiness.ready_for_planning = false;
    assert!(build_bootstrapped_model_headless_plan(&readiness, &session, &proposal).is_err());

    readiness.ready_for_planning = true;
    session.workflow_id = "workflow.other".into();
    let error = build_bootstrapped_model_headless_plan(&readiness, &session, &proposal)
        .expect_err("workflow mismatch");
    assert!(error.to_string().contains("workflow_id does not match"));
}
