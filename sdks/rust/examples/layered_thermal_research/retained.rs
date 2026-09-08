use super::suite::{self, Case};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, io::Read, path::Path};

#[cfg(test)]
#[path = "retained_tests.rs"]
mod tests;

pub struct RetainedCase {
    pub case: Case,
    pub job_id: String,
    pub result: Value,
}

pub struct Baseline {
    pub report_sha256: String,
    pub cases: Vec<RetainedCase>,
}

fn read(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    let path = root.join(name);
    if !fs::symlink_metadata(&path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err(format!(
            "evidence must be a regular, non-symlink file: {name}"
        ));
    }
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("evidence exceeds the bounded example's 8 MiB file limit".into());
    }
    Ok(bytes)
}

fn checked_value(root: &Path, row: &Value, kind: &str, id: &str) -> Result<Value, String> {
    // Derive filenames from the built-in case list, never from an external path.
    let name = format!("{id}-{kind}.json");
    if row[format!("{kind}_file")] != name {
        return Err(format!("{id}: unexpected {kind} filename"));
    }
    let bytes = read(root, &name)?;
    if row[format!("{kind}_sha256")] != format!("{:x}", Sha256::digest(&bytes)) {
        return Err(format!("{id}: retained {kind} digest mismatch"));
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn load(root: &Path) -> Result<Baseline, String> {
    let bytes = read(root, "report.json")?;
    let report: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let cases = suite::cases(report["suite"].as_str().ok_or("missing suite")?)?;
    let rows = report["cases"].as_array().ok_or("missing case rows")?;
    if report["schema_version"] != "kyuubiki.layered-thermal-research/v1"
        || report["execution"] != "official_rust_headless_sdk_service"
        || report["complete"] != true
        || report["failed_count"] != 0
        || report["expected_case_count"] != cases.len()
        || report["case_count"] != cases.len()
        || report["passed_count"] != cases.len()
        || rows.len() != cases.len()
    {
        return Err("baseline must be a complete successful SDK service round".into());
    }
    let mut jobs = HashSet::new();
    let mut retained = vec![];
    for (case, row) in cases.into_iter().zip(rows) {
        let id = case.id();
        if row["case"] != id
            || row["passed"] != true
            || row["terminal"]["status"] != "completed"
            || row["result_readback_matches"] != true
            || row["output_manifest_validated"] != true
        {
            return Err(format!("{id}: baseline identity or gates are not complete"));
        }
        let job_id = row["terminal"]["job_id"]
            .as_str()
            .ok_or("missing job identity")?;
        if job_id.is_empty()
            || job_id.len() > 128
            || !job_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !jobs.insert(job_id.to_string())
        {
            return Err(format!("{id}: invalid or duplicate job identity"));
        }
        let (graph, inputs) = case.workflow();
        if checked_value(root, row, "request", &id)?
            != json!({"graph":graph,"input_artifacts":inputs})
        {
            return Err(format!(
                "{id}: retained request differs from this example's model"
            ));
        }
        let result = checked_value(root, row, "result", &id)?;
        if case.validate(&result)?["passed"] != true {
            return Err(format!(
                "{id}: retained numerical result fails independent gates"
            ));
        }
        retained.push(RetainedCase {
            case,
            job_id: job_id.to_string(),
            result,
        });
    }
    Ok(Baseline {
        report_sha256: format!("{:x}", Sha256::digest(&bytes)),
        cases: retained,
    })
}
