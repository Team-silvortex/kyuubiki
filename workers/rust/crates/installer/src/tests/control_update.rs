use std::fs;

use crate::{
    Platform, cross_platform_audit_report, parse_agent_endpoints, parse_agent_manifest,
    parse_platform, unified_update_plan, unified_update_preview,
};

#[test]
fn parses_unknown_platform_to_current() {
    assert_eq!(
        parse_platform(Some("unknown".to_string())),
        Platform::current()
    );
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
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = dir.join(format!("kyuubiki-agent-manifest-{nonce}.json"));
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
    assert_eq!(plan.target_version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn unified_update_preview_reports_noop_for_current_channel() {
    let preview = unified_update_preview(None).unwrap();
    assert_eq!(preview.channel, "stable");
    assert_eq!(preview.target_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(preview.overall_status, "noop");
}

#[test]
fn cross_platform_audit_runs() {
    let report = cross_platform_audit_report();
    assert_eq!(report.checked_platforms.len(), 3);
}
