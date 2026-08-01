use super::*;
use kyuubiki_project_bundle::normalize_project_bundle;
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str, steps: Value) -> (PathBuf, PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-project-automation-{}-{name}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("project.json");
    let bundle = root.join("project.kyuubiki");
    fs::write(
        &source,
        serde_json::to_vec_pretty(&json!({
            "project_schema_version": "kyuubiki.project/v2",
            "exported_at": "2026-08-01T00:00:00.000Z",
            "project": {
                "project_id": "project-native",
                "name": "Native automation",
                "inserted_at": "2026-08-01T00:00:00.000Z",
                "updated_at": "2026-08-01T00:00:00.000Z"
            },
            "models": [],
            "model_versions": [],
            "automation_presets": [{
                "presetId": "preset-native",
                "projectId": "project-native",
                "name": "Native preset",
                "updatedAt": "2026-08-01T00:00:00.000Z",
                "macro": { "id": "macro/native", "steps": steps }
            }]
        }))
        .expect("serialize fixture"),
    )
    .expect("write fixture");
    normalize_project_bundle(
        source.to_str().expect("source path"),
        bundle.to_str().expect("bundle path"),
    )
    .expect("normalize bundle");
    (root, bundle)
}

fn macro_fixture(name: &str, value: Value) -> (PathBuf, PathBuf) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-macro-{}-{name}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("macro fixture root");
    let path = root.join("automation.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&value).expect("serialize macro fixture"),
    )
    .expect("write macro fixture");
    (root, path)
}

#[test]
fn lists_and_renders_presets_from_native_bundle() {
    let (root, bundle) = fixture(
        "render",
        json!([{
            "action": "project_create",
            "payload": {
                "name": "{{payload.name}}",
                "description": "candidate {{state.round}}"
            }
        }]),
    );
    let path = bundle.to_str().expect("bundle path");

    let presets = list_project_automation_presets(path).expect("list presets");
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].macro_id.as_deref(), Some("macro/native"));

    let envelope = render_project_automation_preset(
        path,
        "Native preset",
        json!({ "name": "Alloy search" }),
        json!({ "round": 4 }),
    )
    .expect("render preset");
    assert_eq!(envelope.plan.steps[0].payload["name"], "Alloy search");
    assert_eq!(envelope.plan.steps[0].payload["description"], "candidate 4");
    assert_eq!(envelope.risk_summary.highest_risk, HeadlessRisk::Normal);

    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn dry_run_simulates_sensitive_browser_steps_without_side_effects() {
    let (root, bundle) = fixture(
        "dry-run",
        json!([{ "action": "snapshot", "payload": { "file": "result.png" } }]),
    );
    let report = run_project_automation_preset(
        bundle.to_str().expect("bundle path"),
        "preset-native",
        json!({}),
        json!({}),
        &AutomationRunOptions::default(),
    )
    .expect("dry run");

    assert_eq!(report.status, "simulated");
    assert_eq!(report.executed_step_count, 1);
    assert_eq!(report.steps[0].status, "simulated");
    assert!(report.steps[0].requires_confirmation);

    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn live_run_blocks_destructive_step_without_network_access() {
    let (root, bundle) = fixture(
        "blocked",
        json!([{
            "action": "project_delete",
            "payload": { "project_id": "project-native" }
        }]),
    );
    let report = run_project_automation_preset(
        bundle.to_str().expect("bundle path"),
        "preset-native",
        json!({}),
        json!({}),
        &AutomationRunOptions {
            execute: true,
            ..AutomationRunOptions::default()
        },
    )
    .expect("blocked run");

    assert_eq!(report.status, "blocked");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(
        report
            .blocked_by_confirmation
            .as_ref()
            .map(|step| step.index),
        Some(0)
    );

    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn live_service_health_uses_native_http_executor() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let port = listener.local_addr().expect("server address").port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..read]);
        assert!(request.starts_with("GET /api/health HTTP/1.1"));
        let body = r#"{"status":"ok","service":"test"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write response");
    });
    let (root, bundle) = fixture(
        "live-health",
        json!([{ "action": "service_health", "payload": {} }]),
    );
    let report = run_project_automation_preset(
        bundle.to_str().expect("bundle path"),
        "preset-native",
        json!({}),
        json!({}),
        &AutomationRunOptions {
            execute: true,
            api_base_url: Some(format!("http://127.0.0.1:{port}")),
            ..AutomationRunOptions::default()
        },
    )
    .expect("live run");
    server.join().expect("server thread");

    assert_eq!(report.status, "completed");
    assert_eq!(report.executed_step_count, 1);
    assert_eq!(report.steps[0].result["service"], "test");

    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn standalone_macro_pipeline_preserves_then_resolves_templates() {
    let (root, input) = macro_fixture(
        "pipeline",
        json!({
            "id": "macro/standalone",
            "steps": [{
                "action": "project_create",
                "payload": {
                    "name": "{{payload.name}}",
                    "description": "round {{state.round}}"
                }
            }]
        }),
    );
    let input = input.to_str().expect("macro path");

    let summary = inspect_macro_file(input).expect("inspect macro");
    assert_eq!(summary.step_count, 1);
    assert_eq!(summary.actions, vec!["project_create"]);
    assert!(validate_macro_file(input).expect("validate macro").ok);

    let normalized = root.join("normalized/macro.json");
    normalize_macro_file(input, normalized.to_str().expect("normalized path"))
        .expect("normalize macro");
    let normalized_value: Value =
        serde_json::from_slice(&fs::read(&normalized).expect("read normalized"))
            .expect("parse normalized");
    assert_eq!(
        normalized_value["steps"][0]["payload"]["name"],
        "{{payload.name}}"
    );

    let envelope = render_macro_file(
        input,
        json!({ "name": "Alloy search" }),
        json!({ "round": 9 }),
    )
    .expect("render macro");
    assert_eq!(envelope.source.kind, "macro_file");
    assert_eq!(envelope.plan.steps[0].payload["name"], "Alloy search");
    assert_eq!(envelope.plan.steps[0].payload["description"], "round 9");

    let report = run_macro_file(
        input,
        json!({ "name": "Alloy search" }),
        json!({ "round": 9 }),
        &AutomationRunOptions::default(),
    )
    .expect("dry-run macro");
    assert_eq!(report.status, "simulated");
    assert_eq!(report.executed_step_count, 1);

    fs::remove_dir_all(root).expect("clean macro fixture");
}

#[test]
fn standalone_macro_validation_reports_shape_and_contract_issues() {
    let (root, path) = macro_fixture(
        "invalid",
        json!({
            "steps": [
                { "action": "not_a_real_action", "payload": {} },
                { "payload": {} }
            ]
        }),
    );
    let report = validate_macro_file(path.to_str().expect("macro path")).expect("validation");

    assert!(!report.ok);
    assert_eq!(report.summary.step_count, 2);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue == "macro id is missing")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("unsupported action"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("missing action"))
    );

    fs::remove_dir_all(root).expect("clean macro fixture");
}

#[test]
fn automation_action_manifest_is_shared_with_native_macro_cli() {
    let actions = list_automation_action_capabilities();
    assert!(actions.iter().any(|entry| entry.action == "service_health"));
    assert!(actions.iter().any(|entry| entry.action == "project_create"));
    assert!(actions.iter().any(|entry| entry.action == "snapshot"));
}
