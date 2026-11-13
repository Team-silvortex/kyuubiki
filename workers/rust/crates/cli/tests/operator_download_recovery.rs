#![cfg(unix)]
#[path = "support/operator_download_fixture.rs"]
mod download;
#[path = "support/operator_package.rs"]
mod support;

use download::{DownloadFixture, PACKAGE, PREFIX};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use support::{
    LiveAgent, external_operator_task, refresh_task_digest, template_library_path,
    template_packages_root,
};

#[derive(Clone, Copy, Debug)]
enum Interruption {
    ParentLoss,
    Cancel,
    Disconnect,
}

#[test]
#[ignore = "transfers and verifies a complete 64 MiB artifact without executing its fixture bytes"]
fn complete_large_artifact_is_verified_before_installation() {
    let root = Cleanup(std::env::temp_dir().join(format!(
        "kyuubiki-download-recovery-full-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    let fixture = DownloadFixture::complete_large();
    let receipt = kyuubiki_installer::fetch_operator_package_into(
        &fixture.url,
        PACKAGE,
        "0.1.0",
        &root.0,
        None,
    )
    .unwrap();
    assert_eq!(receipt.entrypoint_sha256, fixture.digest);
    kyuubiki_installer::verify_managed_operator_package(&root.0.join(&receipt.relative_root))
        .unwrap();
    assert_eq!(fixture.request_count(), 3);
}

#[test]
#[ignore = "streams a large artifact and SIGKILLs only an owned test Agent; requires template cdylib"]
fn killed_agent_download_is_reaped_on_restart_without_replay() {
    exercise(Interruption::ParentLoss);
}

#[test]
#[ignore = "streams a large artifact through a real Agent; requires template cdylib"]
fn cancelled_large_download_removes_real_partial_file() {
    exercise(Interruption::Cancel);
}

#[test]
#[ignore = "streams and truncates a large artifact; requires template cdylib"]
fn truncated_large_download_cannot_publish_and_can_be_explicitly_retried() {
    exercise(Interruption::Disconnect);
}

#[test]
#[ignore = "pauses an owned downloader, kills its Agent and restarts against the same cache"]
fn restart_preserves_suspended_downloader_then_reaps_it_before_explicit_fetch() {
    let root = Cleanup(std::env::temp_dir().join(format!(
        "kyuubiki-download-recovery-paused-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    let packages = root.0.join("store/packages");
    fs::create_dir_all(&packages).unwrap();
    let fixture = DownloadFixture::large();
    let agent = LiveAgent::start_orchestrated(&packages, &fixture.url, "test-token").unwrap();
    let sessions = root.0.join("store/agent-runtime-generations/sessions");
    let stale = directories(&sessions).pop().unwrap();
    let healthy = DownloadFixture::complete(
        fs::read(template_library_path(&template_packages_root())).unwrap(),
    );
    thread::scope(|scope| {
        let work = scope.spawn(|| {
            agent.request("paused-download", "run_operator_task_ir", json!({
            "mode":"execute", "job_id":"paused-download", "task_ir":task(&fixture.digest)
        })).map_err(|error| error.to_string())
        });
        let _parent_guard = KillOnDrop(agent.process_id());
        fixture
            .entered
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        let partial = wait_partial(&sessions);
        let pid = downloader_pid(&partial);
        // Prevent POSIX orphan-group SIGHUP/SIGCONT from silently resuming this process.
        let _anchor = GroupAnchor::start(pid);
        let mut paused = PausedDownloader { pid, armed: true };
        assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let state = std::process::Command::new("ps")
                .args(["-o", "stat=", "-p", &pid.to_string()])
                .output()
                .unwrap();
            if String::from_utf8_lossy(&state.stdout).contains('T') {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "owned downloader did not enter stopped state"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            unsafe { libc::kill(agent.process_id() as i32, libc::SIGKILL) },
            0
        );
        assert!(work.join().unwrap().is_err());
        let lease = fs::File::open(stale.join("kyuubiki-agent-session.lease")).unwrap();
        assert!(
            fs2::FileExt::try_lock_exclusive(&lease).is_err(),
            "suspended child did not retain its session lease"
        );
        drop(lease);
        assert!(
            partial.exists(),
            "partial disappeared before replacement startup"
        );
        let replacement =
            LiveAgent::start_orchestrated(&packages, &healthy.url, "test-token").unwrap();
        assert!(
            partial.exists(),
            "restart deleted an orphan downloader's in-use file; stale={}, process={:?}",
            stale.exists(),
            std::process::Command::new("ps")
                .args(["-o", "pid=,ppid=,stat=,comm=", "-p", &pid.to_string()])
                .output()
                .unwrap()
        );
        assert_eq!(fs::metadata(&partial).unwrap().len(), PREFIX as u64);
        assert_eq!(
            healthy.request_count(),
            0,
            "restart must not replay download"
        );
        let peer = LiveAgent::start_orchestrated(&packages, &healthy.url, "test-token").unwrap();
        let live_sessions: Vec<_> = directories(&sessions)
            .into_iter()
            .filter(|path| path != &stale)
            .collect();
        assert_eq!(live_sessions.len(), 2);
        assert_eq!(unsafe { libc::kill(pid, libc::SIGCONT) }, 0);
        paused.armed = false;
        fixture.closed.recv_timeout(Duration::from_secs(4)).unwrap();
        assert_downloader_gone(pid);
        assert!(
            partial.exists(),
            "cleanup is deferred to explicit generation preparation"
        );
        let reply = replacement
            .request(
                "explicit-after-orphan",
                "run_operator_task_ir",
                json!({
                    "mode":"execute", "job_id":"paused-download", "task_ir":task(&healthy.digest)
                }),
            )
            .unwrap();
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["result"]["result"]["summary"]["sum"], 14.0);
        assert!(!stale.exists(), "deferred orphan session was never reaped");
        assert!(
            live_sessions.iter().all(|path| path.exists()),
            "cleanup removed a live peer"
        );
        let janitor = &reply["result"]["operator_package_execution"]["cache_generation"]["janitor"];
        assert_eq!(janitor["removed_stale_session_count"], 1);
        assert_eq!(janitor["retained_active_session_count"], 1);
        assert_eq!(janitor["retained_invalid_session_count"], 0);
        assert_eq!(fixture.request_count(), 3);
        assert_eq!(healthy.request_count(), 3);
        assert_eq!(
            peer.request("peer-alive", "describe_agent", json!({}))
                .unwrap()["ok"],
            true
        );
    });
}

#[test]
#[ignore = "starts four independent Agents concurrently over one owned package cache"]
fn concurrent_agents_claim_stale_cleanup_once_and_execute_independently() {
    let root = Cleanup(std::env::temp_dir().join(format!(
        "kyuubiki-download-recovery-concurrent-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    let packages = root.0.join("store/packages");
    fs::create_dir_all(&packages).unwrap();
    let fixture = DownloadFixture::complete(
        fs::read(template_library_path(&template_packages_root())).unwrap(),
    );
    let seed = LiveAgent::start_orchestrated(&packages, &fixture.url, "test-token").unwrap();
    let sessions = root.0.join("store/agent-runtime-generations/sessions");
    let stale = directories(&sessions).pop().unwrap();
    drop(seed);
    assert!(stale.exists());
    let ports: Vec<_> = (0..4)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    let barrier = std::sync::Barrier::new(4);
    let agents: Vec<_> = thread::scope(|scope| {
        let workers: Vec<_> = ports
            .into_iter()
            .map(|port| {
                let packages = &packages;
                let url = &fixture.url;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    LiveAgent::start_orchestrated_reserved(packages, url, "test-token", port)
                        .map_err(|e| e.to_string())
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap())
            .collect()
    });
    assert!(!stale.exists());
    assert_eq!(directories(&sessions).len(), 4);
    assert_eq!(
        fixture.request_count(),
        0,
        "startup must never dispatch downloads"
    );
    let replies: Vec<_> = thread::scope(|scope| {
        let workers: Vec<_> = agents
            .iter()
            .enumerate()
            .map(|(index, agent)| {
                let digest = &fixture.digest;
                scope.spawn(move || {
                    agent.request("independent-fetch", "run_operator_task_ir", json!({
                "mode":"execute", "job_id":format!("concurrent-{index}"), "task_ir":task(digest)
            })).map_err(|e| e.to_string())
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap())
            .collect()
    });
    let mut removed = 0;
    for reply in replies {
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["result"]["result"]["summary"]["sum"], 14.0);
        let janitor = &reply["result"]["operator_package_execution"]["cache_generation"]["janitor"];
        removed += janitor["removed_stale_session_count"].as_u64().unwrap();
        assert_eq!(janitor["retained_active_session_count"], 3);
        assert_eq!(janitor["retained_invalid_session_count"], 0);
    }
    assert_eq!(removed, 1);
    assert_eq!(fixture.request_count(), 12);
    assert_eq!(directories(&sessions).len(), 4);
}

fn exercise(interruption: Interruption) {
    let root = Cleanup(std::env::temp_dir().join(format!(
        "kyuubiki-download-recovery-{}-{interruption:?}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    let packages = root.0.join("store/packages");
    fs::create_dir_all(&packages).unwrap();
    let fixture = DownloadFixture::large();
    let agent =
        LiveAgent::start_orchestrated(&packages, &fixture.url, "test-download-token").unwrap();
    let sessions = root.0.join("store/agent-runtime-generations/sessions");
    let old_sessions = directories(&sessions);
    assert_eq!(old_sessions.len(), 1);
    thread::scope(|scope| {
        let work = scope.spawn(|| {
            agent.request("interrupted", "run_operator_task_ir", json!({
            "mode":"execute", "job_id":"download-recovery", "task_ir": task(&fixture.digest)
        })).map_err(|error| error.to_string())
        });
        // On any assertion failure, kill this owned Agent before the scoped request joins.
        let guard = KillOnDrop(agent.process_id());
        fixture
            .entered
            .recv_timeout(Duration::from_secs(10))
            .expect("real entrypoint transfer reached barrier");
        let partial = wait_partial(&sessions);
        let downloader_pid = downloader_pid(&partial);
        assert_ne!(downloader_pid as u32, agent.process_id());
        assert_eq!(fs::metadata(&partial).unwrap().len(), PREFIX as u64);
        assert!(fs::read_dir(&packages).unwrap().next().is_none());
        match interruption {
            Interruption::ParentLoss => {
                assert_eq!(
                    unsafe { libc::kill(agent.process_id() as i32, libc::SIGKILL) },
                    0
                );
                assert!(work.join().unwrap().is_err());
                fixture
                    .closed
                    .recv_timeout(Duration::from_secs(4))
                    .expect("orphan downloader must close HTTP before fixture release");
                assert!(
                    partial.exists(),
                    "SIGKILL leaves real cleanup work for restart"
                );
            }
            other => {
                if matches!(other, Interruption::Cancel) {
                    let cancel = agent
                        .request(
                            "cancel",
                            "cancel_job",
                            json!({"job_id":"download-recovery"}),
                        )
                        .unwrap();
                    assert_eq!(cancel["ok"], true, "{cancel}");
                } else {
                    fixture.disconnect();
                }
                let reply = work.join().unwrap().expect("terminal download receipt");
                assert_eq!(reply["ok"], false, "{reply}");
                assert_eq!(
                    reply["error"]["code"],
                    if matches!(other, Interruption::Cancel) {
                        "cancelled"
                    } else {
                        "operator_package_fetch_failed"
                    },
                    "{reply}"
                );
                assert_eq!(
                    reply["error"]["details"]["operator_task_failure_receipt"]["failure_stage"],
                    "fetch_package"
                );
                assert!(!partial.exists());
                assert!(directories(&old_sessions[0].join("generations")).is_empty());
                let describe = agent
                    .request("responsive", "describe_agent", json!({}))
                    .unwrap();
                assert_eq!(describe["ok"], true);
                if matches!(other, Interruption::Cancel) {
                    fixture.closed.recv_timeout(Duration::from_secs(4)).unwrap();
                }
            }
        }
        assert_downloader_gone(downloader_pid);
        assert_eq!(fixture.request_count(), 3, "no hidden transfer retry");
        drop(guard);
    });
    drop(agent);
    let bytes = fs::read(template_library_path(&template_packages_root())).unwrap();
    let healthy = DownloadFixture::complete(bytes);
    let restarted =
        LiveAgent::start_orchestrated(&packages, &healthy.url, "test-download-token").unwrap();
    assert!(
        old_sessions.iter().all(|path| !path.exists()),
        "startup must remove stale session before a new task"
    );
    assert_eq!(
        healthy.request_count(),
        0,
        "restart must not replay interrupted download"
    );
    assert!(fs::read_dir(&packages).unwrap().next().is_none());
    let reply = restarted
        .request(
            "explicit-retry",
            "run_operator_task_ir",
            json!({
                "mode":"execute", "job_id":"download-recovery", "task_ir":task(&healthy.digest)
            }),
        )
        .unwrap();
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["result"]["result"]["summary"]["sum"], 14.0);
    assert_eq!(
        reply["result"]["operator_package_execution"]["cache_status"],
        "fetched_and_activated"
    );
    assert_eq!(
        reply["result"]["operator_package_execution"]["cache_generation"]["janitor"]["removed_stale_session_count"],
        1
    );
    assert_eq!(healthy.request_count(), 3);
    assert_eq!(fixture.request_count(), 3);
    let release = restarted
        .request(
            "release",
            "release_operator_package_job",
            json!({"job_id":"download-recovery"}),
        )
        .unwrap();
    assert_eq!(release["ok"], true, "{release}");
}

fn task(digest: &str) -> Value {
    let mut task = external_operator_task(digest);
    let reference = format!("orchestra://operator-package/{PACKAGE}");
    task["execution_program"]["package_ref"] = json!(reference);
    task["runtime_hints"]["package_ref"] = json!(reference);
    task["runtime_hints"]["authority_mode"] = json!("central_operator_library");
    task["runtime_hints"]["execution_mode"] = json!("orchestra_fetch");
    task["runtime_hints"]["cache_scope"] = json!("job");
    task["runtime_hints"]["agent_fetchable"] = json!(true);
    refresh_task_digest(&mut task);
    task
}

fn wait_partial(sessions: &Path) -> PathBuf {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(path) = partial_file(sessions) {
            return path;
        }
        assert!(
            Instant::now() < deadline,
            "real download bytes never reached unpublished staging"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn partial_file(root: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        if entry.file_type().ok()?.is_dir() {
            if let Some(path) = partial_file(&entry.path()) {
                return Some(path);
            }
        } else if entry.metadata().ok()?.len() == PREFIX as u64 {
            return Some(entry.path());
        }
    }
    None
}

fn directories(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect()
}

fn downloader_pid(partial: &Path) -> i32 {
    partial
        .ancestors()
        .find_map(|path| {
            path.file_name()?
                .to_str()?
                .strip_prefix("kyuubiki-operator-fetch-")?
                .split('-')
                .next()?
                .parse()
                .ok()
        })
        .expect("partial file belongs to the real downloader process")
}

fn assert_downloader_gone(pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while unsafe { libc::kill(pid, 0) } == 0 {
        assert!(
            Instant::now() < deadline,
            "downloader {pid} survived interruption"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

struct PausedDownloader {
    pid: i32,
    armed: bool,
}
impl Drop for PausedDownloader {
    fn drop(&mut self) {
        if self.armed {
            unsafe {
                libc::kill(self.pid, libc::SIGKILL);
            }
        }
    }
}

struct GroupAnchor(std::process::Child);
impl GroupAnchor {
    fn start(group: i32) -> Self {
        use std::os::unix::process::CommandExt;
        let child = std::process::Command::new("sleep")
            .arg("30")
            .process_group(group)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        assert_eq!(unsafe { libc::getpgid(child.id() as i32) }, group);
        Self(child)
    }
}
impl Drop for GroupAnchor {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct KillOnDrop(u32);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0 as i32, libc::SIGKILL);
        }
    }
}
struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
