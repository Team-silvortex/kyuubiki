use std::fs;
use std::path::Path;

use crate::{
    Platform, build_embedded_runtime_manifest, build_launch_manifest, build_release_manifest,
    cross_platform_audit_report, default_remote_deployment_journal, default_remote_deployment_plan,
    embedded_runtime_report, expected_release_script_contents, installation_integrity_report,
    parse_agent_endpoints, parse_agent_manifest, parse_platform, remote_deployment_roadmap,
    unified_update_plan, unified_update_preview, workspace_root,
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
    assert!(manifest.contains("\"release_dir\": \".\""));
    assert!(manifest.contains("\"workspace\": \"../..\""));
}

#[test]
fn launch_manifest_uses_portable_entrypoints() {
    let macos_manifest = build_launch_manifest(Path::new("/tmp/workspace"), Platform::Macos);
    let windows_manifest = build_launch_manifest(Path::new("/tmp/workspace"), Platform::Windows);
    assert!(macos_manifest.contains("\"entrypoint\": \"./scripts/start.sh\""));
    assert!(windows_manifest.contains("\"entrypoint\": \"./scripts/start.cmd\""));
    assert!(windows_manifest.contains("\"shell\": \"cmd\""));
}

#[test]
fn release_scripts_prefer_embedded_node_runtime() {
    let macos_scripts = expected_release_script_contents(Platform::Macos);
    let start_script = macos_scripts
        .iter()
        .find(|(path, _)| path == "scripts/start.sh")
        .map(|(_, contents)| contents)
        .unwrap();
    assert!(start_script.contains("dist/macos/runtimes/macos/node/bin/node"));
    assert!(start_script.contains("NODE_BIN=\"node\""));

    let windows_scripts = expected_release_script_contents(Platform::Windows);
    let status_script = windows_scripts
        .iter()
        .find(|(path, _)| path == "scripts/status.cmd")
        .map(|(_, contents)| contents)
        .unwrap();
    assert!(status_script.contains("dist\\windows\\runtimes\\windows\\node\\node.exe"));
    assert!(status_script.contains("set NODE_BIN=node"));
}

#[test]
fn embedded_runtime_manifest_declares_self_host_payloads() {
    let root = workspace_root();
    let manifest = build_embedded_runtime_manifest(&root, Platform::Linux).unwrap();
    assert!(manifest.contains("\"schema_version\": \"kyuubiki.embedded-runtimes/v1\""));
    assert!(manifest.contains("\"id\": \"elixir-otp\""));
    assert!(manifest.contains("\"id\": \"node\""));
    assert!(manifest.contains("\"required_for_self_host\": true"));
    assert!(manifest.contains("\"source_contract\": \"config/toolchains.json#/elixir\""));
}

#[test]
fn embedded_runtime_report_renders_contract_summary() {
    let report = embedded_runtime_report().unwrap();
    let rendered = report.render();
    assert!(rendered.contains("kyuubiki embedded runtimes"));
    assert!(rendered.contains("elixir-otp"));
    assert!(rendered.contains("node"));
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
    assert_eq!(plan.target_version, "1.12.0");
}

#[test]
fn unified_update_preview_reports_noop_for_current_channel() {
    let preview = unified_update_preview(None).unwrap();
    assert_eq!(preview.channel, "stable");
    assert_eq!(preview.target_version, "1.12.0");
    assert_eq!(preview.overall_status, "noop");
}

#[test]
fn cross_platform_audit_runs() {
    let report = cross_platform_audit_report();
    assert_eq!(report.checked_platforms.len(), 3);
}

#[test]
fn installation_integrity_reports_component_protocol() {
    let report = installation_integrity_report();
    assert_eq!(
        report.component_protocol.schema_version,
        "kyuubiki.component-integrity/v1"
    );
    assert_eq!(
        report.component_protocol.covered_required_path_count,
        report.component_protocol.required_path_count
    );
    assert!(
        report.version_checks.iter().all(|check| check.ok),
        "version checks should all align after brand metadata is covered"
    );
    assert!(
        report
            .component_protocol
            .components
            .iter()
            .any(|component| component.id == "installer.core")
    );
    assert!(
        report
            .component_protocol
            .components
            .iter()
            .any(|component| component.id == "runtime.state")
    );
    assert!(
        report
            .component_protocol
            .issues
            .iter()
            .all(|issue| !issue.message.contains("outside owned paths"))
    );
    assert!(
        report
            .render()
            .contains("component_protocol: kyuubiki.component-integrity/v1")
    );
    assert!(report.render().contains("required_path_coverage:"));
}

#[test]
fn remote_deployment_roadmap_marks_ssh_wrapper_as_pilot() {
    let roadmap = remote_deployment_roadmap();
    assert_eq!(
        roadmap.schema_version,
        "kyuubiki.remote-deployment-roadmap/v1"
    );
    assert!(roadmap.current_maturity.contains("pilot"));
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "deployment-plan" && stage.status == "started")
    );
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "remote-journal" && stage.status == "started")
    );
    assert!(roadmap.render().contains("Policy-bounded SSH transport"));
}

#[test]
fn remote_deployment_plan_has_retry_safe_step_contract() {
    let plan = default_remote_deployment_plan();
    assert_eq!(plan.schema_version, "kyuubiki.remote-deployment-plan/v1");
    assert!(plan.steps.iter().any(|step| step.id == "policy-check"));
    assert!(plan.steps.iter().any(|step| step.id == "verify-integrity"));
    assert!(plan.steps.iter().any(|step| step.id == "health-check"));
    assert!(
        plan.steps
            .iter()
            .all(|step| !step.idempotency_key.is_empty() && !step.failure_class.is_empty())
    );
    assert!(plan.render().contains("rollback_hint:"));
}

#[test]
fn remote_deployment_journal_matches_plan_steps() {
    let plan = default_remote_deployment_plan();
    let journal = default_remote_deployment_journal();
    assert_eq!(
        journal.schema_version,
        "kyuubiki.remote-deployment-journal/v1"
    );
    assert_eq!(journal.records.len(), plan.steps.len());
    assert!(
        journal
            .records
            .iter()
            .all(|record| record.status == "pending")
    );
    assert!(journal.records.iter().all(|record| {
        record
            .local_record_path
            .starts_with(".kyuubiki/remote-journal/")
    }));
    assert!(
        journal
            .render()
            .contains("remote deployment journal preview")
    );
}
