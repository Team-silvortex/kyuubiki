use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn default_file_manifest() -> Value {
    json!({
        "layout_version": "kyuubiki.project-layout/v1",
        "engine_manifest_path": ".kyuubiki/project.json",
        "root_manifest_path": "project.json",
        "project_record_path": "Assets/project/project.json",
        "workspace_settings_path": "ProjectSettings/workspace.json",
        "workspace_snapshot_path": "Workspace/current-model.json",
        "automation_presets_path": "ProjectSettings/automation-presets.json",
        "asset_catalog_path": "ProjectSettings/asset-catalog.json",
        "asset_references_path": "ProjectSettings/asset-references.json",
        "model_directory": "Assets/models",
        "version_directory": "Assets/versions",
        "job_directory": "Analysis/jobs",
        "result_directory": "Analysis/results",
    })
}

pub(crate) fn normalize(mut bundle: Value) -> Result<Value, String> {
    let schema = bundle.get("project_schema_version").and_then(Value::as_str);
    if !matches!(schema, Some("kyuubiki.project/v1" | "kyuubiki.project/v2")) {
        return Err("unsupported project_schema_version".to_string());
    }
    if !bundle.get("project").is_some_and(Value::is_object)
        || !bundle.get("models").is_some_and(Value::is_array)
        || !bundle.get("model_versions").is_some_and(Value::is_array)
    {
        return Err("project bundle is missing required sections".to_string());
    }
    bundle["project_schema_version"] = Value::String("kyuubiki.project/v2".to_string());
    for key in [
        "automation_presets",
        "asset_catalog",
        "asset_references",
        "jobs",
        "results",
    ] {
        if !bundle.get(key).is_some_and(Value::is_array) {
            bundle[key] = json!([]);
        }
    }
    for key in ["active_model_id", "active_version_id", "workspace_snapshot"] {
        if bundle.get(key).is_none() {
            bundle[key] = Value::Null;
        }
    }
    if bundle.get("project_file_manifest").is_none() {
        bundle["project_file_manifest"] = default_file_manifest();
    }
    if bundle
        .get("asset_catalog")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        bundle["asset_catalog"] = Value::Array(build_asset_catalog(&bundle));
    }
    if bundle
        .get("asset_references")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        bundle["asset_references"] = Value::Array(build_asset_references(&bundle));
    }
    Ok(bundle)
}

fn array_len(bundle: &Value, key: &str) -> usize {
    bundle
        .get(key)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn nested_string(bundle: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(bundle, |value, key| value.get(*key))?
        .as_str()
        .map(ToOwned::to_owned)
}

fn analysis_values(bundle: &Value, field: &str) -> Vec<String> {
    let mut values = BTreeSet::new();
    for asset in bundle
        .get("asset_catalog")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|asset| asset.get("kind").and_then(Value::as_str) == Some("model"))
    {
        match asset.get(field) {
            Some(Value::String(value)) => {
                values.insert(value.clone());
            }
            Some(Value::Array(items)) => {
                values.extend(
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned),
                );
            }
            _ => {}
        }
    }
    values.into_iter().collect()
}

pub(crate) fn summary(bundle: &Value) -> Value {
    json!({
        "schema": bundle.get("project_schema_version").and_then(Value::as_str),
        "layout": nested_string(bundle, &["project_file_manifest", "layout_version"]),
        "project_id": nested_string(bundle, &["project", "project_id"]),
        "project_name": nested_string(bundle, &["project", "name"]),
        "model_count": array_len(bundle, "models"),
        "version_count": array_len(bundle, "model_versions"),
        "job_count": array_len(bundle, "jobs"),
        "result_count": array_len(bundle, "results"),
        "automation_preset_count": array_len(bundle, "automation_presets"),
        "asset_count": array_len(bundle, "asset_catalog"),
        "asset_reference_count": array_len(bundle, "asset_references"),
        "active_model_id": bundle.get("active_model_id"),
        "active_version_id": bundle.get("active_version_id"),
        "has_workspace_snapshot": !bundle.get("workspace_snapshot").unwrap_or(&Value::Null).is_null(),
        "analysis_domains": analysis_values(bundle, "analysis_domain"),
        "analysis_families": analysis_values(bundle, "analysis_family"),
        "thermal_intents": analysis_values(bundle, "thermal_intent"),
    })
}

fn build_asset_catalog(bundle: &Value) -> Vec<Value> {
    let project_id =
        nested_string(bundle, &["project", "project_id"]).unwrap_or_else(|| "project".to_string());
    let project_name =
        nested_string(bundle, &["project", "name"]).unwrap_or_else(|| "Untitled".to_string());
    let exported_at = bundle.get("exported_at").cloned().unwrap_or(Value::Null);
    let manifest = bundle
        .get("project_file_manifest")
        .cloned()
        .unwrap_or_else(default_file_manifest);
    let path = |key: &str, fallback: &str| {
        manifest
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    };
    let mut catalog = vec![
        asset(
            "project",
            &path("project_record_path", "Assets/project/project.json"),
            &project_id,
            &project_name,
            bundle
                .pointer("/project/updated_at")
                .cloned()
                .unwrap_or_else(|| exported_at.clone()),
        ),
        asset(
            "workspace_settings",
            &path("workspace_settings_path", "ProjectSettings/workspace.json"),
            &project_id,
            &format!("{project_name} workspace settings"),
            exported_at.clone(),
        ),
    ];
    if !bundle
        .get("workspace_snapshot")
        .unwrap_or(&Value::Null)
        .is_null()
    {
        let source = active_source(bundle, &project_id);
        catalog.push(asset(
            "workspace_snapshot",
            &path("workspace_snapshot_path", "Workspace/current-model.json"),
            &source,
            "Current workspace snapshot",
            exported_at.clone(),
        ));
    }
    for model in records(bundle, "models") {
        let Some(model_id) = model.get("model_id").and_then(Value::as_str) else {
            continue;
        };
        let mut entry = asset(
            "model",
            &format!(
                "{}/{}.json",
                path("model_directory", "Assets/models"),
                slug(model_id)
            ),
            model_id,
            model
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(model_id),
            model
                .get("updated_at")
                .cloned()
                .unwrap_or_else(|| exported_at.clone()),
        );
        merge_analysis_metadata(&mut entry, model.get("payload"));
        catalog.push(entry);
    }
    for version in records(bundle, "model_versions") {
        let Some(version_id) = version.get("version_id").and_then(Value::as_str) else {
            continue;
        };
        let version_name = version
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "{} v{}",
                    version
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("model"),
                    version
                        .get("version_number")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                )
            });
        let mut entry = asset(
            "model_version",
            &format!(
                "{}/{}.json",
                path("version_directory", "Assets/versions"),
                slug(version_id)
            ),
            version_id,
            &version_name,
            version
                .get("updated_at")
                .cloned()
                .unwrap_or_else(|| exported_at.clone()),
        );
        merge_analysis_metadata(&mut entry, version.get("payload"));
        catalog.push(entry);
    }
    for preset in records(bundle, "automation_presets") {
        let Some(id) = preset.get("presetId").and_then(Value::as_str) else {
            continue;
        };
        catalog.push(asset(
            "automation_preset",
            &path(
                "automation_presets_path",
                "ProjectSettings/automation-presets.json",
            ),
            id,
            preset.get("name").and_then(Value::as_str).unwrap_or(id),
            preset
                .get("updatedAt")
                .cloned()
                .unwrap_or_else(|| exported_at.clone()),
        ));
    }
    for job in records(bundle, "jobs") {
        let Some(id) = job.get("job_id").and_then(Value::as_str) else {
            continue;
        };
        catalog.push(asset(
            "job",
            &format!(
                "{}/{}.json",
                path("job_directory", "Analysis/jobs"),
                slug(id)
            ),
            id,
            job.get("simulation_case_id")
                .and_then(Value::as_str)
                .unwrap_or(id),
            job.get("updated_at")
                .cloned()
                .unwrap_or_else(|| exported_at.clone()),
        ));
    }
    for result in records(bundle, "results") {
        let Some(id) = result.get("job_id").and_then(Value::as_str) else {
            continue;
        };
        catalog.push(asset(
            "result",
            &format!(
                "{}/{}.json",
                path("result_directory", "Analysis/results"),
                slug(id)
            ),
            id,
            result.get("status").and_then(Value::as_str).unwrap_or(id),
            exported_at.clone(),
        ));
    }
    catalog
}

fn asset(kind: &str, path: &str, source_id: &str, name: &str, updated_at: Value) -> Value {
    json!({
        "guid": stable_guid(&format!("{kind}:{source_id}")),
        "meta_version": "kyuubiki.asset-meta/v1",
        "kind": kind,
        "path": path,
        "source_id": source_id,
        "name": name,
        "updated_at": updated_at,
    })
}

fn merge_analysis_metadata(entry: &mut Value, payload: Option<&Value>) {
    let Some(metadata) = payload.and_then(|value| value.get("analysis_metadata")) else {
        return;
    };
    let Some(entry) = entry.as_object_mut() else {
        return;
    };
    for (source, target) in [("domain", "analysis_domain"), ("family", "analysis_family")] {
        if let Some(value) = metadata.get(source).and_then(Value::as_str) {
            entry.insert(target.to_string(), Value::String(value.to_string()));
        }
    }
    if let Some(values) = metadata.get("thermal_intent").and_then(Value::as_array) {
        entry.insert(
            "thermal_intent".to_string(),
            Value::Array(
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|value| Value::String(value.to_string()))
                    .collect(),
            ),
        );
    }
}

fn build_asset_references(bundle: &Value) -> Vec<Value> {
    let guid_index = records(bundle, "asset_catalog")
        .filter_map(|entry| {
            Some((
                format!(
                    "{}:{}",
                    entry.get("kind")?.as_str()?,
                    entry.get("source_id")?.as_str()?
                ),
                entry.get("guid")?.as_str()?.to_string(),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let project_id =
        nested_string(bundle, &["project", "project_id"]).unwrap_or_else(|| "project".to_string());
    let project_guid = guid_index.get(&format!("project:{project_id}"));
    let mut references = Vec::new();
    add_reference(
        &mut references,
        project_guid,
        "workspace_settings_for",
        guid_index.get(&format!("workspace_settings:{project_id}")),
    );
    if let Some(active) = bundle.get("active_model_id").and_then(Value::as_str) {
        add_reference(
            &mut references,
            project_guid,
            "active_model",
            guid_index.get(&format!("model:{active}")),
        );
    }
    if let Some(active) = bundle.get("active_version_id").and_then(Value::as_str) {
        add_reference(
            &mut references,
            project_guid,
            "active_version",
            guid_index.get(&format!("model_version:{active}")),
        );
    }
    if !bundle
        .get("workspace_snapshot")
        .unwrap_or(&Value::Null)
        .is_null()
    {
        add_reference(
            &mut references,
            project_guid,
            "workspace_snapshot_of",
            guid_index.get(&format!(
                "workspace_snapshot:{}",
                active_source(bundle, &project_id)
            )),
        );
    }
    for model in records(bundle, "models") {
        if let Some(id) = model.get("model_id").and_then(Value::as_str) {
            add_reference(
                &mut references,
                project_guid,
                "contains",
                guid_index.get(&format!("model:{id}")),
            );
        }
    }
    for version in records(bundle, "model_versions") {
        let version_guid = version
            .get("version_id")
            .and_then(Value::as_str)
            .and_then(|id| guid_index.get(&format!("model_version:{id}")));
        add_reference(&mut references, project_guid, "contains", version_guid);
        let model_guid = version
            .get("model_id")
            .and_then(Value::as_str)
            .and_then(|id| guid_index.get(&format!("model:{id}")));
        add_reference(&mut references, version_guid, "version_of", model_guid);
    }
    for preset in records(bundle, "automation_presets") {
        if let Some(id) = preset.get("presetId").and_then(Value::as_str) {
            add_reference(
                &mut references,
                project_guid,
                "automation_for",
                guid_index.get(&format!("automation_preset:{id}")),
            );
        }
    }
    for job in records(bundle, "jobs") {
        let job_guid = job
            .get("job_id")
            .and_then(Value::as_str)
            .and_then(|id| guid_index.get(&format!("job:{id}")));
        add_reference(&mut references, project_guid, "job_for_project", job_guid);
        let version_guid = job
            .get("model_version_id")
            .and_then(Value::as_str)
            .and_then(|id| guid_index.get(&format!("model_version:{id}")));
        add_reference(&mut references, job_guid, "job_for_version", version_guid);
    }
    for result in records(bundle, "results") {
        if let Some(id) = result.get("job_id").and_then(Value::as_str) {
            add_reference(
                &mut references,
                guid_index.get(&format!("result:{id}")),
                "result_for_job",
                guid_index.get(&format!("job:{id}")),
            );
        }
    }
    references
}

fn add_reference(
    references: &mut Vec<Value>,
    from: Option<&String>,
    relation: &str,
    to: Option<&String>,
) {
    if let (Some(from), Some(to)) = (from, to) {
        references.push(json!({
            "from_guid": from,
            "relation": relation,
            "to_guid": to,
        }));
    }
}

fn active_source(bundle: &Value, project_id: &str) -> String {
    bundle
        .get("active_version_id")
        .and_then(Value::as_str)
        .or_else(|| bundle.get("active_model_id").and_then(Value::as_str))
        .unwrap_or(project_id)
        .to_string()
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in value.trim().to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            result.push(character);
            separator = false;
        } else if !separator && !result.is_empty() {
            result.push('-');
            separator = true;
        }
    }
    let normalized = result.trim_matches('-');
    if normalized.is_empty() {
        "asset".to_string()
    } else {
        normalized.to_string()
    }
}

fn stable_guid(seed: &str) -> String {
    let hash = seed.bytes().fold(0_u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    let first = format!("{hash:08x}");
    let second = format!("{:08x}", hash ^ 0x9e37_79b9);
    let third = format!("{:08x}", hash ^ 0x85eb_ca6b);
    let fourth = format!("{:08x}", hash ^ 0xc2b2_ae35);
    format!(
        "{}-{}-{}-{}-{}{}",
        first,
        &second[..4],
        &second[4..],
        &third[..4],
        &third[4..],
        &fourth[..4]
    )
}

pub(crate) fn validation(bundle: &Value) -> Value {
    let mut issues = Vec::new();
    let project_id = nested_string(bundle, &["project", "project_id"]);
    if project_id.is_none() {
        issues.push("project.project_id is required".to_string());
    }
    for key in [
        "models",
        "model_versions",
        "automation_presets",
        "asset_catalog",
        "asset_references",
        "jobs",
        "results",
    ] {
        if !bundle.get(key).is_some_and(Value::is_array) {
            issues.push(format!("{key} must be an array"));
        }
    }
    let model_ids = string_set(bundle, "models", "model_id");
    let version_ids = string_set(bundle, "model_versions", "version_id");
    for version in records(bundle, "model_versions") {
        let version_id = version
            .get("version_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if let Some(model_id) = version.get("model_id").and_then(Value::as_str)
            && !model_ids.contains(model_id)
        {
            issues.push(format!(
                "model version {version_id} points to missing model {model_id}"
            ));
        }
    }
    if let Some(active) = bundle.get("active_model_id").and_then(Value::as_str)
        && !model_ids.contains(active)
    {
        issues.push(format!("active_model_id points to missing model {active}"));
    }
    if let Some(active) = bundle.get("active_version_id").and_then(Value::as_str)
        && !version_ids.contains(active)
    {
        issues.push(format!(
            "active_version_id points to missing model version {active}"
        ));
    }
    let job_ids = string_set(bundle, "jobs", "job_id");
    for job in records(bundle, "jobs") {
        let Some(job_id) = job.get("job_id").and_then(Value::as_str) else {
            issues.push("job record is missing job_id".to_string());
            continue;
        };
        if let Some(version_id) = job.get("model_version_id").and_then(Value::as_str)
            && !version_ids.contains(version_id)
        {
            issues.push(format!(
                "job {job_id} points to missing model version {version_id}"
            ));
        }
    }
    for result in records(bundle, "results") {
        if let Some(job_id) = result.get("job_id").and_then(Value::as_str)
            && !job_ids.contains(job_id)
        {
            issues.push(format!("result {job_id} has no matching job record"));
        }
    }

    let mut guids = BTreeSet::new();
    let mut catalog_keys = BTreeSet::new();
    let mut expected_paths = BTreeSet::new();
    for entry in records(bundle, "asset_catalog") {
        let Some(guid) = entry.get("guid").and_then(Value::as_str) else {
            issues.push("asset_catalog contains an entry without a valid guid".to_string());
            continue;
        };
        if !guids.insert(guid.to_string()) {
            issues.push(format!("duplicate asset guid detected: {guid}"));
        }
        if let (Some(kind), Some(source)) = (
            entry.get("kind").and_then(Value::as_str),
            entry.get("source_id").and_then(Value::as_str),
        ) {
            catalog_keys.insert(format!("{kind}:{source}"));
        }
        if let Some(path) = entry.get("path").and_then(Value::as_str) {
            expected_paths.insert(path.to_string());
        } else {
            issues.push(format!("asset {guid} is missing a valid path"));
        }
    }
    if let Some(project_id) = &project_id {
        require_catalog_key(&catalog_keys, &format!("project:{project_id}"), &mut issues);
        require_catalog_key(
            &catalog_keys,
            &format!("workspace_settings:{project_id}"),
            &mut issues,
        );
    }
    for (records_key, id_field, kind) in [
        ("models", "model_id", "model"),
        ("model_versions", "version_id", "model_version"),
        ("automation_presets", "presetId", "automation_preset"),
        ("jobs", "job_id", "job"),
        ("results", "job_id", "result"),
    ] {
        for record in records(bundle, records_key) {
            if let Some(id) = record.get(id_field).and_then(Value::as_str) {
                require_catalog_key(&catalog_keys, &format!("{kind}:{id}"), &mut issues);
            }
        }
    }
    for reference in records(bundle, "asset_references") {
        for field in ["from_guid", "to_guid"] {
            match reference.get(field).and_then(Value::as_str) {
                Some(guid) if guids.contains(guid) => {}
                Some(guid) => issues.push(format!("asset reference has unknown {field} {guid}")),
                None => issues.push(format!("asset reference is missing {field}")),
            }
        }
    }
    json!({
        "ok": issues.is_empty(),
        "issue_count": issues.len(),
        "issues": issues,
        "summary": summary(bundle),
        "expected_paths": expected_paths,
    })
}

fn require_catalog_key(keys: &BTreeSet<String>, key: &str, issues: &mut Vec<String>) {
    if !keys.contains(key) {
        issues.push(format!("missing asset catalog entry for {key}"));
    }
}

pub(crate) fn diff(left: &Value, right: &Value) -> Value {
    let left_summary = summary(left);
    let right_summary = summary(right);
    let left_kinds = asset_kind_index(left);
    let right_kinds = asset_kind_index(right);
    let kinds = left_kinds
        .keys()
        .chain(right_kinds.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let asset_kind_diff = kinds
        .into_iter()
        .map(|kind| {
            let left_values = left_kinds.get(&kind).cloned().unwrap_or_default();
            let right_values = right_kinds.get(&kind).cloned().unwrap_or_default();
            (kind, set_diff(&left_values, &right_values))
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "left": left_summary,
        "right": right_summary,
        "changed_project_identity": nested_string(left, &["project", "project_id"]) != nested_string(right, &["project", "project_id"])
            || nested_string(left, &["project", "name"]) != nested_string(right, &["project", "name"]),
        "active_model_changed": left.get("active_model_id") != right.get("active_model_id"),
        "active_version_changed": left.get("active_version_id") != right.get("active_version_id"),
        "asset_kind_diff": asset_kind_diff,
        "automation_preset_ids": set_diff(
            &string_set(left, "automation_presets", "presetId"),
            &string_set(right, "automation_presets", "presetId")
        ),
    })
}

fn records<'a>(bundle: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    bundle
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn string_set(bundle: &Value, key: &str, field: &str) -> BTreeSet<String> {
    records(bundle, key)
        .filter_map(|record| record.get(field).and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

fn asset_kind_index(bundle: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut index = BTreeMap::<String, BTreeSet<String>>::new();
    for asset in records(bundle, "asset_catalog") {
        if let (Some(kind), Some(source_id)) = (
            asset.get("kind").and_then(Value::as_str),
            asset.get("source_id").and_then(Value::as_str),
        ) {
            index
                .entry(kind.to_string())
                .or_default()
                .insert(source_id.to_string());
        }
    }
    index
}

fn set_diff(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Value {
    json!({
        "added": right.difference(left).cloned().collect::<Vec<_>>(),
        "removed": left.difference(right).cloned().collect::<Vec<_>>(),
    })
}
