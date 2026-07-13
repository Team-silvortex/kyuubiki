use super::{
    Options, build_planned_probes, plan_payload, profile_cli_name, resolve_repo_path, select_probes,
};
use serde_json::json;
use std::path::PathBuf;

fn options() -> Options {
    Options {
        case_filter: None,
        execute: false,
        limit: None,
        manifest_path: PathBuf::from("manifest.json"),
        matrix_filter: None,
        output_slug_prefix: "plan".to_string(),
        output_format: "table".to_string(),
        plan_out: None,
        profile_filter: None,
        repeat: "1".to_string(),
        show_shapes: false,
        solver_preconditioner: "auto".to_string(),
        sync_to_remote: None,
    }
}

#[test]
fn profile_names_map_to_cli_tokens() {
    assert_eq!(profile_cli_name("four_hundred_k"), "400k");
    assert_eq!(profile_cli_name("five_hundred_k"), "500k");
    assert_eq!(profile_cli_name("medium"), "medium");
}

#[test]
fn selects_filtered_profile_matrix_cases() {
    let manifest = json!({
        "targets": [
            {
                "matrix": "mechanical-core",
                "profile": "five_hundred_k",
                "expected_cases": ["axial-bar-500k", "truss-roof-500k"]
            },
            {
                "matrix": "thermal-core",
                "profile": "five_hundred_k",
                "expected_cases": ["heat-plane-quad-500k"]
            }
        ]
    });
    let mut options = options();
    options.profile_filter = Some("500k".to_string());
    options.matrix_filter = Some("mechanical-core".to_string());
    options.case_filter = Some("axial".to_string());

    let probes = select_probes(&manifest, &options);

    assert_eq!(probes.len(), 1);
    assert_eq!(probes[0].profile_cli, "500k");
    assert_eq!(probes[0].case_id, "axial-bar-500k");
}

#[test]
fn planned_probes_include_output_slug_and_command() {
    let probes = vec![super::Probe {
        matrix: "thermal-structural".to_string(),
        profile_manifest: "five_hundred_k".to_string(),
        profile_cli: "500k".to_string(),
        case_id: "frame-2d-500k".to_string(),
    }];
    let options = options();

    let planned = build_planned_probes(PathBuf::from(".").as_path(), &probes, &options)
        .expect("plan should build without shapes");

    assert_eq!(planned.len(), 1);
    assert_eq!(
        planned[0]["output_slug"].as_str(),
        Some("plan-001-thermal-structural-500k-frame-2d-500k")
    );
    assert!(
        planned[0]["command"]
            .as_str()
            .is_some_and(|command| command.contains("benchmark-profile-remote"))
    );
    assert!(planned[0]["shape"].is_null());
}

#[test]
fn plan_payload_records_probe_count() {
    let payload = plan_payload(
        false,
        vec![json!({"case_id": "a"}), json!({"case_id": "b"})],
    );

    assert_eq!(payload["mode"].as_str(), Some("dry-run"));
    assert_eq!(payload["probe_count"].as_u64(), Some(2));
}

#[test]
fn plan_output_path_rejects_parent_traversal() {
    let root = PathBuf::from("/repo");
    let error = resolve_repo_path(root.as_path(), PathBuf::from("../plan.json").as_path())
        .expect_err("parent traversal should be rejected");

    assert!(error.contains("must not contain"));
}
