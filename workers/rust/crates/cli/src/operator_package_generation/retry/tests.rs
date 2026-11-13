use super::*;
use std::time::Instant;

struct Fixture {
    cache: PathBuf,
    session: Arc<OperatorPackageGenerationSession>,
}

impl Fixture {
    fn new() -> Self {
        let cache = std::env::temp_dir().join(format!(
            "kyuubiki-generation-retry-{}",
            next_generation_id().unwrap()
        ));
        fs::create_dir_all(cache.join("packages")).unwrap();
        let session = OperatorPackageGenerationSession::open(&cache).unwrap();
        Self { cache, session }
    }

    fn prepared(&self) -> PreparedOperatorPackageGeneration {
        let prepared = prepare_operator_package_generation(
            self.session.clone(),
            &self.cache.join("packages"),
            "operator.next",
        )
        .unwrap();
        fs::write(prepared.packages_root().join("payload"), b"keep").unwrap();
        prepared
    }

    fn abandon(&self) -> (PathBuf, Vec<u8>) {
        let prepared = self.prepared();
        let root = prepared.store_root().to_path_buf();
        let marker = root.join(GENERATION_MARKER_FILE);
        let saved = fs::read(&marker).unwrap();
        fs::write(&marker, b"unresolved identity").unwrap();
        drop(prepared);
        assert!(root.is_dir());
        assert_eq!(self.session.abandoned_generations.state().known.len(), 1);
        (root, saved)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.cache);
    }
}

fn wait_until(check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < deadline, "cleanup retry did not finish");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn abandoned_cleanup_worker_recovers_without_new_work_and_restarts_when_needed() {
    let fixture = Fixture::new();
    let active = fixture.prepared().commit();
    for _ in 0..2 {
        let (root, marker) = fixture.abandon();
        fs::write(root.join(GENERATION_MARKER_FILE), marker).unwrap();
        wait_until(|| !fixture.session.abandoned_generations.state().worker_running);
        assert!(!root.exists());
        assert!(
            fixture
                .session
                .abandoned_generations
                .state()
                .known
                .is_empty()
        );
        assert_eq!(
            fs::read(active.generation_root.join("packages/payload")).unwrap(),
            b"keep"
        );
        assert!(fixture.session.generations_root().is_dir());
    }
    assert_eq!(
        fixture
            .session
            .janitor_report()
            .pending_abandoned_generation_count,
        0
    );
    assert_eq!(
        fixture
            .session
            .janitor_report()
            .removed_abandoned_generation_count,
        2
    );
}

#[test]
fn cleanup_worker_spawn_failure_preserves_queue_for_preparation_retry() {
    let fixture = Fixture::new();
    let generation = fixture.prepared().commit();
    let queue = &fixture.session.abandoned_generations;
    queue.enqueue(&generation);
    queue.ensure_worker_with(&fixture.session, |_| {
        Err(std::io::Error::other("spawn denied"))
    });
    assert!(!queue.state().worker_running);
    assert_eq!(queue.counts(), (1, 0));
    let next = fixture.prepared();
    assert!(!generation.generation_root.exists());
    assert_eq!(queue.counts(), (0, 1));
    assert!(next.packages_root().is_dir());
}

#[test]
fn abandoned_retry_obeys_session_lifecycle_and_keeps_healthy_generation() {
    let fixture = Fixture::new();
    let healthy = fixture.prepared().commit();
    let prepared = fixture.prepared();
    let root = prepared.store_root().to_path_buf();
    let record = fixture
        .cache
        .join("agent-runtime-generations/initializations")
        .join(format!("{}.json", fixture.session.session_id()));
    fs::write(&record, b"unresolved initialization").unwrap();
    drop(prepared);
    fixture.session.retry_abandoned_generations();
    assert_eq!(fixture.session.abandoned_generations.counts(), (1, 0));
    assert_eq!(fs::read(root.join("packages/payload")).unwrap(), b"keep");
    fs::remove_file(record).unwrap();
    fixture.session.retry_abandoned_generations();
    wait_until(|| fixture.session.abandoned_generations.counts() == (0, 1));
    assert!(!root.exists());
    assert_eq!(
        fs::read(healthy.generation_root.join("packages/payload")).unwrap(),
        b"keep"
    );
}

#[test]
fn committed_and_successfully_removed_preparations_never_enter_retry_queue() {
    let fixture = Fixture::new();
    let active = fixture.prepared().commit();
    drop(fixture.prepared());
    assert!(
        fixture
            .session
            .abandoned_generations
            .state()
            .known
            .is_empty()
    );
    assert!(!fixture.session.abandoned_generations.state().worker_running);
    assert!(active.generation_root.join("packages/payload").is_file());
}

#[test]
fn queued_cleanup_keeps_immutable_identity_until_explicit_repair() {
    let fixture = Fixture::new();
    let (root, marker) = fixture.abandon();
    let mut foreign: GenerationMarker = serde_json::from_slice(&marker).unwrap();
    foreign.session_id.push_str("-foreign");
    fs::write(
        root.join(GENERATION_MARKER_FILE),
        serde_json::to_vec(&foreign).unwrap(),
    )
    .unwrap();
    fixture.session.retry_abandoned_generations();
    assert_eq!(fixture.session.abandoned_generations.state().known.len(), 1);
    assert_eq!(fs::read(root.join("packages/payload")).unwrap(), b"keep");
    fs::write(root.join(GENERATION_MARKER_FILE), marker).unwrap();
    fixture.session.retry_abandoned_generations();
    wait_until(|| {
        fixture
            .session
            .abandoned_generations
            .state()
            .known
            .is_empty()
    });
    assert!(!root.exists());
}

#[test]
fn cleanup_queue_deduplicates_and_rotates_failed_batches() {
    let fixture = Fixture::new();
    let generations: Vec<_> = (0..BATCH_LIMIT + 2)
        .map(|_| fixture.prepared().commit())
        .collect();
    let queue = &fixture.session.abandoned_generations;
    for (index, generation) in generations.iter().enumerate() {
        if index < BATCH_LIMIT {
            fs::write(
                generation.generation_root.join(GENERATION_MARKER_FILE),
                b"blocked",
            )
            .unwrap();
        }
        queue.enqueue(generation);
        queue.enqueue(generation);
    }
    assert_eq!(queue.state().known.len(), BATCH_LIMIT + 2);
    queue.retry(&fixture.session);
    assert_eq!(queue.state().known.len(), BATCH_LIMIT + 2);
    queue.retry(&fixture.session);
    assert_eq!(queue.state().known.len(), BATCH_LIMIT);
    assert!(!generations[BATCH_LIMIT].generation_root.exists());
    assert!(!generations[BATCH_LIMIT + 1].generation_root.exists());
    for generation in &generations[..BATCH_LIMIT] {
        assert!(
            generation
                .generation_root
                .join("packages/payload")
                .is_file()
        );
        fs::write(
            generation.generation_root.join(GENERATION_MARKER_FILE),
            serde_json::to_vec(&generation.marker).unwrap(),
        )
        .unwrap();
    }
    queue.retry(&fixture.session);
    assert!(queue.state().known.is_empty());
}

#[test]
fn cleanup_queue_admission_does_not_wait_for_another_sweep() {
    let fixture = Fixture::new();
    let generation = fixture.prepared().commit();
    let queue = &fixture.session.abandoned_generations;
    let sweep = queue.sweep.lock().unwrap();
    queue.enqueue(&generation);
    queue.retry(&fixture.session);
    assert_eq!(queue.state().known.len(), 1);
    assert!(generation.generation_root.exists());
    drop(sweep);
    queue.retry(&fixture.session);
    assert!(queue.state().known.is_empty());
}

#[test]
fn abandoned_retry_queue_does_not_keep_a_closed_session_alive() {
    let fixture = Fixture::new();
    let session = OperatorPackageGenerationSession::open(&fixture.cache).unwrap();
    let prepared = prepare_operator_package_generation(
        session.clone(),
        &fixture.cache.join("packages"),
        "operator.next",
    )
    .unwrap();
    let root = prepared.store_root().to_path_buf();
    fs::write(root.join(GENERATION_MARKER_FILE), b"unresolved identity").unwrap();
    drop(prepared);
    assert_eq!(session.abandoned_generations.state().known.len(), 1);
    let weak = Arc::downgrade(&session);
    drop(session);
    wait_until(|| weak.upgrade().is_none());
    // Weak expiration precedes the last owner's destructor/lease release.
    wait_until(|| {
        fixture.session.refresh_stale_sessions().unwrap();
        !root.exists()
    });
    assert!(!root.exists());
    assert_eq!(
        fixture.session.janitor_report().removed_stale_session_count,
        1
    );
}

#[cfg(unix)]
#[test]
#[ignore = "requires non-root Unix directory permission enforcement"]
fn abandoned_cleanup_retries_real_permission_failure_while_session_is_live() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    struct Restore(PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, self.1.clone());
        }
    }
    let fixture = Fixture::new();
    let prepared = fixture.prepared();
    let root = prepared.store_root().to_path_buf();
    let packages = prepared.packages_root();
    let restore = Restore(
        packages.clone(),
        fs::metadata(&packages).unwrap().permissions(),
    );
    fs::set_permissions(&packages, fs::Permissions::from_mode(0o500)).unwrap();
    drop(prepared);
    fixture.session.retry_abandoned_generations();
    assert_eq!(fixture.session.abandoned_generations.state().known.len(), 1);
    assert_eq!(fs::read(packages.join("payload")).unwrap(), b"keep");
    assert!(cleanup::record_path(&root).unwrap().is_file());
    drop(restore);
    wait_until(|| {
        fixture
            .session
            .abandoned_generations
            .state()
            .known
            .is_empty()
    });
    assert!(!root.exists());
    assert!(!cleanup::record_path(&root).unwrap().exists());
    assert!(fixture.session.generations_root().is_dir());
}
