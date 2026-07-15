use crate::RunnerResult;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use super::{array, field, read_json};

const RELEASE_RECORDS_PATH: &str = "releases/qualification-records/1.20.0.json";

#[derive(Debug)]
pub(super) struct ReleaseRecord {
    pub(super) status: String,
    pub(super) capture_command: String,
    pub(super) evidence_path: String,
}

pub(super) fn release_records_by_candidate(
    root: &Path,
) -> RunnerResult<HashMap<String, ReleaseRecord>> {
    if !root.join(RELEASE_RECORDS_PATH).exists() {
        return Ok(HashMap::new());
    }
    let records = read_json(root, RELEASE_RECORDS_PATH)?;
    Ok(array(&records, "records")
        .into_iter()
        .map(record_from_value)
        .collect())
}

fn record_from_value(record: &Value) -> (String, ReleaseRecord) {
    (
        field(record, "candidate_id").to_string(),
        ReleaseRecord {
            status: field(record, "status").to_string(),
            capture_command: field(record, "capture_command").to_string(),
            evidence_path: field(record, "evidence_path").to_string(),
        },
    )
}
