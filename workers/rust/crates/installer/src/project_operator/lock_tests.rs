use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-project-lock-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn project_storage_holds_lock_until_operation_finishes() {
    let fixture = Fixture::new();
    let storage = Storage::open(&fixture.0, "project", "scope").unwrap();
    assert_eq!(
        Storage::open(&fixture.0, "other-project", "scope").err(),
        Some("project_operator_busy".into())
    );
    drop(storage);
    assert!(Storage::open(&fixture.0, "project", "scope").is_ok());
}

#[cfg(unix)]
#[test]
fn completed_operation_unlocks_even_while_a_duplicate_handle_survives() {
    let fixture = Fixture::new();
    let storage = Storage::open(&fixture.0, "project", "scope").unwrap();
    // Like a descriptor inherited by a concurrently spawned child before exec,
    // dup keeps the open file description alive after the parent's close.
    let duplicate = storage._lock.try_clone().unwrap();
    drop(storage);
    let next = Storage::open(&fixture.0, "project", "scope");
    assert!(
        next.is_ok(),
        "completed operation retained its lock: {:?}",
        next.err()
    );
    drop(duplicate);
}
