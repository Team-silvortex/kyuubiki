use serde_json::Value;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const DEFAULT_PATH: &str = "config/operator-qualification-evidence-kits.json";

pub(crate) fn load_qualification_evidence_kits(root: &Path) -> RunnerResult<Value> {
    load_qualification_evidence_kits_from(root, DEFAULT_PATH)
}

pub(crate) fn load_qualification_evidence_kits_from(
    root: &Path,
    relative_path: &str,
) -> RunnerResult<Value> {
    let mut source = read_json(root, relative_path)?;
    let schema_version = field(&source, "schema_version").to_string();
    let version_line = field(&source, "version_line").to_string();
    let mut kits = source
        .get("kits")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| format!("{relative_path}: kits must be an array"))?;

    for shard_path in string_array(source.get("kit_shards")) {
        let shard = read_json(root, shard_path)?;
        if field(&shard, "schema_version") != schema_version {
            return Err(format!(
                "{shard_path}: schema_version must match {relative_path}"
            ));
        }
        if field(&shard, "version_line") != version_line {
            return Err(format!(
                "{shard_path}: version_line must match {relative_path}"
            ));
        }
        if !string_array(shard.get("kit_shards")).is_empty() {
            return Err(format!("{shard_path}: nested kit_shards are not supported"));
        }
        kits.extend(
            shard
                .get("kits")
                .and_then(Value::as_array)
                .cloned()
                .ok_or_else(|| format!("{shard_path}: kits must be an array"))?,
        );
    }

    source["kits"] = Value::Array(kits);
    Ok(source)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    if relative_path.starts_with('/') || relative_path.contains("..") {
        return Err(format!("{relative_path}: path must be repository-relative"));
    }
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
