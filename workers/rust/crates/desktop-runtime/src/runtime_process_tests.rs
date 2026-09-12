use std::fs;
use std::net::TcpListener;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{ManagedProcess, records, spawn_managed};

#[test]
fn child_waits_without_readiness() {
    if std::env::var_os("KYUUBIKI_LIFECYCLE_UNIT_CHILD").is_some() {
        std::thread::sleep(Duration::from_secs(30));
    }
}

#[test]
fn registration_failure_and_readiness_timeout_do_not_leak_children() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-lifecycle-cleanup-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let pid_path = root.join("test.pid");
    let blocked_record = pid_path.with_extension("process.json");
    for fail_registration in [true, false] {
        if fail_registration {
            fs::create_dir(&blocked_record).unwrap();
        }
        let reservation = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = reservation.local_addr().unwrap().port();
        drop(reservation);
        let process = ManagedProcess {
            label: "test-child".to_string(),
            command: std::env::current_exe().unwrap(),
            args: vec![
                "--exact".to_string(),
                "runtime_process::tests::child_waits_without_readiness".to_string(),
            ],
            cwd: root.clone(),
            pid: pid_path.clone(),
            log: root.join("test.log"),
            port: Some(port),
            env: [("KYUUBIKI_LIFECYCLE_UNIT_CHILD".to_string(), "1".to_string())].into(),
        };
        let error = spawn_managed(process, Duration::from_millis(250)).unwrap_err();
        let pid = error
            .split("(pid ")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(records::identity(pid).unwrap().is_none(), "{error}");
        assert!(!pid_path.exists());
        if fail_registration {
            assert!(error.contains("failed to create"), "{error}");
            fs::remove_dir(&blocked_record).unwrap();
        } else {
            assert!(error.contains("timed out waiting"), "{error}");
            assert!(!blocked_record.exists());
        }
    }
    fs::remove_dir_all(root).unwrap();
}
