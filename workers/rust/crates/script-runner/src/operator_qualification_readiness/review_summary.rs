use serde_json::{Value, json};
use std::path::Path;

use super::{array, field, repo_local_path};

const RELEASE_REVIEW_STATUSES: &[&str] = &[
    "missing",
    "pending_signoff",
    "approved",
    "blocked_scope",
    "rejected",
];

pub(super) fn count_release_review_statuses<T>(candidates: &[T]) -> Value
where
    T: std::borrow::Borrow<Value>,
{
    let mut map = serde_json::Map::new();
    for status in RELEASE_REVIEW_STATUSES {
        let count = release_artifacts(candidates)
            .into_iter()
            .filter(|artifact| {
                let review_status = field(artifact, "release_review_status");
                (review_status.is_empty() && *status == "missing") || review_status == *status
            })
            .count();
        map.insert((*status).to_string(), Value::from(count));
    }
    Value::Object(map)
}

pub(super) fn count_release_review_decisions<T>(root: &Path, candidates: &[T]) -> Value
where
    T: std::borrow::Borrow<Value>,
{
    let release_artifacts = release_artifacts(candidates);
    let declared = release_artifacts
        .iter()
        .filter(|artifact| !field(artifact, "release_review_decision_path").is_empty())
        .collect::<Vec<_>>();
    let retained = declared
        .iter()
        .filter(|artifact| {
            let decision_path = field(artifact, "release_review_decision_path");
            repo_local_path(root, decision_path, "release_review_decision_path")
                .map(|(absolute, _)| absolute.exists())
                .unwrap_or(false)
        })
        .count();
    json!({
        "required": release_artifacts.len(),
        "declared": declared.len(),
        "retained": retained,
        "missing": release_artifacts.len().saturating_sub(retained),
    })
}

fn release_artifacts<T>(candidates: &[T]) -> Vec<&Value>
where
    T: std::borrow::Borrow<Value>,
{
    candidates
        .iter()
        .flat_map(|candidate| array(candidate.borrow(), "artifacts"))
        .filter(|artifact| field(artifact, "kind") == "release_retained_regression_output")
        .collect()
}
