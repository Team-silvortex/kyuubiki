use super::*;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-retained-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn incomplete_or_source_only_reports_are_not_service_baselines() {
    let root = Fixture::new();
    for report in [
        json!({"suite":"layered", "cases":[], "complete":false}),
        json!({"suite":"layered", "cases":[], "complete":true, "passed_count":14}),
        json!({"suite":"unknown", "cases":[]}),
    ] {
        fs::write(
            root.0.join("report.json"),
            serde_json::to_vec(&report).unwrap(),
        )
        .unwrap();
        assert!(load(&root.0).is_err());
    }
}

#[test]
fn evidence_names_and_digests_are_checked_before_consumption() {
    let root = Fixture::new();
    let bytes = b"{\"value\":12}";
    fs::write(root.0.join("case-result.json"), bytes).unwrap();
    let mut row = json!({"result_file":"case-result.json",
        "result_sha256":format!("{:x}", Sha256::digest(bytes))});
    assert_eq!(
        checked_value(&root.0, &row, "result", "case").unwrap(),
        json!({"value":12})
    );
    row["result_file"] = "../case-result.json".into();
    assert!(checked_value(&root.0, &row, "result", "case").is_err());
    row["result_file"] = "case-result.json".into();
    fs::write(root.0.join("case-result.json"), b"{\"value\":13}").unwrap();
    assert!(checked_value(&root.0, &row, "result", "case").is_err());
}

#[test]
fn oversized_evidence_is_not_loaded_unboundedly() {
    let root = Fixture::new();
    fs::File::create(root.0.join("report.json"))
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    assert!(read(&root.0, "report.json").unwrap_err().contains("8 MiB"));
}

#[cfg(unix)]
#[test]
fn evidence_file_symlinks_are_rejected() {
    let root = Fixture::new();
    fs::write(root.0.join("elsewhere.json"), b"{}").unwrap();
    std::os::unix::fs::symlink("elsewhere.json", root.0.join("report.json")).unwrap();
    assert!(read(&root.0, "report.json").is_err());
}
