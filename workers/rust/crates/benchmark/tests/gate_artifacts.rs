use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-benchmark-gate-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn run(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_kyuubiki-benchmark"))
            .current_dir(&self.0)
            .args([
                "--matrix",
                "core",
                "--profile",
                "medium",
                "--repeat",
                "1",
                "--case",
                "axial-bar",
                "--format",
                "json",
            ])
            .args(extra)
            .output()
            .unwrap()
    }

    fn baseline(&self) -> Value {
        let output = self.run(&[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn failed_timing_gate_preserves_current_json_and_comparison_evidence() {
    let fixture = Fixture::new();
    let mut baseline = fixture.baseline();
    baseline["cases"][0]["median_ms"] = json!(1e-9);
    fs::write(
        fixture.0.join("baseline.json"),
        serde_json::to_vec(&baseline).unwrap(),
    )
    .unwrap();
    let output = fixture.run(&[
        "--baseline-compare",
        "baseline.json",
        "--baseline-out",
        "current.json",
        "--compare-report-out",
        "comparison.md",
        "--fail-on-median-regression-pct",
        "25",
        "--min-baseline-median-ms",
        "0",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("median regression"));
    let current: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("current.json"))
            .expect("failed gate must retain current evidence"),
    )
    .unwrap();
    let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(current, stdout);
    assert_eq!(current["cases"][0]["ok"], true);
    assert!(
        fs::read_to_string(fixture.0.join("comparison.md"))
            .unwrap()
            .contains("axial-bar-medium")
    );
}

#[test]
fn missing_baseline_still_preserves_completed_measurement() {
    let fixture = Fixture::new();
    let output = fixture.run(&[
        "--baseline-compare",
        "absent.json",
        "--baseline-out",
        "current.json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to read baseline report"));
    let report: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("current.json"))
            .expect("comparison failure must retain measurement"),
    )
    .unwrap();
    assert_eq!(report["cases"][0]["ok"], true);
}

#[test]
fn shared_input_output_path_cannot_replace_baseline_before_comparison() {
    let fixture = Fixture::new();
    let mut baseline = fixture.baseline();
    baseline["cases"][0]["median_ms"] = json!(1e-9);
    fs::write(
        fixture.0.join("shared.json"),
        serde_json::to_vec(&baseline).unwrap(),
    )
    .unwrap();
    let output = fixture.run(&[
        "--baseline-compare",
        "shared.json",
        "--baseline-out",
        "shared.json",
        "--fail-on-median-regression-pct",
        "25",
        "--min-baseline-median-ms",
        "0",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("median regression"));
    let retained: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("shared.json")).unwrap()).unwrap();
    assert_eq!(
        retained,
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    );
}

#[test]
fn passing_gate_preserves_report_and_zero_exit_status() {
    let fixture = Fixture::new();
    let baseline = fixture.baseline();
    fs::write(
        fixture.0.join("baseline.json"),
        serde_json::to_vec(&baseline).unwrap(),
    )
    .unwrap();
    let output = fixture.run(&[
        "--baseline-compare",
        "baseline.json",
        "--baseline-out",
        "current.json",
        "--fail-on-median-regression-pct",
        "25",
        "--min-baseline-median-ms",
        "1000000",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("current.json")).unwrap()).unwrap();
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    );
}
