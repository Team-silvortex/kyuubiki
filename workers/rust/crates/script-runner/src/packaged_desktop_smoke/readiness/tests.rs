use super::super::{ChildGuard, RECEIPT_SCHEMA, VERSION, create_receipt_dir};
use super::*;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    child: ChildGuard,
    root: PathBuf,
    path: PathBuf,
    pid: u32,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let name = format!(
            "readiness-test-{}",
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        );
        let root = create_receipt_dir(&name).unwrap();
        let path = root.join("hub.json");
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "packaged_desktop_smoke::readiness::tests::child_fixture",
                "--nocapture",
            ])
            .env("KYUUBIKI_READINESS_TEST_CHILD", mode)
            .env("KYUUBIKI_READINESS_TEST_PATH", &path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        Self {
            child: ChildGuard(Some(child)),
            root,
            path,
            pid,
        }
    }

    fn poll(&mut self, deadline: Instant) -> RunnerResult<BootReceipt> {
        wait_for_receipt(
            self.child.child_mut().unwrap(),
            &self.path,
            "hub",
            Some(self.pid),
            deadline,
        )
    }

    fn write_receipt(&self) {
        fs::write(&self.path, receipt(self.pid)).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.child.stop();
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn receipt(pid: u32) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": RECEIPT_SCHEMA, "surface": "hub", "version": VERSION, "pid": pid
    }))
    .unwrap()
}

// Execute this fixture in a child test process so the lifecycle checks work on
// Windows and Unix without shell scripts or global environment mutation.
#[test]
fn child_fixture() {
    let Ok(mode) = std::env::var("KYUUBIKI_READINESS_TEST_CHILD") else {
        return;
    };
    if mode == "exit" {
        return;
    }
    let path = PathBuf::from(std::env::var_os("KYUUBIKI_READINESS_TEST_PATH").unwrap());
    if mode == "delayed" {
        thread::sleep(Duration::from_millis(250));
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, receipt(std::process::id())).unwrap();
        fs::rename(temporary, path).unwrap();
    }
    thread::sleep(Duration::from_secs(30));
}

#[test]
fn desktop_readiness_rejects_receipt_from_an_exited_child() {
    let mut fixture = Fixture::new("exit");
    fixture.child.child_mut().unwrap().wait().unwrap();
    fixture.write_receipt();
    let error = fixture
        .poll(Instant::now() + Duration::from_secs(3))
        .unwrap_err();
    assert!(error.contains("exited before readiness"), "{error}");
}

#[test]
fn desktop_readiness_rejects_receipt_after_deadline() {
    let mut fixture = Fixture::new("hold");
    fixture.write_receipt();
    let error = fixture.poll(Instant::now()).unwrap_err();
    assert!(error.contains("deadline"), "{error}");
}

#[test]
fn desktop_readiness_accepts_delayed_live_receipt_without_relaunch() {
    let mut fixture = Fixture::new("delayed");
    let report = fixture
        .poll(Instant::now() + Duration::from_secs(5))
        .unwrap();
    assert_eq!(report.pid, fixture.pid);
    assert!(
        fixture
            .child
            .child_mut()
            .unwrap()
            .try_wait()
            .unwrap()
            .is_none()
    );
}

#[test]
fn desktop_readiness_still_rejects_missing_or_malformed_receipts() {
    let mut fixture = Fixture::new("hold");
    let error = fixture
        .poll(Instant::now() + Duration::from_millis(120))
        .unwrap_err();
    assert!(error.contains("deadline"), "{error}");
    fs::write(&fixture.path, b"{}").unwrap();
    let error = fixture
        .poll(Instant::now() + Duration::from_secs(3))
        .unwrap_err();
    assert!(error.contains("invalid boot receipt"), "{error}");
}

#[test]
fn desktop_readiness_requires_original_surface_version_and_pid() {
    let mut fixture = Fixture::new("hold");
    for (key, value) in [
        ("schema_version", json!("unsupported")),
        ("surface", json!("installer")),
        ("version", json!("wrong-version")),
        ("pid", json!(0)),
        ("pid", json!(u32::MAX)),
    ] {
        let mut invalid: serde_json::Value = serde_json::from_slice(&receipt(fixture.pid)).unwrap();
        invalid[key] = value;
        fs::write(&fixture.path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(
            fixture
                .poll(Instant::now() + Duration::from_secs(3))
                .is_err(),
            "accepted {key}"
        );
    }
}

#[test]
fn desktop_readiness_ci_budget_is_explicit_and_bounded() {
    use super::super::parse_options;
    use std::ffi::OsString;
    let root = Path::new("repo");
    let args = |seconds: &str| {
        ["windows", "--install-nsis", "--timeout-secs", seconds]
            .map(OsString::from)
            .to_vec()
    };
    let options = parse_options(root, args("90")).unwrap();
    assert_eq!(options.timeout_secs, 90);
    assert!(options.install_nsis);
    assert_eq!(
        parse_options(root, vec![OsString::from("windows")])
            .unwrap()
            .timeout_secs,
        25
    );
    for invalid in ["0", "301", "-1", "infinite"] {
        assert!(parse_options(root, args(invalid)).is_err());
    }
}
