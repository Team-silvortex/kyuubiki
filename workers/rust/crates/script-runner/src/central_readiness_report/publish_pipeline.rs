use serde_json::{Value, json};
use std::collections::HashMap;

const SCHEMA: &str = "kyuubiki.central-publish-pipeline/v1";
const STAGES: &[&str] = &[
    "publisher_identity",
    "artifact_envelope",
    "signature_attestation",
    "review_queue",
    "catalog_indexing",
    "recall_and_yank",
    "download_verification",
];
const BLOCKERS: &[&str] = &[
    "publisher_accounts_not_enabled",
    "token_issuer_not_configured",
    "artifact_upload_endpoint_disabled",
    "signing_keys_not_configured",
    "write_side_review_queue_not_enabled",
    "central_write_api_disabled",
    "write_audit_log_not_enabled",
];

pub(super) fn contract(files: &HashMap<String, String>) -> Value {
    let backend = files.get("apps/web/lib/kyuubiki_web/central_store.ex");
    let docs = files.get("docs/central-server-components.md");
    json!({
        "schema_version": SCHEMA,
        "status": "blocked_preview",
        "accepting_writes": false,
        "stage_count": STAGES.len(),
        "stages_present": STAGES.iter().map(|stage| json!({
            "id": stage,
            "present": backend.is_some_and(|text| text.contains(stage))
        })).collect::<Vec<_>>(),
        "blockers_present": BLOCKERS.iter().map(|blocker| json!({
            "id": blocker,
            "present": backend.is_some_and(|text| text.contains(blocker))
        })).collect::<Vec<_>>(),
        "readonly_guard_present": includes_all(backend, &[
            "\"mode\" => \"read_only_contract\"",
            "\"accepting_writes\" => false",
            "\"writes_enabled\" => false"
        ]),
        "docs_present": includes_all(docs, &[
            "publish pipeline",
            "accepting_writes=false",
            "publisher identity",
            "installer download"
        ])
    })
}

pub(super) fn validate(issues: &mut Vec<String>, contract: Option<&Value>) {
    let Some(contract) = contract else {
        issues.push("publish pipeline contract missing".to_string());
        return;
    };
    if contract.get("schema_version").and_then(Value::as_str) != Some(SCHEMA) {
        issues.push("publish pipeline schema version mismatch".to_string());
    }
    if contract.get("status").and_then(Value::as_str) != Some("blocked_preview") {
        issues.push("publish pipeline status must remain blocked_preview".to_string());
    }
    if contract.get("accepting_writes").and_then(Value::as_bool) != Some(false) {
        issues.push("publish pipeline must not accept writes yet".to_string());
    }
    if contract
        .get("readonly_guard_present")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push("publish pipeline readonly guard missing".to_string());
    }
    if contract.get("docs_present").and_then(Value::as_bool) != Some(true) {
        issues.push("publish pipeline docs coverage missing".to_string());
    }
    validate_rows(issues, contract, "stages_present", "stage");
    validate_rows(issues, contract, "blockers_present", "blocker");
}

pub(super) fn fixture_backend() -> String {
    [
        "\"mode\" => \"read_only_contract\"",
        "\"accepting_writes\" => false",
        "\"writes_enabled\" => false",
    ]
    .into_iter()
    .chain(STAGES.iter().copied())
    .chain(BLOCKERS.iter().copied())
    .collect::<Vec<_>>()
    .join("\n")
}

pub(super) fn fixture_docs() -> String {
    [
        "central-web-service",
        "not a separate top-level module",
        "publish pipeline",
        "accepting_writes=false",
        "publisher identity",
        "installer download",
    ]
    .join("\n")
}

fn validate_rows(issues: &mut Vec<String>, contract: &Value, field: &str, label: &str) {
    for row in contract
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if row.get("present").and_then(Value::as_bool) != Some(true) {
            issues.push(format!(
                "publish pipeline {label} missing {}",
                row.get("id").and_then(Value::as_str).unwrap_or_default()
            ));
        }
    }
}

fn includes_all(text: Option<&String>, needles: &[&str]) -> bool {
    text.is_some_and(|text| needles.iter().all(|needle| text.contains(needle)))
}
