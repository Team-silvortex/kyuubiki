use super::{array, compare_actions, readiness_errors};
use crate::RunnerResult;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::path::Path;

pub(super) fn run_self_test(root: &Path) -> RunnerResult<()> {
    let sample = sample_report();
    if let Some(issue) = readiness_errors(root, &sample, "self-test")?
        .into_iter()
        .next()
    {
        return Err(issue);
    }
    let actions = array(&sample, "next_actions");
    if compare_actions(actions[1], actions[0]) != Ordering::Greater {
        return Err("self-test expected p0 action to sort before p1 action".to_string());
    }
    let mut missing_command = sample.clone();
    if let Some(action) = missing_command
        .get_mut("next_actions")
        .and_then(Value::as_array_mut)
        .and_then(|actions| actions.get_mut(1))
        .and_then(Value::as_object_mut)
    {
        action.remove("command");
    }
    if !readiness_errors(root, &missing_command, "self-test")?
        .iter()
        .any(|error| error.contains("command"))
    {
        return Err("self-test expected missing run_command command to fail".to_string());
    }
    let mut unsorted = sample.clone();
    if let Some(actions) = unsorted
        .get_mut("next_actions")
        .and_then(Value::as_array_mut)
    {
        actions.swap(0, 1);
    }
    if !readiness_errors(root, &unsorted, "self-test")?
        .iter()
        .any(|error| error.contains("sorted"))
    {
        return Err("self-test expected unsorted next_actions to fail".to_string());
    }
    Ok(())
}

fn sample_report() -> Value {
    json!({
        "schema_version": "kyuubiki.operator-qualification-readiness/v1",
        "version_line": "tamamono 1.20.x",
        "generated_at_utc": "2026-01-01T00:00:00.000Z",
        "summary": {
            "candidates": 1,
            "collecting": 0,
            "planned": 1,
            "with_entries": 0,
            "not_started": 1,
            "broken": 0,
            "next_action_count": 2,
            "target_levels": { "baseline": 0, "review": 0, "qualification": 1 },
            "evidence_phases": { "planned": 1, "collecting": 0, "ready_for_review": 0, "blocked": 0 },
            "release_gate_impacts": { "release_blocker": 1, "release_watch": 0, "experimental_only": 0 }
        },
        "candidates": [{
            "candidate_id": "sample",
            "priority": "p0",
            "domain": "sample",
            "target_level": "qualification",
            "evidence_phase": "planned",
            "status": "planned",
            "readiness": "planned",
            "operator_ids": ["solve.sample"],
            "artifact_counts": { "total": 1, "present": 0, "command_available": 0, "missing": 0, "not_started": 1 },
            "artifacts": [],
            "primary_blocker": "sample blocker",
            "evidence_gaps": ["sample"],
            "graduation_gate": "sample gate",
            "preferred_validation_lane": "make sample-validation",
            "release_gate_impact": "release_blocker"
        }],
        "next_actions": [
            {
                "candidate_id": "candidate_a",
                "priority": "p0",
                "target_level": "qualification",
                "evidence_phase": "planned",
                "readiness": "planned",
                "action_kind": "collect_artifact",
                "artifact_id": "note",
                "artifact_state": "not_started",
                "artifact_kind": "reference_note",
                "command": null,
                "path": null,
                "gate": "collect canonical reference note",
                "preferred_validation_lane": "make sample-validation",
                "release_gate_impact": "release_blocker"
            },
            {
                "candidate_id": "candidate_b",
                "priority": "p1",
                "target_level": "review",
                "evidence_phase": "collecting",
                "readiness": "collecting_with_entries",
                "action_kind": "run_command",
                "artifact_id": "release-output",
                "artifact_state": "command_available",
                "artifact_kind": "release_output",
                "command": "make sample-release-evidence",
                "path": null,
                "gate": "retain release evidence",
                "preferred_validation_lane": "make sample-release-evidence",
                "release_gate_impact": "release_watch"
            }
        ]
    })
}
