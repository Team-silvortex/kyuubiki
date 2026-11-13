use super::super::tests::Fixture;
#[cfg(unix)]
use super::super::tests::kill_at_checkpoint;
use super::*;

#[cfg(unix)]
#[test]
fn killed_staging_publication_recovers_empty_partial_synced_and_published_records() {
    for phase in [
        "intent-staging-created",
        "intent-partial",
        "intent-staging-synced",
        "intent-published",
    ] {
        let fixture = Fixture::new(phase);
        let absent = kill_at_checkpoint(&fixture, phase);
        assert!(!absent.exists());
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        let report = session.janitor_report();
        assert_eq!(report.removed_initialization_staging_count, 1, "{phase}");
        assert_eq!(
            report.removed_stale_session_count,
            usize::from(phase == "intent-published"),
            "{phase}"
        );
        assert_eq!(report.retained_invalid_session_count, 0, "{phase}");
        assert_eq!(fs::read_dir(fixture.intents()).unwrap().count(), 0);
        session.refresh_stale_sessions().unwrap();
        assert_eq!(
            session.janitor_report(),
            report,
            "no duplicate cleanup count"
        );
        drop(session);
        let next = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(
            next.janitor_report().removed_initialization_staging_count,
            0
        );
    }
}

#[cfg(unix)]
#[test]
fn killed_staging_cleanup_leaves_published_record_recoverable() {
    let fixture = Fixture::new("staging-cleanup");
    let stale = fixture.incomplete();
    let id = stale.file_name().unwrap().to_str().unwrap();
    let bytes = expected_bytes(id, &fixture.cache).unwrap();
    let pending = fixture.intents().join(staging_name(id, &bytes));
    fs::hard_link(fixture.intents().join(format!("{id}.json")), &pending).unwrap();
    assert_eq!(
        kill_at_checkpoint(&fixture, "intent-staging-removed"),
        stale
    );
    assert!(!pending.exists());
    assert!(stale.exists());
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!stale.exists());
    assert_eq!(session.janitor_report().removed_stale_session_count, 1);
    assert_eq!(
        session
            .janitor_report()
            .removed_initialization_staging_count,
        0
    );
    assert_eq!(session.janitor_report().retained_invalid_session_count, 0);
}

#[test]
fn staging_recovery_requires_commitment_prefix_and_unambiguous_publication() {
    for case in [
        "hash",
        "cache",
        "prefix",
        "tail",
        "oversize",
        "final-conflict",
        "partial-with-final",
        "unexpected-session",
        "malformed-name",
    ] {
        let fixture = Fixture::new(case);
        let (id, mut pending, bytes) = pending_fixture(&fixture);
        let final_path = fixture.intents().join(format!("{id}.json"));
        let root = fixture.sessions.join(&id);
        match case {
            "hash" => {
                let wrong = fixture
                    .intents()
                    .join(format!("{id}.{}{SUFFIX}", "0".repeat(64)));
                fs::rename(&pending, &wrong).unwrap();
                pending = wrong;
            }
            "cache" => {
                let foreign = expected_bytes(&id, &fixture.root.join("elsewhere")).unwrap();
                let wrong = fixture.intents().join(staging_name(&id, &foreign));
                fs::rename(&pending, &wrong).unwrap();
                pending = wrong;
                fs::write(&pending, foreign).unwrap();
            }
            "prefix" => fs::write(&pending, b"not our payload").unwrap(),
            "tail" => fs::write(&pending, [bytes.as_slice(), b"extra"].concat()).unwrap(),
            "oversize" => fs::write(&pending, vec![0; MAX_MARKER_BYTES as usize + 1]).unwrap(),
            "final-conflict" => {
                fs::write(&pending, &bytes).unwrap();
                fs::write(&final_path, b"keep").unwrap();
            }
            "partial-with-final" => fs::write(&final_path, &bytes).unwrap(),
            "unexpected-session" => {
                fs::create_dir(&root).unwrap();
                fs::write(root.join("keep"), b"keep").unwrap();
            }
            _ => {
                let wrong = fixture.intents().join(format!("unknown{SUFFIX}"));
                fs::rename(&pending, &wrong).unwrap();
                pending = wrong;
            }
        }
        let original = fs::read(&pending).unwrap();
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(fs::read(&pending).unwrap(), original, "{case}");
        assert_eq!(
            session
                .janitor_report()
                .removed_initialization_staging_count,
            0,
            "{case}"
        );
        assert!(
            session.janitor_report().retained_invalid_session_count >= 1,
            "{case}"
        );
        if case == "unexpected-session" {
            assert_eq!(fs::read(root.join("keep")).unwrap(), b"keep");
        }
        if case == "partial-with-final" {
            assert_eq!(fs::read(final_path).unwrap(), bytes);
        }
    }
}

#[test]
fn rejected_staging_protects_a_complete_session_from_both_janitors() {
    let fixture = Fixture::new("blocked-complete");
    let root = fixture.incomplete();
    let id = root.file_name().unwrap().to_str().unwrap();
    let bytes = expected_bytes(id, &fixture.cache).unwrap();
    let pending = fixture.intents().join(staging_name(id, &bytes));
    fs::write(&pending, b"mismatch").unwrap();
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(root.exists());
    assert!(fixture.intents().join(format!("{id}.json")).exists());
    assert_eq!(session.janitor_report().removed_stale_session_count, 0);
    assert_eq!(session.janitor_report().retained_invalid_session_count, 1);
    fs::remove_file(&pending).unwrap();
    session.refresh_stale_sessions().unwrap();
    assert!(!root.exists());
    assert_eq!(session.janitor_report().removed_stale_session_count, 1);
}

#[test]
fn completed_staging_unlink_preserves_active_lease_and_final_record() {
    let fixture = Fixture::new("active-published");
    let root = fixture.incomplete();
    let id = root.file_name().unwrap().to_str().unwrap();
    let bytes = expected_bytes(id, &fixture.cache).unwrap();
    let final_path = fixture.intents().join(format!("{id}.json"));
    let pending = fixture.intents().join(staging_name(id, &bytes));
    fs::hard_link(&final_path, &pending).unwrap();
    let lease = File::open(root.join(SESSION_LEASE_FILE)).unwrap();
    FileExt::try_lock_shared(&lease).unwrap();
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!pending.exists());
    assert!(root.exists());
    assert_eq!(fs::read(&final_path).unwrap(), bytes);
    assert_eq!(
        session
            .janitor_report()
            .removed_initialization_staging_count,
        1
    );
    assert_eq!(session.janitor_report().retained_active_session_count, 1);
    // Concurrent test child creation can temporarily duplicate the raw descriptor.
    FileExt::unlock(&lease).unwrap();
    drop(lease);
    session.refresh_stale_sessions().unwrap();
    assert!(!root.exists());
    assert_eq!(session.janitor_report().removed_stale_session_count, 1);
    assert_eq!(
        session
            .janitor_report()
            .removed_initialization_staging_count,
        1
    );
}

#[test]
fn publish_complete_never_replaces_existing_content() {
    let fixture = Fixture::new("publication-collision");
    let (_, pending, bytes) = pending_fixture(&fixture);
    fs::write(&pending, &bytes).unwrap();
    let destination = fixture.intents().join("existing.json");
    fs::write(&destination, b"keep").unwrap();
    assert!(publish_complete(&pending, &destination).is_err());
    assert_eq!(fs::read(destination).unwrap(), b"keep");
    assert_eq!(fs::read(pending).unwrap(), bytes);
}

#[cfg(unix)]
#[test]
fn staging_and_publication_reject_symlinks_without_touching_targets() {
    for kind in ["staging", "final", "session"] {
        let fixture = Fixture::new(kind);
        let (id, pending, bytes) = pending_fixture(&fixture);
        let target = fixture.root.join("outside");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"keep").unwrap();
        let link = match kind {
            "staging" => {
                fs::remove_file(&pending).unwrap();
                pending.clone()
            }
            "final" => fixture.intents().join(format!("{id}.json")),
            _ => fixture.sessions.join(id),
        };
        std::os::unix::fs::symlink(&target, &link).unwrap();
        if kind == "final" {
            fs::write(&pending, &bytes).unwrap();
            assert!(publish_complete(&pending, &link).is_err());
        }
        let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"keep");
        assert_eq!(
            session
                .janitor_report()
                .removed_initialization_staging_count,
            0
        );
    }
}

#[test]
fn staging_identity_is_canonical_and_never_uses_pid_for_liveness() {
    for bad in [
        "session-0-1-0",
        "session-01-1-0",
        "session-1-01-0",
        "session-1-1-00",
        "session-+1-1-0",
        "session-1-2-3-extra",
        "session-../1-2-3",
    ] {
        assert!(owner_pid(bad).is_err(), "{bad}");
    }
    assert_eq!(owner_pid("session-4294967295-1-0").unwrap(), u32::MAX);
}

fn pending_fixture(fixture: &Fixture) -> (String, PathBuf, Vec<u8>) {
    let id = next_session_id().unwrap();
    let bytes = expected_bytes(&id, &fixture.cache).unwrap();
    fs::create_dir_all(fixture.intents()).unwrap();
    let path = fixture.intents().join(staging_name(&id, &bytes));
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    (id, path, bytes)
}
