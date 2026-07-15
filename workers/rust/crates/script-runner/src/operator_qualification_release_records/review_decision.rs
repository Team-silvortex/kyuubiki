use crate::RunnerResult;
use serde_json::Value;
use std::path::Path;

use super::{CandidateGate, assert_eq, field, read_json};

pub(super) fn validate_review_status_transition(
    candidate_id: &str,
    review_status: &str,
    candidate: &CandidateGate,
) -> RunnerResult<()> {
    if review_status == "approved"
        && (candidate.release_gate_impact == "experimental_only"
            || candidate.target_level == "review")
    {
        return Err(format!(
            "{candidate_id}: review_status=approved is not allowed for review-only or experimental candidates"
        ));
    }
    if review_status == "blocked_scope"
        && candidate.release_gate_impact != "experimental_only"
        && candidate.target_level != "review"
    {
        return Err(format!(
            "{candidate_id}: blocked_scope is only for review-only or experimental candidates"
        ));
    }
    Ok(())
}

pub(super) fn validate_review_decision_path(
    root: &Path,
    release_version: &str,
    record: &Value,
) -> RunnerResult<()> {
    let decision_path = field(record, "review_decision_path");
    if decision_path.is_empty() {
        return Ok(());
    }
    let decision = read_json(root, decision_path)?;
    let candidate_id = field(record, "candidate_id");
    validate_review_decision_shape(candidate_id, &decision)?;
    assert_eq(
        field(&decision, "schema_version"),
        "kyuubiki.operator-qualification-review-decision/v1",
        "review decision schema_version",
    )?;
    assert_eq(
        field(&decision, "candidate_id"),
        candidate_id,
        "candidate_id",
    )?;
    assert_eq(
        field(&decision, "release_version"),
        release_version,
        "release_version",
    )?;
    assert_eq(
        field(&decision, "evidence_path"),
        field(record, "evidence_path"),
        "evidence_path",
    )?;
    assert_eq(
        field(&decision, "review_gate"),
        field(record, "review_gate"),
        "review_gate",
    )?;
    let expected_status = match field(&decision, "decision") {
        "approve_promotion" => "approved",
        "request_changes" => "pending_signoff",
        "reject_promotion" => "rejected",
        "block_scope" => "blocked_scope",
        other => {
            return Err(format!(
                "{candidate_id}: unsupported review decision {other}"
            ));
        }
    };
    if field(record, "review_status") != expected_status {
        return Err(format!(
            "{candidate_id}: review_decision_path decision does not match review_status"
        ));
    }
    Ok(())
}

fn validate_review_decision_shape(candidate_id: &str, decision: &Value) -> RunnerResult<()> {
    for key in [
        "schema_version",
        "candidate_id",
        "release_version",
        "evidence_path",
        "review_gate",
        "decision",
        "reason",
        "decided_at",
    ] {
        if field(decision, key).is_empty() {
            return Err(format!(
                "{candidate_id}: review decision {key} must be non-empty"
            ));
        }
    }
    let reviewer = decision.get("reviewer").unwrap_or(&Value::Null);
    if field(reviewer, "id").is_empty() || field(reviewer, "display_name").is_empty() {
        return Err(format!(
            "{candidate_id}: review decision reviewer id and display_name must be non-empty"
        ));
    }
    let completed_items = decision
        .get("completed_gate_items")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{candidate_id}: completed_gate_items must be an array"))?;
    let requested_changes = decision
        .get("requested_changes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{candidate_id}: requested_changes must be an array"))?;
    if field(decision, "decision") == "approve_promotion" && completed_items.is_empty() {
        return Err(format!(
            "{candidate_id}: approve_promotion requires completed_gate_items"
        ));
    }
    if field(decision, "decision") == "request_changes" && requested_changes.is_empty() {
        return Err(format!(
            "{candidate_id}: request_changes requires requested_changes"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_review_decision_path, validate_review_decision_shape,
        validate_review_status_transition,
    };
    use crate::operator_qualification_release_records::CandidateGate;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn review_only_candidate_cannot_be_approved() {
        let candidate = CandidateGate {
            target_level: "review".to_string(),
            release_gate_impact: "experimental_only".to_string(),
            graduation_gate: "scope gate".to_string(),
            operator_ids: vec!["solve.example".to_string()],
        };
        let error = validate_review_status_transition("screening", "approved", &candidate)
            .expect_err("review-only candidate approval should fail");
        assert!(error.contains("not allowed"));
        validate_review_status_transition("screening", "blocked_scope", &candidate)
            .expect("scope block should be valid for review-only candidates");
    }

    #[test]
    fn qualification_candidate_cannot_use_scope_block() {
        let candidate = CandidateGate {
            target_level: "qualification".to_string(),
            release_gate_impact: "release_blocker".to_string(),
            graduation_gate: "qualification gate".to_string(),
            operator_ids: vec!["solve.example".to_string()],
        };
        let error = validate_review_status_transition("beam-frame", "blocked_scope", &candidate)
            .expect_err("qualification candidate scope block should fail");
        assert!(error.contains("blocked_scope"));
        validate_review_status_transition("beam-frame", "pending_signoff", &candidate)
            .expect("pending signoff should be valid for qualification candidates");
    }

    #[test]
    fn review_decision_requires_reviewer_identity() {
        let decision = json!({
            "schema_version": "kyuubiki.operator-qualification-review-decision/v1",
            "candidate_id": "beam-frame",
            "release_version": "2.0.0",
            "evidence_path": "releases/qualification-evidence/2.0.0/beam-frame.json",
            "review_gate": "gate",
            "decision": "request_changes",
            "reviewer": { "id": "", "display_name": "Reviewer" },
            "reason": "reason",
            "completed_gate_items": ["evidence exists"],
            "requested_changes": ["sign off"],
            "decided_at": "2026-07-15T00:00:00Z"
        });
        let error = validate_review_decision_shape("beam-frame", &decision)
            .expect_err("empty reviewer id should fail");
        assert!(error.contains("reviewer"));
    }

    #[test]
    fn request_changes_decision_requires_requested_changes() {
        let decision = json!({
            "schema_version": "kyuubiki.operator-qualification-review-decision/v1",
            "candidate_id": "beam-frame",
            "release_version": "2.0.0",
            "evidence_path": "releases/qualification-evidence/2.0.0/beam-frame.json",
            "review_gate": "gate",
            "decision": "request_changes",
            "reviewer": { "id": "reviewer", "display_name": "Reviewer" },
            "reason": "reason",
            "completed_gate_items": ["evidence exists"],
            "requested_changes": [],
            "decided_at": "2026-07-15T00:00:00Z"
        });
        let error = validate_review_decision_shape("beam-frame", &decision)
            .expect_err("request_changes without requested_changes should fail");
        assert!(error.contains("request_changes"));
    }

    #[test]
    fn review_decision_path_must_match_record_status() {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-review-decision-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        let decision_path = "review-decision.json";
        fs::create_dir_all(&root).expect("temp directory should be created");
        fs::write(
            root.join(decision_path),
            serde_json::to_string(&json!({
                "schema_version": "kyuubiki.operator-qualification-review-decision/v1",
                "candidate_id": "beam-frame",
                "release_version": "2.0.0",
                "evidence_path": "evidence.json",
                "review_gate": "gate",
                "decision": "approve_promotion",
                "reviewer": { "id": "reviewer", "display_name": "Reviewer" },
                "reason": "ready",
                "completed_gate_items": ["evidence exists"],
                "requested_changes": [],
                "decided_at": "2026-07-15T00:00:00Z"
            }))
            .expect("decision should encode"),
        )
        .expect("decision should write");
        let record = json!({
            "candidate_id": "beam-frame",
            "evidence_path": "evidence.json",
            "review_gate": "gate",
            "review_status": "pending_signoff",
            "review_decision_path": decision_path
        });
        let error = validate_review_decision_path(&root, "2.0.0", &record)
            .expect_err("approve decision should not match pending signoff");
        assert!(error.contains("does not match review_status"));
        fs::remove_dir_all(root).expect("temp directory should be removed");
    }
}
