use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;

type RunnerResult<T> = Result<T, String>;

mod self_test;

pub(crate) const ROADMAP_SCHEMA_VERSION: &str = "kyuubiki.operator-qualification-roadmap/v1";
pub(crate) const EVIDENCE_KITS_SCHEMA_VERSION: &str =
    "kyuubiki.operator-qualification-evidence-kits/v1";
const ORDERED_LEVELS: &[&str] = &["smoke", "baseline", "review", "qualification"];
const ALLOWED_KIT_STATUSES: &[&str] = &["planned", "collecting", "ready_for_review", "blocked"];

pub(crate) fn run_check_operator_reliability_rules(args: Vec<OsString>) -> RunnerResult<u8> {
    if !args.is_empty() {
        return Err("check-operator-reliability-rules does not accept arguments".to_string());
    }
    let failures = self_test::run_self_test();
    if !failures.is_empty() {
        eprintln!(
            "operator reliability rules self-test failed: {}",
            failures.join("; ")
        );
        return Ok(1);
    }
    println!("operator reliability rules self-test passed");
    Ok(0)
}

pub(super) fn level_rank(level: &str) -> isize {
    ORDERED_LEVELS
        .iter()
        .position(|candidate| *candidate == level)
        .map(|index| index as isize)
        .unwrap_or(-1)
}

pub(crate) fn is_below_minimum_coverage_level(level: &str, minimum_level: &str) -> bool {
    level_rank(level) < level_rank(minimum_level)
}

pub(crate) fn qualification_evidence_errors(entry: &Value) -> Vec<String> {
    let Some(qualification) = entry.pointer("/evidence/qualification") else {
        return vec![
            "qualification-level operators must declare evidence.qualification".to_string(),
        ];
    };
    let mut errors = Vec::new();
    for field in [
        "validation_sources",
        "convergence_checks",
        "provenance",
        "release_gates",
        "tests",
    ] {
        if !non_empty_array(qualification.get(field)) {
            errors.push(format!("evidence.qualification.{field} must be non-empty"));
        }
    }
    errors
}

pub(crate) fn qualification_roadmap_errors<'a>(
    roadmap: &Value,
    manifest: &Value,
    seen_operators: &HashSet<&'a str>,
    operator_levels: &HashMap<&'a str, &'a str>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if field(roadmap, "schema_version") != ROADMAP_SCHEMA_VERSION {
        errors.push("unexpected schema_version".to_string());
    }
    if field(roadmap, "version_line") != field(manifest, "version_line") {
        errors.push("version_line must match reliability manifest".to_string());
    }
    let minimum = field(roadmap, "minimum_candidate_level");
    if !ORDERED_LEVELS.contains(&minimum) {
        errors.push(format!("unknown minimum_candidate_level {minimum}"));
    }
    let manifest_minimum = field(manifest, "minimum_coverage_level");
    if ORDERED_LEVELS.contains(&minimum)
        && ORDERED_LEVELS.contains(&manifest_minimum)
        && is_below_minimum_coverage_level(minimum, manifest_minimum)
    {
        errors.push(format!(
            "minimum_candidate_level {minimum} is below manifest minimum {manifest_minimum}"
        ));
    }
    let Some(candidates) = roadmap.get("candidates").and_then(Value::as_array) else {
        errors.push("candidates must be non-empty".to_string());
        return errors;
    };
    if candidates.is_empty() {
        errors.push("candidates must be non-empty".to_string());
        return errors;
    }
    let mut seen_candidates = HashSet::new();
    for candidate in candidates {
        let candidate_id = field(candidate, "candidate_id");
        let context = if candidate_id.is_empty() {
            "qualification roadmap unknown".to_string()
        } else {
            format!("qualification roadmap {candidate_id}")
        };
        if candidate_id.is_empty() || !seen_candidates.insert(candidate_id.to_string()) {
            errors.push(format!("{context}: candidate_id must be unique"));
            continue;
        }
        for field_name in ["priority", "domain", "rationale", "graduation_gate"] {
            if field(candidate, field_name).is_empty() {
                errors.push(format!("{context}: {field_name} must be non-empty"));
            }
        }
        for field_name in ["operator_ids", "evidence_gaps", "required_artifacts"] {
            if !non_empty_array(candidate.get(field_name)) {
                errors.push(format!("{context}: {field_name} must be non-empty"));
            }
        }
        for operator_id in string_array(candidate.get("operator_ids")) {
            if !seen_operators.contains(operator_id) {
                errors.push(format!(
                    "{context}: operator_id {operator_id} is not in reliability manifest"
                ));
                continue;
            }
            let operator_level = operator_levels
                .get(operator_id)
                .copied()
                .unwrap_or_default();
            if is_below_minimum_coverage_level(operator_level, minimum) {
                errors.push(format!(
                    "{context}: operator_id {operator_id} is below roadmap minimum {minimum}"
                ));
            }
        }
    }
    errors
}

pub(crate) fn qualification_evidence_kit_errors(
    kits: &Value,
    roadmap: &Value,
    manifest: &Value,
) -> Vec<String> {
    let mut errors = Vec::new();
    if field(kits, "schema_version") != EVIDENCE_KITS_SCHEMA_VERSION {
        errors.push("unexpected schema_version".to_string());
    }
    if field(kits, "version_line") != field(manifest, "version_line") {
        errors.push("version_line must match reliability manifest".to_string());
    }
    let Some(kit_items) = kits.get("kits").and_then(Value::as_array) else {
        errors.push("kits must be non-empty".to_string());
        return errors;
    };
    if kit_items.is_empty() {
        errors.push("kits must be non-empty".to_string());
        return errors;
    }
    let roadmap_candidates = roadmap
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|candidate| Some((field(candidate, "candidate_id"), candidate)))
        .collect::<HashMap<_, _>>();
    let mut seen_kits = HashSet::new();
    for kit in kit_items {
        let candidate_id = field(kit, "candidate_id");
        let context = if candidate_id.is_empty() {
            "qualification evidence kit unknown".to_string()
        } else {
            format!("qualification evidence kit {candidate_id}")
        };
        if candidate_id.is_empty() || !seen_kits.insert(candidate_id.to_string()) {
            errors.push(format!("{context}: candidate_id must be unique"));
            continue;
        }
        let roadmap_candidate = roadmap_candidates.get(candidate_id).copied();
        if roadmap_candidate.is_none() {
            errors.push(format!(
                "{context}: candidate_id is not in qualification roadmap"
            ));
        }
        if !ALLOWED_KIT_STATUSES.contains(&field(kit, "status")) {
            errors.push(format!(
                "{context}: unknown status {}",
                field(kit, "status")
            ));
        }
        if field(kit, "artifact_profile").is_empty() {
            errors.push(format!("{context}: artifact_profile must be non-empty"));
        }
        if !non_empty_array(kit.get("operator_ids")) {
            errors.push(format!("{context}: operator_ids must be non-empty"));
        }
        let mut seen_operator_ids = HashSet::new();
        for operator_id in string_array(kit.get("operator_ids")) {
            if !seen_operator_ids.insert(operator_id) {
                errors.push(format!("{context}: duplicate operator_id {operator_id}"));
                continue;
            }
            if roadmap_candidate.is_some_and(|candidate| {
                !string_array(candidate.get("operator_ids")).contains(&operator_id)
            }) {
                errors.push(format!(
                    "{context}: operator_id {operator_id} is not in the roadmap candidate"
                ));
            }
        }
        if let Some(candidate) = roadmap_candidate {
            for operator_id in string_array(candidate.get("operator_ids")) {
                if !string_array(kit.get("operator_ids")).contains(&operator_id) {
                    errors.push(format!(
                        "{context}: missing roadmap operator_id {operator_id}"
                    ));
                }
            }
        }
        validate_artifacts(kit, &context, &mut errors);
    }
    for candidate_id in roadmap_candidates.keys() {
        if !seen_kits.contains(*candidate_id) {
            errors.push(format!(
                "qualification evidence kits: missing kit for roadmap candidate {candidate_id}"
            ));
        }
    }
    errors
}

fn validate_artifacts(kit: &Value, context: &str, errors: &mut Vec<String>) {
    let Some(requirements) = kit.get("artifact_requirements").and_then(Value::as_array) else {
        errors.push(format!(
            "{context}: artifact_requirements must be non-empty"
        ));
        return;
    };
    if requirements.is_empty() {
        errors.push(format!(
            "{context}: artifact_requirements must be non-empty"
        ));
    }
    let status = field(kit, "status");
    let has_collecting_entry = requirements.iter().any(|requirement| {
        !field(requirement, "artifact_path").is_empty()
            || !field(requirement, "artifact_command").is_empty()
    });
    if matches!(status, "collecting" | "ready_for_review") && !has_collecting_entry {
        errors.push(format!(
            "{context}: {status} kits must declare artifact_path or artifact_command"
        ));
    }
    let mut seen_artifacts = HashSet::new();
    for requirement in requirements {
        let artifact_id = field(requirement, "artifact_id");
        if !seen_artifacts.insert(artifact_id) {
            errors.push(format!("{context}: duplicate artifact_id {artifact_id}"));
            continue;
        }
        for field_name in ["artifact_id", "kind", "gate", "path_policy"] {
            if field(requirement, field_name).is_empty() {
                errors.push(format!(
                    "{context}: artifact_requirements.{field_name} must be non-empty"
                ));
            }
        }
        for field_name in ["artifact_path", "artifact_command"] {
            if requirement.get(field_name).is_some() && field(requirement, field_name).is_empty() {
                errors.push(format!(
                    "{context}: artifact_requirements.{field_name} must be non-empty when set"
                ));
            }
        }
        let artifact_path = field(requirement, "artifact_path");
        if !artifact_path.is_empty()
            && (artifact_path.starts_with('/') || artifact_path.contains(".."))
        {
            errors.push(format!(
                "{context}: artifact_path must be repo-relative and stay inside the repo"
            ));
        }
    }
}

fn non_empty_array(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
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

#[cfg(test)]
mod tests {
    use super::{is_below_minimum_coverage_level, level_rank};

    #[test]
    fn levels_are_ordered() {
        assert!(level_rank("smoke") < level_rank("baseline"));
        assert!(is_below_minimum_coverage_level("baseline", "review"));
        assert!(!is_below_minimum_coverage_level("qualification", "review"));
    }
}
