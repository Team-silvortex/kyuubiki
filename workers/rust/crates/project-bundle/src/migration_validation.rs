use crate::{migration_input::safe_name, model};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn normalize_checked(raw: Value) -> Result<Value, String> {
    if raw
        .pointer("/project/project_id")
        .and_then(Value::as_str)
        .is_none_or(|id| id.trim().is_empty())
    {
        return Err("project.project_id must be a nonempty string".into());
    }
    for (section, id) in [
        ("models", "model_id"),
        ("model_versions", "version_id"),
        ("jobs", "job_id"),
        ("results", "job_id"),
        ("automation_presets", "presetId"),
    ] {
        let Some(records) = raw.get(section) else {
            continue;
        };
        let records = records
            .as_array()
            .ok_or_else(|| format!("{section} must be an array; refusing data loss"))?;
        let mut ids = BTreeSet::new();
        for record in records {
            let value = record
                .get(id)
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| format!("{section} contains a record without {id}"))?;
            if !ids.insert(value) {
                return Err(format!("duplicate {id} in {section}: {value}"));
            }
        }
    }
    for field in ["active_model_id", "active_version_id"] {
        if let Some(value) = raw.get(field) {
            if !value.is_null() && !value.is_string() {
                return Err(format!("{field} must be a string or null"));
            }
        }
    }
    if let Some(value) = raw.get("workspace_snapshot") {
        if !value.is_null() && !value.is_object() {
            return Err("workspace_snapshot must be an object or null".into());
        }
    }
    if let Some(layout) = raw.get("project_file_manifest") {
        if !layout.is_object()
            || layout.get("layout_version").and_then(Value::as_str)
                != Some("kyuubiki.project-layout/v1")
        {
            return Err("unsupported project layout; explicit layout migration required".into());
        }
        for (key, _) in model::default_file_manifest().as_object().unwrap() {
            let value = layout
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("layout is missing {key}"))?;
            if key != "layout_version" {
                safe_name(value)?;
            }
        }
        if layout["root_manifest_path"] != "project.json" {
            return Err("unsupported root manifest path; project.json is required".into());
        }
    }
    let target = model::normalize(raw)?;
    let report = model::validation(&target);
    if report["ok"] != true {
        return Err(format!(
            "project references are invalid: {}",
            report["issues"]
        ));
    }
    let mut paths = BTreeSet::new();
    for asset in target["asset_catalog"].as_array().unwrap() {
        let path = asset
            .get("path")
            .and_then(Value::as_str)
            .ok_or("asset path missing")?;
        safe_name(path)?;
        // Presets intentionally share one catalog file; all other assets have unique paths.
        if asset["kind"] != "automation_preset" && !paths.insert(path.to_lowercase()) {
            return Err(format!("colliding asset path: {path}"));
        }
    }
    Ok(target)
}
