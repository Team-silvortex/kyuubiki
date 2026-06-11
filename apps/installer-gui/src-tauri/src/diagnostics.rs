use kyuubiki_installer::{
    doctor_report as build_doctor_report,
    installation_integrity_report as build_installation_integrity_report,
    unified_update_plan as build_unified_update_plan,
    unified_update_preview as build_unified_update_preview,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct DoctorReportPayload {
    platform: String,
    workspace: String,
    checks: Vec<DoctorCheckPayload>,
    rendered: String,
}

#[derive(Serialize)]
struct DoctorCheckPayload {
    label: String,
    ok: bool,
}

#[derive(Serialize)]
pub struct InstallationIntegrityPayload {
    schema_version: String,
    platform: String,
    workspace: String,
    current_version: String,
    contract_rules: Vec<IntegrityContractRulePayload>,
    layout: Vec<InstallationIntegrityEntryPayload>,
    version_checks: Vec<VersionAlignmentPayload>,
    residues: Vec<ResidueCandidatePayload>,
    issues: Vec<String>,
    rendered: String,
}

#[derive(Serialize)]
pub struct UnifiedUpdatePlanPayload {
    schema_version: String,
    workspace: String,
    current_version: String,
    target_channel: String,
    target_tag: String,
    target_version: String,
    update_state: String,
    summary: String,
    contract_rules: Vec<IntegrityContractRulePayload>,
    artifacts: Vec<UnifiedUpdateArtifactPayload>,
    rendered: String,
}

#[derive(Serialize)]
pub struct UnifiedUpdatePreviewPayload {
    schema_version: String,
    channel: String,
    target_version: String,
    overall_status: String,
    blocking_issues: usize,
    removable_residue: usize,
    steps: Vec<UnifiedUpdatePreviewStepPayload>,
    rendered: String,
}

#[derive(Serialize)]
struct IntegrityContractRulePayload {
    category: String,
    label: String,
    value: String,
    editable: bool,
    description: String,
}

#[derive(Serialize)]
struct InstallationIntegrityEntryPayload {
    label: String,
    relative_path: String,
    required: bool,
    present: bool,
    size_bytes: u64,
}

#[derive(Serialize)]
struct VersionAlignmentPayload {
    label: String,
    expected: String,
    actual: String,
    ok: bool,
}

#[derive(Serialize)]
struct ResidueCandidatePayload {
    relative_path: String,
    reason: String,
    removable: bool,
}

#[derive(Serialize)]
struct UnifiedUpdateArtifactPayload {
    product: String,
    platform: String,
    kind: String,
    path: String,
    exists: bool,
}

#[derive(Serialize)]
struct UnifiedUpdatePreviewStepPayload {
    label: String,
    status: String,
    detail: String,
}

#[tauri::command]
pub fn doctor_report() -> Result<DoctorReportPayload, String> {
    let report = build_doctor_report();
    Ok(DoctorReportPayload {
        rendered: report.render(),
        platform: report.platform,
        workspace: report.workspace,
        checks: report
            .checks
            .into_iter()
            .map(|check| DoctorCheckPayload {
                label: check.label,
                ok: check.ok,
            })
            .collect(),
    })
}

#[tauri::command]
pub fn installation_integrity_report() -> Result<InstallationIntegrityPayload, String> {
    let report = build_installation_integrity_report();
    let rendered = report.render();
    Ok(InstallationIntegrityPayload {
        schema_version: report.schema_version,
        platform: report.platform,
        workspace: report.workspace,
        current_version: report.current_version,
        rendered,
        contract_rules: report
            .contract_rules
            .into_iter()
            .map(|rule| IntegrityContractRulePayload {
                category: rule.category,
                label: rule.label,
                value: rule.value,
                editable: rule.editable,
                description: rule.description,
            })
            .collect(),
        layout: report
            .layout
            .into_iter()
            .map(|entry| InstallationIntegrityEntryPayload {
                label: entry.label,
                relative_path: entry.relative_path,
                required: entry.required,
                present: entry.present,
                size_bytes: entry.size_bytes,
            })
            .collect(),
        version_checks: report
            .version_checks
            .into_iter()
            .map(|check| VersionAlignmentPayload {
                label: check.label,
                expected: check.expected,
                actual: check.actual,
                ok: check.ok,
            })
            .collect(),
        residues: report
            .residues
            .into_iter()
            .map(|residue| ResidueCandidatePayload {
                relative_path: residue.relative_path,
                reason: residue.reason,
                removable: residue.removable,
            })
            .collect(),
        issues: report.issues,
    })
}

#[tauri::command]
pub fn unified_update_plan(channel: Option<String>) -> Result<UnifiedUpdatePlanPayload, String> {
    let report = build_unified_update_plan(channel)?;
    let rendered = report.render();
    Ok(UnifiedUpdatePlanPayload {
        schema_version: report.schema_version,
        workspace: report.workspace,
        current_version: report.current_version,
        target_channel: report.target_channel,
        target_tag: report.target_tag,
        target_version: report.target_version,
        update_state: report.update_state,
        summary: report.summary,
        rendered,
        contract_rules: report
            .contract_rules
            .into_iter()
            .map(|rule| IntegrityContractRulePayload {
                category: rule.category,
                label: rule.label,
                value: rule.value,
                editable: rule.editable,
                description: rule.description,
            })
            .collect(),
        artifacts: report
            .artifacts
            .into_iter()
            .map(|artifact| UnifiedUpdateArtifactPayload {
                product: artifact.product,
                platform: artifact.platform,
                kind: artifact.kind,
                path: artifact.path,
                exists: artifact.exists,
            })
            .collect(),
    })
}

#[tauri::command]
pub fn unified_update_preview(
    channel: Option<String>,
) -> Result<UnifiedUpdatePreviewPayload, String> {
    let report = build_unified_update_preview(channel)?;
    let rendered = report.render();
    Ok(UnifiedUpdatePreviewPayload {
        schema_version: report.schema_version,
        channel: report.channel,
        target_version: report.target_version,
        overall_status: report.overall_status,
        blocking_issues: report.blocking_issues,
        removable_residue: report.removable_residue,
        rendered,
        steps: report
            .steps
            .into_iter()
            .map(|step| UnifiedUpdatePreviewStepPayload {
                label: step.label,
                status: step.status,
                detail: step.detail,
            })
            .collect(),
    })
}
