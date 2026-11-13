use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const LIMIT: u64 = 96;
const CHILD: &str = "operator_package_generation_session::initialization::io_failures::write_limit::file_limit_child";

struct FileLimit {
    old: libc::rlimit,
    signal: libc::sighandler_t,
}
impl FileLimit {
    fn enter() -> Self {
        let mut old: libc::rlimit = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_FSIZE, &mut old) }, 0);
        let signal = unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) };
        assert_ne!(signal, libc::SIG_ERR);
        let limit = libc::rlimit {
            rlim_cur: LIMIT as libc::rlim_t,
            rlim_max: old.rlim_max,
        };
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &limit) }, 0);
        Self { old, signal }
    }
}
impl Drop for FileLimit {
    fn drop(&mut self) {
        unsafe {
            libc::setrlimit(libc::RLIMIT_FSIZE, &self.old);
            libc::signal(libc::SIGXFSZ, self.signal);
        }
    }
}

#[test]
#[ignore = "owned child entrypoint; resource limit applies only in this test subprocess"]
fn file_limit_child() {
    let Some(cache) = std::env::var_os("KYUUBIKI_SESSION_LIMIT_TEST_CACHE") else {
        return;
    };
    let limit = FileLimit::enter();
    let error = OperatorPackageGenerationSession::open(Path::new(&cache)).unwrap_err();
    drop(limit);
    assert!(
        error.contains("cannot persist initialization staging"),
        "{error}"
    );
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn real_short_write_never_publishes_and_can_be_recovered_without_replay() {
    let fixture = Fixture::new("file-limit");
    let peer = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let peer_marker = fs::read(peer.session_root.join(SESSION_MARKER_FILE)).unwrap();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD, "--ignored", "--nocapture"])
            .env("KYUUBIKI_SESSION_LIMIT_TEST_CACHE", &fixture.cache)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "{status}");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "limited writer failed to terminate"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let entries: Vec<_> = fs::read_dir(fixture.intents())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1);
    assert!(staging::is_staging(&entries[0]));
    assert_eq!(fs::metadata(&entries[0]).unwrap().len(), LIMIT);
    assert_eq!(fs::read_dir(&fixture.sessions).unwrap().count(), 1);
    assert_eq!(
        fs::read(peer.session_root.join(SESSION_MARKER_FILE)).unwrap(),
        peer_marker
    );
    let replacement = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!entries[0].exists());
    assert_eq!(
        replacement
            .janitor_report()
            .removed_initialization_staging_count,
        1
    );
    assert_eq!(
        replacement.janitor_report().retained_invalid_session_count,
        0
    );
    assert!(peer.session_root.exists());
}
