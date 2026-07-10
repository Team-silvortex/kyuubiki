use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::{
    IntegrityContractRule, Platform, load_integrity_contract, repair_installation, stage_release,
    update_source::current_update_catalog_path, workspace_root,
};

#[cfg(test)]
#[path = "update_catalog_fuzz.rs"]
mod update_catalog_fuzz;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateArtifactRef {
    pub product: String,
    pub platform: String,
    pub kind: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnifiedUpdatePlan {
    pub schema_version: String,
    pub workspace: String,
    pub current_version: String,
    pub target_channel: String,
    pub target_tag: String,
    pub target_version: String,
    pub update_state: String,
    pub summary: String,
    pub contract_rules: Vec<IntegrityContractRule>,
    pub artifacts: Vec<UpdateArtifactRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnifiedUpdatePreviewStep {
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnifiedUpdatePreview {
    pub schema_version: String,
    pub channel: String,
    pub target_version: String,
    pub overall_status: String,
    pub blocking_issues: usize,
    pub removable_residue: usize,
    pub steps: Vec<UnifiedUpdatePreviewStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedUpdateRecord {
    pub channel: String,
    pub target_version: String,
    pub release_dir: String,
    pub manifest_path: String,
    pub audit_path: String,
}

impl UnifiedUpdatePlan {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki unified update plan".to_string(),
            format!("workspace: {}", self.workspace),
            format!("current_version: {}", self.current_version),
            format!(
                "target_channel: {} ({})",
                self.target_channel, self.target_tag
            ),
            format!("target_version: {}", self.target_version),
            format!("state: {}", self.update_state),
            format!("summary: {}", self.summary),
        ];

        for rule in &self.contract_rules {
            lines.push(format!(
                "[rule] {} = {} ({})",
                rule.label, rule.value, rule.description
            ));
        }

        for artifact in &self.artifacts {
            lines.push(format!(
                "[artifact] {} {} {} {} {}",
                artifact.product,
                artifact.platform,
                artifact.kind,
                if artifact.exists {
                    "present"
                } else {
                    "declared"
                },
                artifact.path
            ));
        }

        lines.join("\n")
    }
}

impl UnifiedUpdatePreview {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki unified update preview".to_string(),
            format!("channel: {}", self.channel),
            format!("target_version: {}", self.target_version),
            format!("overall_status: {}", self.overall_status),
            format!("blocking_issues: {}", self.blocking_issues),
            format!("removable_residue: {}", self.removable_residue),
        ];

        for step in &self.steps {
            lines.push(format!(
                "[step] {} {} - {}",
                step.status, step.label, step.detail
            ));
        }

        lines.join("\n")
    }
}

impl StagedUpdateRecord {
    pub fn render(&self) -> String {
        [
            "kyuubiki staged update prepared".to_string(),
            format!("channel: {}", self.channel),
            format!("target_version: {}", self.target_version),
            format!("release_dir: {}", self.release_dir),
            format!("manifest_path: {}", self.manifest_path),
            format!("audit_path: {}", self.audit_path),
        ]
        .join("\n")
    }
}

pub fn unified_update_plan(channel: Option<String>) -> Result<UnifiedUpdatePlan, String> {
    let root = workspace_root();
    let catalog = load_update_catalog()?;
    let selected_channel = select_channel(&catalog, channel.as_deref())?;
    let target_version = value_string(selected_channel.get("version"));
    let current_version = load_integrity_contract(&root, Platform::current())?.shipping_version;
    let update_state = match compare_versions(&current_version, &target_version) {
        Ordering::Less => "update_available",
        Ordering::Equal => "up_to_date",
        Ordering::Greater => "ahead_of_channel",
    }
    .to_string();

    Ok(UnifiedUpdatePlan {
        schema_version: value_string(catalog.get("schema_version")),
        workspace: root.display().to_string(),
        current_version,
        target_channel: value_string(selected_channel.get("id")),
        target_tag: value_string(selected_channel.get("tag")),
        target_version,
        update_state,
        summary: value_string(selected_channel.get("summary")),
        contract_rules: parse_rules(selected_channel.get("visible_rules")),
        artifacts: parse_artifacts(&root, selected_channel.get("desktop_artifacts")),
    })
}

pub fn unified_update_preview(channel: Option<String>) -> Result<UnifiedUpdatePreview, String> {
    let plan = unified_update_plan(channel)?;
    let integrity = crate::installation_integrity_report();
    let version_mismatches = integrity
        .version_checks
        .iter()
        .filter(|check| !check.ok)
        .count();
    let missing_paths = integrity
        .layout
        .iter()
        .filter(|entry| entry.required && !entry.present)
        .count();
    let removable_residue = integrity
        .residues
        .iter()
        .filter(|entry| entry.removable)
        .count();
    let blocking_issues = integrity.issues.len() + version_mismatches + missing_paths;
    let overall_status = if plan.update_state == "up_to_date" {
        "noop"
    } else if blocking_issues > 0 {
        "blocked"
    } else {
        "ready_for_apply"
    }
    .to_string();

    let steps = vec![
        UnifiedUpdatePreviewStep {
            label: "resolve install drift".to_string(),
            status: if integrity.issues.is_empty() {
                "ok"
            } else {
                "attention"
            }
            .to_string(),
            detail: if integrity.issues.is_empty() {
                "installation contract is currently healthy".to_string()
            } else {
                format!(
                    "{} visible install issues should be cleared first",
                    integrity.issues.len()
                )
            },
        },
        UnifiedUpdatePreviewStep {
            label: "clean removable residue".to_string(),
            status: if removable_residue == 0 {
                "ok"
            } else {
                "recommended"
            }
            .to_string(),
            detail: if removable_residue == 0 {
                "no allowlisted cleanup candidate detected".to_string()
            } else {
                format!(
                    "{removable_residue} allowlisted residue item(s) can be removed before update"
                )
            },
        },
        UnifiedUpdatePreviewStep {
            label: "validate channel target".to_string(),
            status: match plan.update_state.as_str() {
                "update_available" => "ready",
                "up_to_date" => "noop",
                "ahead_of_channel" => "review",
                _ => "review",
            }
            .to_string(),
            detail: format!(
                "channel {} resolves to {} with state {}",
                plan.target_tag, plan.target_version, plan.update_state
            ),
        },
        UnifiedUpdatePreviewStep {
            label: "review artifact references".to_string(),
            status: if plan.artifacts.is_empty() {
                "review"
            } else {
                "ok"
            }
            .to_string(),
            detail: if plan.artifacts.is_empty() {
                "selected channel has no declared desktop artifact references".to_string()
            } else {
                format!(
                    "{} desktop artifact reference(s) are declared for this channel",
                    plan.artifacts.len()
                )
            },
        },
    ];

    Ok(UnifiedUpdatePreview {
        schema_version: "kyuubiki.update-preview/v1".to_string(),
        channel: plan.target_channel,
        target_version: plan.target_version,
        overall_status,
        blocking_issues,
        removable_residue,
        steps,
    })
}

pub fn prepare_staged_update(
    channel: Option<String>,
    platform: Platform,
    target_dir: Option<PathBuf>,
) -> Result<StagedUpdateRecord, String> {
    let plan = unified_update_plan(channel.clone())?;
    let preview = unified_update_preview(channel)?;
    if preview.overall_status != "ready_for_apply" {
        return Err(format!(
            "staged update blocked: preview status is {}",
            preview.overall_status
        ));
    }

    repair_installation()?;
    stage_release(platform, target_dir.clone())?;

    let root = workspace_root();
    let release_dir = target_dir.unwrap_or_else(|| root.join("dist").join(platform.as_str()));
    let latest_path = root.join("dist").join("staged-update-latest.json");
    let manifest_path = release_dir.join("manifests").join("staged-update.json");
    let audit_path = release_dir.join("logs").join("staged-update-audit.jsonl");
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let manifest = json!({
        "schema_version": "kyuubiki.staged-update/v1",
        "generated_at": generated_at,
        "platform": platform.as_str(),
        "channel": {
            "id": plan.target_channel,
            "tag": plan.target_tag,
            "target_version": plan.target_version,
            "summary": plan.summary,
        },
        "preview": {
            "overall_status": preview.overall_status,
            "blocking_issues": preview.blocking_issues,
            "removable_residue": preview.removable_residue,
        },
        "workspace": root.display().to_string(),
        "release_dir": release_dir.display().to_string(),
        "artifacts": plan
            .artifacts
            .iter()
            .map(|artifact| json!({
                "product": artifact.product,
                "platform": artifact.platform,
                "kind": artifact.kind,
                "path": artifact.path,
                "exists": artifact.exists,
            }))
            .collect::<Vec<Value>>(),
    });
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))?;
    let audit_line = json!({
        "ts": generated_at,
        "action": "prepare_staged_update",
        "channel": manifest["channel"]["id"],
        "target_version": manifest["channel"]["target_version"],
        "release_dir": release_dir.display().to_string(),
        "manifest_path": manifest_path.display().to_string(),
    });
    let mut audit_contents = fs::read_to_string(&audit_path).unwrap_or_default();
    audit_contents.push_str(&format!("{audit_line}\n"));
    fs::write(&audit_path, audit_contents)
        .map_err(|error| format!("failed to write {}: {error}", audit_path.display()))?;
    fs::write(
        &latest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": "kyuubiki.staged-update-pointer/v1",
            "channel": manifest["channel"]["id"],
            "target_version": manifest["channel"]["target_version"],
            "release_dir": release_dir.display().to_string(),
            "manifest_path": manifest_path.display().to_string(),
            "audit_path": audit_path.display().to_string(),
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", latest_path.display()))?;

    Ok(StagedUpdateRecord {
        channel: manifest["channel"]["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        target_version: manifest["channel"]["target_version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        release_dir: release_dir.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        audit_path: audit_path.display().to_string(),
    })
}

pub fn latest_staged_update_record() -> Result<Option<StagedUpdateRecord>, String> {
    let path = workspace_root()
        .join("dist")
        .join("staged-update-latest.json");
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(Some(StagedUpdateRecord {
        channel: value_string(value.get("channel")),
        target_version: value_string(value.get("target_version")),
        release_dir: value_string(value.get("release_dir")),
        manifest_path: value_string(value.get("manifest_path")),
        audit_path: value_string(value.get("audit_path")),
    }))
}

fn load_update_catalog() -> Result<Value, String> {
    let path = update_catalog_path();
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) fn update_catalog_path() -> PathBuf {
    current_update_catalog_path(&workspace_root())
}

fn select_channel<'a>(catalog: &'a Value, requested: Option<&str>) -> Result<&'a Value, String> {
    let channels = catalog
        .get("channels")
        .and_then(Value::as_array)
        .ok_or_else(|| "update catalog is missing channels".to_string())?;
    let default_channel = value_string(catalog.get("default_channel"));
    let desired = requested.unwrap_or(default_channel.as_str());

    channels
        .iter()
        .find(|channel| {
            value_string(channel.get("id")) == desired
                || value_string(channel.get("tag")) == desired
                || channel
                    .get("aliases")
                    .and_then(Value::as_array)
                    .map(|aliases| {
                        aliases
                            .iter()
                            .any(|alias| value_string(Some(alias)) == desired)
                    })
                    .unwrap_or(false)
        })
        .ok_or_else(|| format!("unknown update channel: {desired}"))
}

fn parse_rules(value: Option<&Value>) -> Vec<IntegrityContractRule> {
    value
        .and_then(Value::as_array)
        .map(|rules| {
            rules
                .iter()
                .map(|rule| IntegrityContractRule {
                    category: "update".to_string(),
                    label: value_string(rule.get("label")),
                    value: value_string(rule.get("value")),
                    editable: false,
                    description: value_string(rule.get("description")),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_artifacts(root: &std::path::Path, value: Option<&Value>) -> Vec<UpdateArtifactRef> {
    value
        .and_then(Value::as_array)
        .map(|artifacts| {
            artifacts
                .iter()
                .map(|artifact| {
                    let path = value_string(artifact.get("path"));
                    UpdateArtifactRef {
                        product: value_string(artifact.get("product")),
                        platform: value_string(artifact.get("platform")),
                        kind: value_string(artifact.get("kind")),
                        exists: artifact
                            .get("exists")
                            .and_then(Value::as_bool)
                            .unwrap_or_else(|| root.join(&path).exists()),
                        path,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn value_string(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = parse_version_parts(left);
    let right_parts = parse_version_parts(right);
    let width = left_parts.len().max(right_parts.len());

    for index in 0..width {
        let left_value = *left_parts.get(index).unwrap_or(&0);
        let right_value = *right_parts.get(index).unwrap_or(&0);
        match left_value.cmp(&right_value) {
            Ordering::Equal => continue,
            non_equal => return non_equal,
        }
    }

    Ordering::Equal
}

fn parse_version_parts(value: &str) -> Vec<u32> {
    value
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .collect()
}
