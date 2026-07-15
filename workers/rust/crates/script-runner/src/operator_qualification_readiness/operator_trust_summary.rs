use crate::RunnerResult;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use super::{array, field, read_json};

const RELIABILITY_MANIFEST_PATH: &str = "config/operator-reliability-manifest.json";
const OPERATOR_TRUST_LEVELS: &[&str] = &["smoke", "baseline", "review", "qualification"];

pub(super) fn operator_trust_level_counts(root: &Path) -> RunnerResult<Value> {
    let manifest = read_json(root, RELIABILITY_MANIFEST_PATH)?;
    let mut counts = OPERATOR_TRUST_LEVELS
        .iter()
        .map(|level| ((*level).to_string(), 0usize))
        .collect::<HashMap<_, _>>();
    for shard_path in array(&manifest, "shards") {
        let shard_path = shard_path
            .as_str()
            .ok_or_else(|| "reliability manifest shards must be strings".to_string())?;
        let shard = read_json(root, shard_path)?;
        for operator in array(&shard, "operators") {
            if let Some(count) = counts.get_mut(field(operator, "coverage_level")) {
                *count += 1;
            }
        }
    }
    let map = OPERATOR_TRUST_LEVELS
        .iter()
        .map(|level| {
            (
                (*level).to_string(),
                Value::from(counts.get(*level).copied().unwrap_or(0)),
            )
        })
        .collect();
    Ok(Value::Object(map))
}
