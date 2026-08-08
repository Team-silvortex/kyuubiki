use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::RunnerResult;
use crate::desktop::{Platform, host_platform};
use crate::native_time::utc_iso_timestamp;

const RECEIPT_ENV: &str = "KYUUBIKI_PACKAGED_BOOT_RECEIPT";
const RECEIPT_SCHEMA: &str = "kyuubiki.packaged-desktop-boot-receipt/v1";
const REPORT_SCHEMA: &str = "kyuubiki.packaged-desktop-smoke-report/v1";
const DEFAULT_TIMEOUT_SECS: u64 = 25;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct BootReceipt {
    schema_version: String,
    surface: String,
    version: String,
    pid: u32,
}

#[derive(Debug, Serialize)]
struct SurfaceResult {
    surface: &'static str,
    app_path: String,
    executable_path: String,
    log_path: String,
    status: &'static str,
    elapsed_ms: u128,
    pid: Option<u32>,
    detail: String,
}

#[derive(Debug, Serialize)]
struct SmokeReport {
    schema_version: &'static str,
    generated_at: String,
    platform: &'static str,
    bundle_root: String,
    expected_version: &'static str,
    timeout_secs: u64,
    status: &'static str,
    passed: usize,
    failed: usize,
    surfaces: Vec<SurfaceResult>,
}

struct SmokeOptions {
    timeout_secs: u64,
    output_path: PathBuf,
    bundle_root: PathBuf,
    verify_report: Option<PathBuf>,
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn child_mut(&mut self) -> RunnerResult<&mut Child> {
        self.0
            .as_mut()
            .ok_or_else(|| "packaged desktop child is unavailable".to_string())
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) fn run_packaged_desktop_smoke(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = parse_options(root, args)?;
    if let Some(report_path) = &options.verify_report {
        verify_retained_report(report_path)?;
        println!(
            "packaged desktop smoke report passed: {}",
            report_path.display()
        );
        return Ok(0);
    }
    if host_platform() != Platform::Macos {
        return Err(
            "desktop-packaged-smoke currently requires a macOS host; Linux and Windows host probes remain release blockers"
                .to_string(),
        );
    }
    let log_root = root.join("tmp/packaged-desktop-smoke");
    fs::create_dir_all(&log_root)
        .map_err(|error| format!("failed to create {}: {error}", log_root.display()))?;

    let mut surfaces = Vec::new();
    for definition in surface_definitions() {
        surfaces.push(run_surface(
            root,
            &options.bundle_root,
            &log_root,
            definition,
            options.timeout_secs,
        ));
    }

    let passed = surfaces
        .iter()
        .filter(|result| result.status == "pass")
        .count();
    let failed = surfaces.len() - passed;
    let report = SmokeReport {
        schema_version: REPORT_SCHEMA,
        generated_at: utc_iso_timestamp(),
        platform: "macos",
        bundle_root: portable_path(root, &options.bundle_root),
        expected_version: VERSION,
        timeout_secs: options.timeout_secs,
        status: if failed == 0 { "pass" } else { "fail" },
        passed,
        failed,
        surfaces,
    };
    write_report(&options.output_path, &report)?;
    println!(
        "packaged desktop smoke: {} passed, {} failed; report {}",
        passed,
        failed,
        options.output_path.display()
    );
    Ok(if failed == 0 { 0 } else { 1 })
}

#[derive(Clone, Copy)]
struct SurfaceDefinition {
    surface: &'static str,
    product_name: &'static str,
    executable_name: &'static str,
}

fn surface_definitions() -> [SurfaceDefinition; 3] {
    [
        SurfaceDefinition {
            surface: "hub",
            product_name: "Kyuubiki Hub",
            executable_name: "kyuubiki-hub-gui",
        },
        SurfaceDefinition {
            surface: "installer",
            product_name: "Kyuubiki Installer",
            executable_name: "kyuubiki-installer-gui",
        },
        SurfaceDefinition {
            surface: "workbench",
            product_name: "Kyuubiki Workbench",
            executable_name: "kyuubiki-workbench-gui",
        },
    ]
}

fn run_surface(
    root: &Path,
    bundle_root: &Path,
    log_root: &Path,
    definition: SurfaceDefinition,
    timeout_secs: u64,
) -> SurfaceResult {
    let app_path = bundle_root.join(format!("{}.app", definition.product_name));
    let executable_path = app_path
        .join("Contents/MacOS")
        .join(definition.executable_name);
    let log_path = log_root.join(format!("{}.log", definition.surface));
    let started = Instant::now();
    let outcome = run_surface_inner(
        &executable_path,
        &log_path,
        definition.surface,
        timeout_secs,
    );
    let (status, pid, detail) = match outcome {
        Ok(receipt) => (
            "pass",
            Some(receipt.pid),
            format!(
                "interactive startup receipt accepted for {} {}",
                receipt.surface, receipt.version
            ),
        ),
        Err(error) => ("fail", None, portable_detail(root, bundle_root, &error)),
    };
    println!(
        "{} packaged boot: {} ({})",
        definition.surface, status, detail
    );
    SurfaceResult {
        surface: definition.surface,
        app_path: portable_bundle_path(root, bundle_root, &app_path),
        executable_path: portable_bundle_path(root, bundle_root, &executable_path),
        log_path: portable_path(root, &log_path),
        status,
        elapsed_ms: started.elapsed().as_millis(),
        pid,
        detail,
    }
}

fn run_surface_inner(
    executable_path: &Path,
    log_path: &Path,
    surface: &str,
    timeout_secs: u64,
) -> RunnerResult<BootReceipt> {
    if !executable_path.is_file() {
        return Err(format!(
            "packaged executable is missing: {}; run desktop-build-host first",
            executable_path.display()
        ));
    }
    let receipt_dir = create_receipt_dir(surface)?;
    let receipt_path = receipt_dir.join(format!("{surface}.json"));
    let result = launch_and_wait(
        executable_path,
        log_path,
        &receipt_path,
        surface,
        timeout_secs,
    );
    let _ = fs::remove_dir_all(&receipt_dir);
    result
}

fn launch_and_wait(
    executable_path: &Path,
    log_path: &Path,
    receipt_path: &Path,
    surface: &str,
    timeout_secs: u64,
) -> RunnerResult<BootReceipt> {
    let log = File::create(log_path)
        .map_err(|error| format!("failed to create {}: {error}", log_path.display()))?;
    let stderr = log
        .try_clone()
        .map_err(|error| format!("failed to clone {}: {error}", log_path.display()))?;
    let child = Command::new(executable_path)
        .env(RECEIPT_ENV, receipt_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| format!("failed to launch {}: {error}", executable_path.display()))?;
    let spawned_pid = child.id();
    let mut guard = ChildGuard(Some(child));
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        if receipt_path.is_file() {
            let receipt = read_and_validate_receipt(receipt_path, surface, spawned_pid)?;
            guard.stop();
            return Ok(receipt);
        }
        if let Some(status) = guard
            .child_mut()?
            .try_wait()
            .map_err(|error| format!("failed to inspect {surface} process: {error}"))?
        {
            return Err(format!(
                "packaged {surface} exited before readiness with {status}; see {}",
                log_path.display()
            ));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "packaged {surface} did not report readiness within {timeout_secs}s; see {}",
                log_path.display()
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn read_and_validate_receipt(
    path: &Path,
    expected_surface: &str,
    spawned_pid: u32,
) -> RunnerResult<BootReceipt> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let receipt: BootReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid boot receipt {}: {error}", path.display()))?;
    if receipt.schema_version != RECEIPT_SCHEMA {
        return Err(format!(
            "unexpected boot receipt schema: {}",
            receipt.schema_version
        ));
    }
    if receipt.surface != expected_surface {
        return Err(format!(
            "boot receipt surface mismatch: expected {expected_surface}, got {}",
            receipt.surface
        ));
    }
    if receipt.version != VERSION {
        return Err(format!(
            "boot receipt version mismatch: expected {VERSION}, got {}",
            receipt.version
        ));
    }
    if receipt.pid != spawned_pid {
        return Err(format!(
            "boot receipt pid mismatch: expected {spawned_pid}, got {}",
            receipt.pid
        ));
    }
    Ok(receipt)
}

fn create_receipt_dir(surface: &str) -> RunnerResult<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "kyuubiki-packaged-smoke-{}-{nonce}-{surface}",
        std::process::id()
    ));
    fs::create_dir(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    Ok(path)
}

fn parse_options(root: &Path, args: Vec<OsString>) -> RunnerResult<SmokeOptions> {
    let args = args
        .into_iter()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "desktop smoke arguments must be UTF-8".to_string())
        })
        .collect::<RunnerResult<Vec<_>>>()?;
    let mut timeout_secs = DEFAULT_TIMEOUT_SECS;
    let mut output_path = root.join("tmp/packaged-desktop-smoke.json");
    let mut bundle_root = root.join("target/desktop-cache/macos/release/bundle/macos");
    let mut verify_report = None;
    let mut index = 0;
    if args.first().is_some_and(|value| value == "macos") {
        index += 1;
    }
    while index < args.len() {
        match args[index].as_str() {
            "--timeout-secs" => {
                index += 1;
                timeout_secs = args
                    .get(index)
                    .ok_or_else(|| "--timeout-secs requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "--timeout-secs must be a positive integer".to_string())?;
                if timeout_secs == 0 || timeout_secs > 300 {
                    return Err("--timeout-secs must be between 1 and 300".to_string());
                }
            }
            "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--out requires a path".to_string())?;
                let path = PathBuf::from(value);
                output_path = if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                };
            }
            "--bundle-root" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--bundle-root requires a path".to_string())?;
                let path = PathBuf::from(value);
                bundle_root = if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                };
            }
            "--verify-report" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--verify-report requires a path".to_string())?;
                let path = PathBuf::from(value);
                verify_report = Some(if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                });
            }
            argument => {
                return Err(format!(
                    "unknown desktop-packaged-smoke argument: {argument}"
                ));
            }
        }
        index += 1;
    }
    Ok(SmokeOptions {
        timeout_secs,
        output_path,
        bundle_root,
        verify_report,
    })
}

fn verify_retained_report(path: &Path) -> RunnerResult<()> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read retained report {}: {error}", path.display()))?;
    let report: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid retained report {}: {error}", path.display()))?;
    validate_retained_report(&report)
}

fn validate_retained_report(report: &Value) -> RunnerResult<()> {
    for (pointer, expected) in [
        ("/schema_version", REPORT_SCHEMA),
        ("/platform", "macos"),
        ("/expected_version", VERSION),
        ("/status", "pass"),
    ] {
        if report.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "retained packaged desktop report {pointer} must be {expected}"
            ));
        }
    }
    if report.get("passed").and_then(Value::as_u64) != Some(3)
        || report.get("failed").and_then(Value::as_u64) != Some(0)
    {
        return Err(
            "retained packaged desktop report must record 3 passed and 0 failed".to_string(),
        );
    }
    validate_report_path(report, "/bundle_root", "@external/Applications")?;
    let surfaces = report
        .get("surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| "retained packaged desktop report must contain surfaces".to_string())?;
    let mut seen = BTreeSet::new();
    for surface in surfaces {
        let id = surface
            .get("surface")
            .and_then(Value::as_str)
            .ok_or_else(|| "retained packaged desktop surface must have an id".to_string())?;
        if !seen.insert(id.to_string()) {
            return Err(format!(
                "retained packaged desktop report repeats surface {id}"
            ));
        }
        if surface.get("status").and_then(Value::as_str) != Some("pass")
            || surface.get("pid").and_then(Value::as_u64).unwrap_or(0) == 0
        {
            return Err(format!(
                "retained packaged desktop surface {id} did not pass"
            ));
        }
        let definition = surface_definitions()
            .into_iter()
            .find(|definition| definition.surface == id)
            .ok_or_else(|| format!("unexpected retained packaged desktop surface {id}"))?;
        let app_path = format!("@bundle-root/{}.app", definition.product_name);
        let executable_path = format!("{app_path}/Contents/MacOS/{}", definition.executable_name);
        let log_path = format!("tmp/packaged-desktop-smoke/{id}.log");
        let detail = format!("interactive startup receipt accepted for {id} {VERSION}");
        validate_report_path(surface, "/app_path", &app_path)?;
        validate_report_path(surface, "/executable_path", &executable_path)?;
        validate_report_path(surface, "/log_path", &log_path)?;
        validate_report_text(surface, "/detail", &detail)?;
    }
    let expected = BTreeSet::from([
        "hub".to_string(),
        "installer".to_string(),
        "workbench".to_string(),
    ]);
    if seen != expected {
        return Err(
            "retained packaged desktop report must cover hub, installer, and workbench".to_string(),
        );
    }
    Ok(())
}

fn validate_report_path(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    let path = value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("retained packaged desktop report misses {pointer}"))?;
    let parsed = Path::new(path);
    if parsed.is_absolute() || parsed.components().any(|part| part.as_os_str() == "..") {
        return Err(format!(
            "retained packaged desktop report path must be portable: {path}"
        ));
    }
    validate_report_text(value, pointer, expected)
}

fn validate_report_text(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_str) != Some(expected) {
        return Err(format!(
            "retained packaged desktop report {pointer} must be {expected}"
        ));
    }
    Ok(())
}

fn write_report(path: &Path, report: &SmokeReport) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("failed to serialize desktop smoke report: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn portable_path(root: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_string_lossy().replace('\\', "/");
    }
    format!(
        "@external/{}",
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .replace('\\', "/")
    )
}

fn portable_bundle_path(root: &Path, bundle_root: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_string_lossy().replace('\\', "/");
    }
    if let Ok(relative) = path.strip_prefix(bundle_root) {
        return format!(
            "@bundle-root/{}",
            relative.to_string_lossy().replace('\\', "/")
        );
    }
    portable_path(root, path)
}

fn portable_detail(root: &Path, bundle_root: &Path, detail: &str) -> String {
    detail
        .replace(&root.to_string_lossy().to_string(), ".")
        .replace(&bundle_root.to_string_lossy().to_string(), "@bundle-root")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_smoke_options() {
        let root = Path::new("/tmp/repo");
        let options = parse_options(
            root,
            [
                OsString::from("macos"),
                OsString::from("--timeout-secs"),
                OsString::from("40"),
                OsString::from("--out"),
                OsString::from("tmp/report.json"),
            ]
            .to_vec(),
        )
        .expect("options should parse");
        assert_eq!(options.timeout_secs, 40);
        assert_eq!(options.output_path, root.join("tmp/report.json"));
        assert_eq!(
            options.bundle_root,
            root.join("target/desktop-cache/macos/release/bundle/macos")
        );
        assert!(options.verify_report.is_none());
    }

    #[test]
    fn rejects_invalid_timeout() {
        let error = parse_options(
            Path::new("/tmp/repo"),
            [OsString::from("--timeout-secs"), OsString::from("0")].to_vec(),
        )
        .err()
        .expect("zero timeout should fail");
        assert!(error.contains("between 1 and 300"));
    }

    #[test]
    fn report_paths_are_portable() {
        let root = Path::new("/tmp/repo");
        assert_eq!(
            portable_path(root, &root.join("tmp/report.json")),
            "tmp/report.json"
        );
        assert_eq!(
            portable_path(root, Path::new("/Applications")),
            "@external/Applications"
        );
        assert_eq!(
            portable_bundle_path(
                root,
                Path::new("/Applications"),
                Path::new("/Applications/Kyuubiki Hub.app/Contents/MacOS/kyuubiki-hub-gui")
            ),
            "@bundle-root/Kyuubiki Hub.app/Contents/MacOS/kyuubiki-hub-gui"
        );
        assert_eq!(
            portable_detail(
                root,
                Path::new("/Applications"),
                "failed at /tmp/repo/target/app"
            ),
            "failed at ./target/app"
        );
    }

    #[test]
    fn validates_portable_retained_report() {
        let mut report = serde_json::json!({
            "schema_version": REPORT_SCHEMA,
            "platform": "macos",
            "bundle_root": "@external/Applications",
            "expected_version": VERSION,
            "status": "pass",
            "passed": 3,
            "failed": 0,
            "surfaces": [
                retained_surface("hub", 1),
                retained_surface("installer", 2),
                retained_surface("workbench", 3)
            ]
        });
        validate_retained_report(&report).expect("portable report should pass");
        report["surfaces"][0]["detail"] = Value::String("startup assumed".to_string());
        let error = validate_retained_report(&report).expect_err("invalid receipt should fail");
        assert!(error.contains("/detail"));
        report["surfaces"][0] = retained_surface("hub", 1);
        report["surfaces"][0]["app_path"] = Value::String("/Applications/Hub.app".to_string());
        let error = validate_retained_report(&report).expect_err("absolute path should fail");
        assert!(error.contains("must be portable"));
    }

    fn retained_surface(surface: &str, pid: u64) -> Value {
        let definition = surface_definitions()
            .into_iter()
            .find(|definition| definition.surface == surface)
            .expect("fixture surface should exist");
        let app_path = format!("@bundle-root/{}.app", definition.product_name);
        serde_json::json!({
            "surface": surface,
            "app_path": app_path,
            "executable_path": format!(
                "@bundle-root/{}.app/Contents/MacOS/{}",
                definition.product_name,
                definition.executable_name
            ),
            "log_path": format!("tmp/packaged-desktop-smoke/{surface}.log"),
            "status": "pass",
            "pid": pid,
            "detail": format!("interactive startup receipt accepted for {surface} {VERSION}")
        })
    }
}
