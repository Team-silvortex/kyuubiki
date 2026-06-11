use std::fs;
use std::path::Path;

use crate::{
    Platform, build_release_manifest, parse_agent_endpoints, parse_agent_manifest, parse_platform,
    unified_update_plan, unified_update_preview,
};

#[test]
fn parses_unknown_platform_to_current() {
    assert_eq!(
        parse_platform(Some("unknown".to_string())),
        Platform::current()
    );
}

#[test]
fn release_manifest_contains_expected_schema() {
    let manifest = build_release_manifest(
        Path::new("/tmp/workspace"),
        Path::new("/tmp/dist/macos"),
        Platform::Macos,
    );
    assert!(manifest.contains("\"schema_version\": \"kyuubiki.release/v1\""));
    assert!(manifest.contains("\"platform\": \"macos\""));
}

#[test]
fn parses_agent_endpoint_list() {
    let parsed = parse_agent_endpoints("127.0.0.1:5001,solver.local:5002").unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].0, "127.0.0.1");
    assert_eq!(parsed[0].1, 5001);
    assert_eq!(parsed[1].0, "solver.local");
    assert_eq!(parsed[1].1, 5002);
}

#[test]
fn rejects_empty_agent_endpoint_list() {
    assert!(parse_agent_endpoints(" , ").is_err());
}

#[test]
fn parses_manifest_agents() {
    let dir = std::env::temp_dir();
    let path = dir.join("kyuubiki-agent-manifest.json");
    fs::write(
        &path,
        r#"{
          "schema_version": "kyuubiki.agent-manifest/v1",
          "agents": [
            {"id": "alpha", "host": "127.0.0.1", "port": 5001},
            {"id": "beta", "host": "solver.local", "port": 5002}
          ]
        }"#,
    )
    .unwrap();

    let parsed = parse_agent_manifest(&path).unwrap();
    assert_eq!(
        parsed,
        vec![
            ("127.0.0.1".to_string(), 5001),
            ("solver.local".to_string(), 5002)
        ]
    );

    let _ = fs::remove_file(path);
}

#[test]
fn unified_update_plan_uses_default_channel() {
    let plan = unified_update_plan(None).unwrap();
    assert_eq!(plan.target_channel, "stable");
    assert_eq!(plan.target_version, "1.5.0");
}

#[test]
fn unified_update_preview_reports_noop_for_current_channel() {
    let preview = unified_update_preview(None).unwrap();
    assert_eq!(preview.channel, "stable");
    assert_eq!(preview.target_version, "1.5.0");
    assert_eq!(preview.overall_status, "noop");
}
