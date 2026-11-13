use super::*;

#[test]
fn retirement_rejects_unknown_contents_and_ambiguous_identity_without_fallback() {
    for scenario in [
        "unknown",
        "foreign",
        "invalid",
        "marker-conflict",
        "missing-marker",
        "missing-lease",
    ] {
        let fixture = Fixture::new(scenario);
        let stale = fixture.recorded();
        let record = fixture.record(&stale);
        match scenario {
            "unknown" => fs::write(stale.join("unknown"), b"keep").unwrap(),
            "invalid" => fs::write(&record, b"invalid").unwrap(),
            "foreign" => {
                let mut marker = read_marker(&stale).unwrap();
                marker.cache_store_root = fixture.root.display().to_string();
                fs::write(&record, serde_json::to_vec(&marker).unwrap()).unwrap();
            }
            "marker-conflict" => {
                fs::remove_file(stale.join(SESSION_MARKER_FILE)).unwrap();
                fs::write(stale.join(SESSION_MARKER_FILE), b"not the record").unwrap();
            }
            "missing-marker" => fs::remove_file(stale.join(SESSION_MARKER_FILE)).unwrap(),
            _ => fs::remove_file(stale.join(SESSION_LEASE_FILE)).unwrap(),
        }
        let before = fs::read(&record).unwrap();
        let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(
            healthy.janitor_report().removed_stale_session_count,
            0,
            "{scenario}"
        );
        assert_eq!(
            healthy.janitor_report().retained_invalid_session_count,
            1,
            "{scenario}"
        );
        assert_eq!(fs::read(&record).unwrap(), before);
        assert_eq!(
            fs::read(stale.join("generations/payload/remaining")).unwrap(),
            b"recover me"
        );
        healthy.refresh_stale_sessions().unwrap();
        assert!(stale.exists());
    }
}

#[test]
fn retirement_keeps_live_lease_and_rejects_late_downloader_admission() {
    let fixture = Fixture::new("lease");
    let stale = fixture.recorded();
    let generation = stale.join("generations/payload");
    assert!(acquire_download_session_lease(&generation).is_err());
    // Even an unexpected surviving holder must prevent reclamation of a recorded session.
    let holder = File::open(stale.join(SESSION_LEASE_FILE)).unwrap();
    FileExt::try_lock_shared(&holder).unwrap();
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert_eq!(healthy.janitor_report().retained_active_session_count, 1);
    assert!(generation.exists());
    FileExt::unlock(&holder).unwrap();
    drop(holder);
    healthy.refresh_stale_sessions().unwrap();
    assert!(!stale.exists());
    assert!(acquire_download_session_lease(&generation).is_err());
    assert!(!stale.exists());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
}

#[test]
fn retirement_publication_never_replaces_an_existing_record() {
    let fixture = Fixture::new("publication-conflict");
    let stale = fixture.stale();
    let record = fixture.record(&stale);
    fs::write(&record, b"existing record").unwrap();
    let catalog = CatalogLease::acquire(&fixture.cache).unwrap();
    assert!(try_reap_session(&stale, &fixture.cache, &catalog).is_err());
    assert_eq!(fs::read(&record).unwrap(), b"existing record");
    assert!(stale.join("generations/payload/remaining").exists());
}

#[test]
fn concurrent_retirement_recovery_claims_cleanup_once_and_preserves_active_peer() {
    let fixture = Fixture::new("concurrent");
    let active = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let stale = fixture.recorded();
    let start = std::sync::Barrier::new(6);
    let peers: Vec<_> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..6)
            .map(|_| {
                scope.spawn(|| {
                    start.wait();
                    OperatorPackageGenerationSession::open(&fixture.cache).unwrap()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect()
    });
    assert!(!stale.exists());
    assert!(!fixture.record(&stale).exists());
    assert_eq!(
        peers
            .iter()
            .map(|p| p.janitor_report().removed_stale_session_count)
            .sum::<usize>(),
        1
    );
    assert!(
        peers
            .iter()
            .all(|p| p.janitor_report().retained_invalid_session_count == 0)
    );
    assert!(active.session_root.exists());
}

#[cfg(unix)]
#[test]
fn retirement_unlinks_nested_payload_symlinks_without_following_them() {
    let fixture = Fixture::new("nested-link");
    let stale = fixture.recorded();
    let outside = fixture.root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"keep").unwrap();
    for location in ["generations/outside", "generations/payload/outside"] {
        std::os::unix::fs::symlink(&outside, stale.join(location)).unwrap();
    }
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(!stale.exists());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 1);
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
}

#[test]
fn conflicting_initialization_and_retirement_records_block_both_janitors() {
    let fixture = Fixture::new("conflicting-records");
    let stale = fixture.recorded();
    fs::remove_dir_all(stale.join(SESSION_GENERATIONS_DIRECTORY)).unwrap();
    let marker = read_marker(&stale).unwrap();
    let intent = serde_json::json!({"schema_version":"kyuubiki.agent-session-initialization/v1", "session":marker});
    let initializations = stale
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(initialization::DIRECTORY);
    fs::write(
        initializations.join(format!("{}.json", marker.session_id)),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert!(stale.exists());
    assert!(fixture.record(&stale).is_file());
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 0);
    assert_eq!(healthy.janitor_report().retained_invalid_session_count, 2);
}

#[test]
fn legacy_partial_cleanup_without_a_record_remains_unproven() {
    let fixture = Fixture::new("legacy");
    let stale = fixture.stale();
    fs::remove_file(stale.join(SESSION_MARKER_FILE)).unwrap();
    let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    assert_eq!(healthy.janitor_report().removed_stale_session_count, 0);
    assert_eq!(healthy.janitor_report().retained_invalid_session_count, 1);
    assert!(stale.join("generations/payload/remaining").exists());
}

#[cfg(unix)]
#[test]
fn retirement_preserves_symlink_roots_records_and_payload_targets() {
    for entry in [
        "root",
        "record",
        SESSION_GENERATIONS_DIRECTORY,
        SESSION_MARKER_FILE,
        SESSION_LEASE_FILE,
    ] {
        let fixture = Fixture::new(entry);
        let stale = fixture.recorded();
        let outside = fixture.root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), b"keep").unwrap();
        let path = match entry {
            "root" => {
                fs::remove_dir_all(&stale).unwrap();
                stale.clone()
            }
            "record" => {
                let p = fixture.record(&stale);
                fs::remove_file(&p).unwrap();
                p
            }
            SESSION_GENERATIONS_DIRECTORY => {
                let p = stale.join(entry);
                fs::remove_dir_all(&p).unwrap();
                p
            }
            _ => {
                let p = stale.join(entry);
                fs::remove_file(&p).unwrap();
                p
            }
        };
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        let healthy = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
        assert_eq!(
            healthy.janitor_report().removed_stale_session_count,
            0,
            "{entry}"
        );
        assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
    }
}
