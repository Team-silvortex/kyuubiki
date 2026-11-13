use super::*;
use crate::operator_package_generation::prepare_operator_package_generation;
use crate::operator_package_generation_session::OperatorPackageGenerationSession;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    cache: PathBuf,
    _session: Arc<OperatorPackageGenerationSession>,
    record: PathBuf,
    marker: Vec<u8>,
}

impl Fixture {
    fn removed_root() -> (Self, Vec<RetiredOperatorPackageGeneration>) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let cache = std::env::temp_dir().join(format!(
            "kyuubiki-generation-queue-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(cache.join("packages")).unwrap();
        let session = OperatorPackageGenerationSession::open(&cache).unwrap();
        let prepared = prepare_operator_package_generation(
            session.clone(),
            &cache.join("packages"),
            "operator.next",
        )
        .unwrap();
        let root = prepared.store_root().to_path_buf();
        let marker_path = root.join("kyuubiki-agent-generation.json");
        let marker = fs::read(&marker_path).unwrap();
        let record = root.parent().unwrap().join(format!(
            ".kyuubiki-retired-{}.json",
            prepared.generation_id()
        ));
        fs::hard_link(&marker_path, &record).unwrap();
        // Reconstruct the persisted state after root unlink, before journal unlink.
        fs::remove_dir_all(&root).unwrap();
        let retired = vec![RetiredOperatorPackageGeneration {
            host: Weak::new(),
            generation: prepared.commit(),
        }];
        (
            Self {
                cache,
                _session: session,
                record,
                marker,
            },
            retired,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.cache);
    }
}

#[test]
fn retirement_queue_keeps_owner_when_the_remaining_record_is_invalid() {
    let (fixture, mut retired) = Fixture::removed_root();
    fs::write(&fixture.record, b"ambiguous record").unwrap();
    reap_retired_entries(&mut retired);
    assert_eq!(retired.len(), 1);
    assert_eq!(fs::read(&fixture.record).unwrap(), b"ambiguous record");
    fs::write(&fixture.record, &fixture.marker).unwrap();
    reap_retired_entries(&mut retired);
    assert!(retired.is_empty());
    assert!(!fixture.record.exists());
    reap_retired_entries(&mut retired);
    assert!(retired.is_empty());
}

#[cfg(unix)]
#[test]
#[ignore = "requires non-root Unix directory permission enforcement"]
fn retirement_queue_retains_owner_after_final_record_unlink_permission_failure() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    struct Restore(PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, self.1.clone());
        }
    }
    let (fixture, mut retired) = Fixture::removed_root();
    let parent = fixture.record.parent().unwrap();
    let restore = Restore(parent.into(), fs::metadata(parent).unwrap().permissions());
    fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
    reap_retired_entries(&mut retired);
    assert_eq!(retired.len(), 1, "failed final unlink lost the retry owner");
    assert_eq!(fs::read(&fixture.record).unwrap(), fixture.marker);
    drop(restore);
    reap_retired_entries(&mut retired);
    assert!(retired.is_empty());
    assert!(!fixture.record.exists());
}
