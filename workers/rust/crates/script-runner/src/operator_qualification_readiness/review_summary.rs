use serde_json::{Value, json};
use std::path::Path;

use super::{array, field, read_json, repo_local_path};

const RELEASE_REVIEW_STATUSES: &[&str] = &[
    "missing",
    "pending_signoff",
    "approved",
    "blocked_scope",
    "rejected",
];
const RELEASE_RECORDS_PATH: &str = "releases/qualification-records/1.20.0.json";

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

pub(super) fn count_release_promotion_summaries<T>(root: &Path, candidates: &[T]) -> Value
where
    T: std::borrow::Borrow<Value>,
{
    let release_version = read_json(root, RELEASE_RECORDS_PATH)
        .ok()
        .map(|records| field(&records, "release_version").to_string())
        .unwrap_or_default();
    let approved = release_artifacts(candidates)
        .into_iter()
        .filter(|artifact| field(artifact, "release_review_status") == "approved")
        .collect::<Vec<_>>();
    let retained = approved
        .iter()
        .filter(|artifact| retained_evidence(root, artifact).is_some())
        .count();
    let declared = approved
        .iter()
        .filter(|artifact| {
            retained_evidence(root, artifact)
                .and_then(|evidence| evidence.get("promotion_summary").cloned())
                .is_some()
        })
        .count();
    let matched = candidates
        .iter()
        .flat_map(|candidate| {
            array(candidate.borrow(), "artifacts")
                .into_iter()
                .filter(|artifact| {
                    field(artifact, "kind") == "release_retained_regression_output"
                        && field(artifact, "release_review_status") == "approved"
                        && promotion_summary_matches(
                            root,
                            candidate.borrow(),
                            artifact,
                            &release_version,
                        )
                })
                .collect::<Vec<_>>()
        })
        .count();
    json!({
        "required": approved.len(),
        "retained": retained,
        "declared": declared,
        "matched": matched,
        "missing": approved.len().saturating_sub(matched),
    })
}

fn promotion_summary_matches(
    root: &Path,
    candidate: &Value,
    artifact: &Value,
    release_version: &str,
) -> bool {
    let Some(summary) = retained_evidence(root, artifact)
        .and_then(|evidence| evidence.get("promotion_summary").cloned())
    else {
        return false;
    };
    field(&summary, "candidate_id") == field(candidate, "candidate_id")
        && field(&summary, "release_version") == release_version
        && field(&summary, "approved_coverage_level") == "qualification"
        && field(&summary, "retained_evidence_path") == field(artifact, "release_record_path")
        && field(&summary, "release_record_path") == RELEASE_RECORDS_PATH
        && field(&summary, "review_decision_path")
            == field(artifact, "release_review_decision_path")
        && sorted_strings(array_strings(&summary, "promoted_operator_ids"))
            == sorted_strings(array_strings(candidate, "operator_ids"))
}

fn retained_evidence(root: &Path, artifact: &Value) -> Option<Value> {
    let evidence_path = field(artifact, "release_record_path");
    if evidence_path.is_empty() {
        return None;
    }
    read_json(root, evidence_path).ok()
}

fn array_strings(value: &Value, key: &str) -> Vec<String> {
    array(value, key)
        .into_iter()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn sorted_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
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
