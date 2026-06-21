use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

mod certificates;
mod certificates_openssl;
mod certificates_store;
mod certificates_types;
mod diagnostics;
mod env_panel;
mod remote;
mod remote_certificates;
mod remote_exec;
mod remote_nodes;
mod runtime_logs;

use certificates::{
    certificate_authority_policy, initialize_certificate_authority, issue_node_certificate,
    revoke_node_certificate, write_certificate_authority_policy,
};
use certificates_types::{
    IssueNodeCertificatePayload, RevokeNodeCertificatePayload,
    WriteCertificateAuthorityPolicyPayload,
};
use diagnostics::{
    doctor_report, installation_integrity_report, latest_applied_update_record,
    latest_downloaded_update_record, latest_staged_update_record, regression_gate_report,
    unified_update_plan, unified_update_preview, update_source_config,
};
use env_panel::{WriteEnvPayload, read_env_file, write_env_file};
use kyuubiki_desktop_runtime::{
    ServiceMode, ServiceStatusSummary, append_desktop_audit_line as desktop_append_audit_line,
    read_global_language_preference as desktop_read_global_language_preference,
    service_restart as desktop_service_restart, service_start as desktop_service_start,
    service_status as desktop_service_status, service_stop as desktop_service_stop,
    summarize_service_status as desktop_summarize_service_status,
    write_global_language_preference as desktop_write_global_language_preference,
};
use kyuubiki_installer::{
    apply_downloaded_update as installer_apply_downloaded_update,
    download_update as installer_download_update, export_launch_config,
    init_env as installer_init_env, parse_platform, prepare_layout as installer_prepare_layout,
    prepare_staged_update as installer_prepare_staged_update,
    repair_installation as installer_repair_installation, stage_release as installer_stage_release,
    validate_env_file, workspace_root,
    write_update_source_config as installer_write_update_source_config,
};
use remote::{
    RemoteAgentPayload, RemoteBootstrapPayload, WriteRemoteDeployPolicyPayload, probe_remote_node,
    remote_bootstrap, remote_deploy_policy, remote_start_agent, write_remote_deploy_policy,
};
use remote_nodes::{
    WriteRemoteNodeRegistryPayload, remote_node_registry, write_remote_node_registry,
};
use runtime_logs::{read_runtime_log, start_log_stream, stop_log_stream};
use serde::Serialize;
use serde_json::json;

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleasePayload {
    platform: String,
    target_dir: Option<String>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformPayload {
    platform: String,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPreferencesInputPayload {
    language: String,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildPayload {
    bundle_mode: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallerGuardedMutationPayload {
    action: String,
    channel: Option<String>,
    catalog_path: Option<String>,
    artifact_root: Option<String>,
    download_dir: Option<String>,
    mode: Option<String>,
    force: Option<bool>,
    platform: Option<String>,
    target_dir: Option<String>,
    bundle_mode: Option<String>,
    env_payload: Option<WriteEnvPayload>,
    remote_bootstrap: Option<RemoteBootstrapPayload>,
    remote_agent: Option<RemoteAgentPayload>,
    remote_policy: Option<WriteRemoteDeployPolicyPayload>,
    remote_nodes: Option<WriteRemoteNodeRegistryPayload>,
    certificate_policy: Option<WriteCertificateAuthorityPolicyPayload>,
    certificate_issue: Option<IssueNodeCertificatePayload>,
    certificate_revoke: Option<RevokeNodeCertificatePayload>,
}

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
    summary: ServiceStatusSummary,
}
#[derive(Serialize)]
struct DesktopPreferencesPayload {
    language: String,
}
const INSTALLER_GUARDED_MUTATION_AUDIT_FILE: &str = "installer-guarded-mutations.jsonl";

fn audit_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn append_installer_guarded_mutation_audit(
    payload: &InstallerGuardedMutationPayload,
    status: &str,
    detail: &str,
) {
    let record = json!({
        "ts": audit_timestamp(),
        "action": payload.action,
        "mode": payload.mode,
        "platform": payload.platform,
        "channel": payload.channel,
        "catalogPath": payload.catalog_path,
        "bundleMode": payload.bundle_mode,
        "status": status,
        "detail": detail,
    });
    let _ = desktop_append_audit_line(INSTALLER_GUARDED_MUTATION_AUDIT_FILE, &record.to_string());
}

#[tauri::command]
fn export_launch(payload: PlatformPayload) -> Result<String, String> {
    Ok(export_launch_config(parse_platform(Some(payload.platform))))
}

#[tauri::command]
fn stage_release(payload: ReleasePayload) -> Result<String, String> {
    let target_dir = payload
        .target_dir
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    installer_stage_release(parse_platform(Some(payload.platform)), target_dir)
}

#[tauri::command]
fn service_status() -> Result<ServiceStatusPayload, String> {
    let rendered = desktop_service_status()?;
    Ok(ServiceStatusPayload {
        summary: desktop_summarize_service_status(&rendered),
        rendered,
    })
}

#[tauri::command]
fn get_global_language_preference() -> DesktopPreferencesPayload {
    DesktopPreferencesPayload {
        language: desktop_read_global_language_preference().unwrap_or_else(|| "en".to_string()),
    }
}

#[tauri::command]
fn set_global_language_preference(
    payload: DesktopPreferencesInputPayload,
) -> Result<DesktopPreferencesPayload, String> {
    Ok(DesktopPreferencesPayload {
        language: desktop_write_global_language_preference(&payload.language)?,
    })
}

fn parse_service_mode(mode: Option<&str>) -> ServiceMode {
    match mode {
        Some("local") => ServiceMode::Local,
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        _ => ServiceMode::Default,
    }
}

#[tauri::command]
fn build_installer_bundle(payload: BuildPayload) -> Result<String, String> {
    let installer_gui_dir = workspace_root().join("apps").join("installer-gui");
    let bundle_mode = payload
        .bundle_mode
        .unwrap_or_else(|| "debug-check".to_string());
    let extra_args: Vec<&str> = match bundle_mode.as_str() {
        "release-bundle" => vec!["run", "tauri:build"],
        "release-no-bundle" => vec!["run", "tauri:build", "--", "--no-bundle"],
        "debug-check" => vec!["run", "tauri:build", "--", "--debug", "--no-bundle"],
        other => return Err(format!("unknown build mode: {other}")),
    };

    let output = Command::new("npm")
        .args(&extra_args)
        .current_dir(&installer_gui_dir)
        .output()
        .map_err(|error| format!("failed to run installer build: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            format!("installer build completed ({bundle_mode})")
        } else {
            stdout
        })
    } else {
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(if detail.is_empty() {
            "installer build failed".to_string()
        } else {
            detail
        })
    }
}

#[tauri::command]
fn guarded_mutation_action(payload: InstallerGuardedMutationPayload) -> Result<String, String> {
    let result = match payload.action.as_str() {
        "validate_env" => validate_env_file(),
        "init_env" => installer_init_env(payload.force.unwrap_or(false)),
        "prepare_layout" => installer_prepare_layout(),
        "repair_installation" => installer_repair_installation(),
        "bootstrap" => {
            installer_prepare_layout()?;
            installer_init_env(false)?;
            validate_env_file()
        }
        "write_env_file" => {
            let env = payload
                .env_payload
                .clone()
                .ok_or_else(|| "env payload is required".to_string())?;
            write_env_file(env)
        }
        "service_start" => desktop_service_start(parse_service_mode(payload.mode.as_deref())),
        "service_restart" => desktop_service_restart(parse_service_mode(payload.mode.as_deref())),
        "service_stop" => desktop_service_stop(),
        "remote_bootstrap" => {
            let remote = payload
                .remote_bootstrap
                .clone()
                .ok_or_else(|| "remote bootstrap payload is required".to_string())?;
            remote_bootstrap(remote)
        }
        "remote_start_agent" => {
            let remote = payload
                .remote_agent
                .clone()
                .ok_or_else(|| "remote agent payload is required".to_string())?;
            remote_start_agent(remote)
        }
        "write_remote_policy" => {
            let remote_policy = payload
                .remote_policy
                .clone()
                .ok_or_else(|| "remote policy payload is required".to_string())?;
            write_remote_deploy_policy(remote_policy)
        }
        "write_remote_nodes" => {
            let remote_nodes = payload
                .remote_nodes
                .clone()
                .ok_or_else(|| "remote node registry payload is required".to_string())?;
            write_remote_node_registry(remote_nodes)
        }
        "write_certificate_policy" => {
            let certificate_policy = payload
                .certificate_policy
                .clone()
                .ok_or_else(|| "certificate policy payload is required".to_string())?;
            write_certificate_authority_policy(certificate_policy)
        }
        "initialize_certificate_authority" => initialize_certificate_authority(),
        "issue_node_certificate" => {
            let certificate_issue = payload
                .certificate_issue
                .clone()
                .ok_or_else(|| "certificate issue payload is required".to_string())?;
            issue_node_certificate(certificate_issue)
        }
        "revoke_node_certificate" => {
            let certificate_revoke = payload
                .certificate_revoke
                .clone()
                .ok_or_else(|| "certificate revoke payload is required".to_string())?;
            revoke_node_certificate(certificate_revoke)
        }
        "probe_remote_node" => {
            let remote = payload
                .remote_bootstrap
                .clone()
                .ok_or_else(|| "remote bootstrap payload is required".to_string())?;
            probe_remote_node(remote)
        }
        "stage_release" => {
            let platform = payload
                .platform
                .clone()
                .ok_or_else(|| "platform is required".to_string())?;
            stage_release(ReleasePayload {
                platform,
                target_dir: payload.target_dir.clone(),
            })
        }
        "prepare_staged_update" => {
            let platform = payload
                .platform
                .clone()
                .ok_or_else(|| "platform is required".to_string())?;
            installer_prepare_staged_update(
                payload.channel.clone(),
                parse_platform(Some(platform)),
                payload.target_dir.clone().map(PathBuf::from),
            )
            .map(|report| report.render())
        }
        "write_update_source_config" => installer_write_update_source_config(
            payload.catalog_path.clone().unwrap_or_default(),
            payload.artifact_root.clone().unwrap_or_default(),
            payload.download_dir.clone().unwrap_or_default(),
        ),
        "download_update" => {
            let platform = payload
                .platform
                .clone()
                .ok_or_else(|| "platform is required".to_string())?;
            installer_download_update(payload.channel.clone(), parse_platform(Some(platform)))
                .map(|record| record.render())
        }
        "apply_downloaded_update" => {
            installer_apply_downloaded_update().map(|record| record.render())
        }
        "build_installer_bundle" => build_installer_bundle(BuildPayload {
            bundle_mode: payload.bundle_mode.clone(),
        }),
        other => Err(format!("unsupported guarded installer action: {other}")),
    };

    match &result {
        Ok(detail) => append_installer_guarded_mutation_audit(&payload, "ok", detail),
        Err(detail) => append_installer_guarded_mutation_audit(&payload, "failed", detail),
    }

    result
}
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            doctor_report,
            installation_integrity_report,
            regression_gate_report,
            latest_applied_update_record,
            latest_downloaded_update_record,
            latest_staged_update_record,
            update_source_config,
            unified_update_plan,
            unified_update_preview,
            export_launch,
            read_env_file,
            service_status,
            get_global_language_preference,
            set_global_language_preference,
            read_runtime_log,
            start_log_stream,
            stop_log_stream,
            certificate_authority_policy,
            remote_deploy_policy,
            remote_node_registry,
            guarded_mutation_action
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki installer gui");
}
