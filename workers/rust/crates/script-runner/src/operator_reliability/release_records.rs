use super::{RunnerResult, array, ensure_file_contains, field, read_json};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const RELEASE_RECORDS_PATH: &str = "releases/qualification-records/1.20.0.json";

pub(super) fn validate_qualification_release_records(
    root: &Path,
    roadmap: &Value,
    kits: &Value,
) -> RunnerResult<()> {
    let records = read_json(root, RELEASE_RECORDS_PATH)?;
    if field(&records, "schema_version") != "kyuubiki.operator-qualification-release-records/v1" {
        return Err("qualification release records: unexpected schema_version".to_string());
    }
    ensure_file_contains(
        root,
        field(&records, "snapshot_path"),
        None,
        "qualification release records",
    )?;
    let candidates = array(roadmap, "candidates")
        .into_iter()
        .map(|candidate| field(candidate, "candidate_id").to_string())
        .collect::<HashSet<_>>();
    let mut requirements = HashMap::new();
    for kit in array(kits, "kits") {
        for requirement in array(kit, "artifact_requirements") {
            if field(requirement, "kind") == "release_retained_regression_output" {
                requirements.insert(field(kit, "candidate_id").to_string(), requirement);
            }
        }
    }
    for record in array(&records, "records") {
        let candidate_id = field(record, "candidate_id");
        if !candidates.contains(candidate_id) {
            return Err(format!(
                "qualification release records: unknown candidate {candidate_id}"
            ));
        }
        let Some(requirement) = requirements.get(candidate_id) else {
            return Err(format!(
                "qualification release records: {candidate_id} has no release-retained requirement"
            ));
        };
        if field(record, "capture_command") != field(requirement, "artifact_command") {
            return Err(format!(
                "qualification release records: {candidate_id} capture_command must match evidence kit"
            ));
        }
        if field(record, "check_command") != field(requirement, "artifact_check_command") {
            return Err(format!(
                "qualification release records: {candidate_id} check_command must match evidence kit"
            ));
        }
    }
    Ok(())
}
