use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const CHILD: &str =
    "operator_package_generation_session::retirement_tests::crash::retirement_crash_child";

pub(super) fn checkpoint(stage: &str, root: &Path) {
    if std::env::var("KYUUBIKI_RETIRE_TEST_PHASE").as_deref() != Ok(stage) {
        return;
    }
    let target = std::env::var_os("KYUUBIKI_RETIRE_TEST_TARGET").unwrap();
    if root != Path::new(&target) {
        return;
    }
    let ready = PathBuf::from(std::env::var_os("KYUUBIKI_RETIRE_TEST_READY").unwrap());
    let pending = ready.with_extension("pending");
    fs::write(&pending, stage).unwrap();
    fs::rename(pending, ready).unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(60));
    }
}

#[test]
#[ignore = "owned subprocess entrypoint, driven by the retirement crash regression"]
fn retirement_crash_child() {
    let Some(cache) = std::env::var_os("KYUUBIKI_RETIRE_TEST_CACHE") else {
        return;
    };
    let _session = OperatorPackageGenerationSession::open(Path::new(&cache)).unwrap();
    panic!("retirement did not reach the selected checkpoint");
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn killed_retirement_resumes_payload_and_metadata_cleanup_without_replay() {
    use std::os::unix::process::ExitStatusExt;
    for stage in STAGES {
        let fixture = Fixture::new(stage);
        let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let original_peer = fs::read(peer.session_root.join(SESSION_MARKER_FILE)).unwrap();
        let stale = fixture.stale();
        let ready = fixture.root.join("ready");
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", CHILD, "--ignored", "--nocapture"])
                .env("KYUUBIKI_RETIRE_TEST_PHASE", stage)
                .env("KYUUBIKI_RETIRE_TEST_CACHE", &fixture.cache)
                .env("KYUUBIKI_RETIRE_TEST_TARGET", &stale)
                .env("KYUUBIKI_RETIRE_TEST_READY", &ready)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "child exited at {stage}"
            );
            assert!(Instant::now() < deadline, "timeout at {stage}");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(fs::read_to_string(&ready).unwrap(), *stage);
        child.0.kill().unwrap();
        assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
        assert!(fixture.record(&stale).is_file(), "{stage}");
        if *stage == "payload-entry-removed" {
            assert!(!stale.join("generations/first-removable").exists());
            assert!(stale.join("generations/payload/remaining").exists());
        }
        let recovered = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert!(!stale.exists(), "{stage}");
        assert!(!fixture.record(&stale).exists(), "{stage}");
        let report = recovered.janitor_report();
        assert_eq!(report.removed_stale_session_count, 1, "{stage}");
        assert_eq!(report.retained_invalid_session_count, 0, "{stage}");
        assert_eq!(
            fs::read(peer.session_root.join(SESSION_MARKER_FILE)).unwrap(),
            original_peer
        );
        recovered.refresh_stale_sessions().unwrap();
        assert_eq!(recovered.janitor_report(), report, "{stage}");
    }
}
