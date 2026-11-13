use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const CHILD: &str =
    "operator_package_generation::cleanup_tests::crash::generation_cleanup_crash_child";

pub(super) fn checkpoint(stage: &str, root: &Path) {
    if std::env::var("KYUUBIKI_GENERATION_CLEANUP_TEST_PHASE").as_deref() != Ok(stage) {
        return;
    }
    let ready = PathBuf::from(std::env::var_os("KYUUBIKI_GENERATION_CLEANUP_TEST_READY").unwrap());
    let pending = ready.with_extension("pending");
    fs::write(&pending, root.as_os_str().as_encoded_bytes()).unwrap();
    fs::rename(pending, ready).unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(60));
    }
}

#[test]
#[ignore = "owned subprocess entrypoint, driven by generation cleanup crash regression"]
fn generation_cleanup_crash_child() {
    let Some(cache) = std::env::var_os("KYUUBIKI_GENERATION_CLEANUP_TEST_CACHE") else {
        return;
    };
    let root = PathBuf::from(cache);
    let session = OperatorPackageGenerationSession::open(&root).unwrap();
    let generation = prepare_fixture(&root, session).commit();
    remove_owned_operator_package_generation(&generation).unwrap();
    panic!("generation cleanup missed selected barrier");
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn killed_generation_cleanup_is_reclaimed_without_replay_or_live_peer_deletion() {
    use std::os::unix::{ffi::OsStringExt, process::ExitStatusExt};
    for stage in STAGES {
        let fixture = Fixture::new(stage);
        let peer = fixture.prepared().commit();
        let peer_marker = fs::read(peer.generation_root.join(GENERATION_MARKER_FILE)).unwrap();
        let ready = fixture.root.join("ready");
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", CHILD, "--ignored", "--nocapture"])
                .env("KYUUBIKI_GENERATION_CLEANUP_TEST_CACHE", &fixture.root)
                .env("KYUUBIKI_GENERATION_CLEANUP_TEST_PHASE", stage)
                .env("KYUUBIKI_GENERATION_CLEANUP_TEST_READY", &ready)
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
        let root = PathBuf::from(std::ffi::OsString::from_vec(fs::read(&ready).unwrap()));
        child.0.kill().unwrap();
        assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
        let journal = cleanup::record_path(&root).unwrap();
        assert!(journal.is_file(), "{stage}");
        if *stage == "payload-entry-removed" {
            assert!(!root.join("first-removable").exists());
            assert!(root.join("packages/locked/remaining").exists());
        }
        let child_session = root.parent().unwrap().parent().unwrap();
        fixture.session.refresh_stale_sessions().unwrap();
        assert!(!child_session.exists(), "{stage}");
        assert!(!journal.exists(), "{stage}");
        assert_eq!(
            fixture.session.janitor_report().removed_stale_session_count,
            1,
            "{stage}"
        );
        assert_eq!(
            fs::read(peer.generation_root.join(GENERATION_MARKER_FILE)).unwrap(),
            peer_marker
        );
        assert_eq!(
            fs::read(peer.generation_root.join("packages/locked/remaining")).unwrap(),
            b"keep"
        );
        fixture.session.refresh_stale_sessions().unwrap();
        assert_eq!(
            fixture.session.janitor_report().removed_stale_session_count,
            1
        );
    }
}
