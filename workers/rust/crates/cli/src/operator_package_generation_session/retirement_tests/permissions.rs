use super::*;
use std::os::unix::fs::PermissionsExt;

thread_local! { static DENIED: RefCell<Option<PathBuf>> = const { RefCell::new(None) }; }

pub(super) fn checkpoint(stage: &str, root: &Path) {
    if stage == "directory-removed" && DENIED.with_borrow(|target| target.as_deref() == Some(root))
    {
        let records = root
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(retirement::DIRECTORY);
        fs::set_permissions(records, fs::Permissions::from_mode(0o500)).unwrap();
    }
}

struct Restore {
    directory: PathBuf,
    permissions: fs::Permissions,
    previous: Option<PathBuf>,
}
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.directory, self.permissions.clone());
        DENIED.replace(self.previous.take());
    }
}

#[test]
#[ignore = "requires real non-root Unix directory permission enforcement"]
fn retirement_record_permission_failures_preserve_proof_until_explicit_retry() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for publication in [true, false] {
        let fixture = Fixture::new("record-permissions");
        let stale = fixture.stale();
        let record = fixture.record(&stale);
        let directory = record.parent().unwrap().to_path_buf();
        let guard = Restore {
            permissions: fs::metadata(&directory).unwrap().permissions(),
            directory: directory.clone(),
            previous: DENIED.replace((!publication).then(|| stale.clone())),
        };
        if publication {
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o500)).unwrap();
        }
        let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(healthy.janitor_report().removed_stale_session_count, 0);
        assert_eq!(healthy.janitor_report().retained_invalid_session_count, 1);
        assert_eq!(stale.exists(), publication);
        assert_eq!(record.exists(), !publication);
        if publication {
            assert!(stale.join(SESSION_MARKER_FILE).is_file());
            assert_eq!(
                fs::read(stale.join("generations/payload/remaining")).unwrap(),
                b"recover me"
            );
        }
        healthy.refresh_stale_sessions().unwrap();
        assert_eq!(healthy.janitor_report().removed_stale_session_count, 0);
        drop(guard);
        healthy.refresh_stale_sessions().unwrap();
        assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
        assert_eq!(healthy.janitor_report().retained_invalid_session_count, 0);
        assert!(!stale.exists());
        assert!(!record.exists());
        let report = healthy.janitor_report();
        healthy.refresh_stale_sessions().unwrap();
        assert_eq!(healthy.janitor_report(), report);
    }
}
