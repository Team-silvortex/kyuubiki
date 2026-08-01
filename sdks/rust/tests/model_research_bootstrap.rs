use kyuubiki_headless_sdk::{
    MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION, ModelResearchSdk,
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
