use super::{array, field, read_json};
use crate::RunnerResult;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;

const VALIDATION_PROFILES_PATH: &str = "config/operator-validation-profiles.json";

pub(super) fn validation_profiles_by_candidate(
    root: &Path,
) -> RunnerResult<HashMap<String, Vec<Value>>> {
    let source = load_validation_profiles(root)?;
    let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();
    for profile in array(&source, "profiles") {
        let candidate_id = field(profile, "qualification_candidate_id");
        grouped
            .entry(candidate_id.to_string())
            .or_default()
            .push(json!({
                "profile_id": field(profile, "profile_id"),
                "profile_role": field(profile, "profile_role"),
                "trust_goal": field(profile, "trust_goal"),
                "operator_count": array(profile, "operators").len(),
                "command_count": array(profile, "commands").len(),
            }));
    }
    for profiles in grouped.values_mut() {
        profiles.sort_by(|left, right| {
            field(left, "profile_role")
                .cmp(field(right, "profile_role"))
                .then(field(left, "profile_id").cmp(field(right, "profile_id")))
        });
    }
    Ok(grouped)
}

fn load_validation_profiles(root: &Path) -> RunnerResult<Value> {
    let mut source = read_json(root, VALIDATION_PROFILES_PATH)?;
    let version_line = field(&source, "version_line").to_string();
    let mut profiles = array(&source, "profiles")
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    for shard_path in string_array(source.get("profile_shards")) {
        let shard = read_json(root, shard_path)?;
        if field(&shard, "schema_version") != field(&source, "schema_version") {
            return Err(format!(
                "{shard_path}: schema_version must match {VALIDATION_PROFILES_PATH}"
            ));
        }
        if field(&shard, "version_line") != version_line {
            return Err(format!(
                "{shard_path}: version_line must match {VALIDATION_PROFILES_PATH}"
            ));
        }
        profiles.extend(array(&shard, "profiles").into_iter().cloned());
    }
    source["profiles"] = Value::Array(profiles);
    Ok(source)
}

fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}
