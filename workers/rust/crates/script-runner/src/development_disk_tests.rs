use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut random = [0; 16];
        getrandom::fill(&mut random).unwrap();
        let path = std::env::temp_dir().join(format!("kyuubiki-dev-disk-{random:x?}"));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"fixture").unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn development_cleanup_preview_and_apply_preserve_sources_libraries_tools_and_results() {
    let fixture = Fixture::new();
    let removed: Vec<_> = [
        "workers/rust/target/debug/incremental/crate/session/work.o",
        "workers/rust/target/debug/deps/test-ab0123456789abcd.xyz.rcgu.o",
        "workers/rust/target/debug/deps/test-ab0123456789abcd",
        "sdks/rust/target/debug/incremental/sdk/session/work.o",
        "apps/frontend/.next/cache/compiler/cache",
    ]
    .iter()
    .map(|p| fixture.write(p))
    .collect();
    fixture.write("workers/rust/target/debug/deps/test-ab0123456789abcd.d");
    let preserved: Vec<_> = [
        "workers/rust/target/debug/kyuubiki-script-runner",
        "workers/rust/target/debug/deps/libtest-ab0123456789abcd.rlib",
        "workers/rust/target/debug/deps/libtest-ab0123456789abcd.rmeta",
        "workers/rust/target/debug/deps/unrelated-ab0123456789abcd",
        "workers/rust/target/release/deps/release-ab0123456789abcd",
        "workers/rust/crates/solver/src/lib.rs",
        "tmp/data/model.json",
        "reports/research.md",
        "dist/macos/bin/kyuubiki-cli",
    ]
    .iter()
    .map(|p| fixture.write(p))
    .collect();
    let candidates = plan(&fixture.0, true).unwrap();
    assert!(
        removed.iter().all(|p| p.exists()),
        "preview must not mutate files"
    );
    apply(&fixture.0, &candidates).unwrap();
    assert!(removed.iter().all(|p| !p.exists()));
    assert!(preserved.iter().all(|p| p.exists()));
    assert!(
        plan(&fixture.0, true).unwrap().is_empty(),
        "repeat cleanup is idempotent"
    );
}

#[test]
fn development_cleanup_keeps_test_binaries_until_explicitly_requested() {
    let fixture = Fixture::new();
    let binary = fixture.write("workers/rust/target/debug/deps/test-ab0123456789abcd.exe");
    fixture.write("workers/rust/target/debug/deps/test-ab0123456789abcd.d");
    apply(&fixture.0, &plan(&fixture.0, false).unwrap()).unwrap();
    assert!(binary.exists());
    apply(&fixture.0, &plan(&fixture.0, true).unwrap()).unwrap();
    assert!(!binary.exists());
}

#[test]
fn development_cleanup_refuses_active_cargo_before_modifying_any_cache() {
    let fixture = Fixture::new();
    let keep = fixture.write("workers/rust/target/debug/incremental/crate/keep");
    let lock_path = fixture.write("sdks/rust/target/debug/.cargo-lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path)
        .unwrap();
    lock.try_lock_exclusive().unwrap();
    let error = run(&fixture.0, vec!["--apply".into()]).unwrap_err();
    assert!(error.contains("in use"), "{error}");
    assert!(keep.exists());
}

#[test]
fn development_cleanup_rejects_tracked_outputs_and_unknown_options() {
    let fixture = Fixture::new();
    let keep = fixture.write("workers/rust/target/debug/incremental/crate/keep");
    for args in [
        vec!["--root", "/"],
        vec!["--all"],
        vec!["--apply", "../../other"],
    ] {
        assert!(parse_options(&args.into_iter().map(OsString::from).collect::<Vec<_>>()).is_err());
    }
    assert!(
        Command::new("git")
            .current_dir(&fixture.0)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .current_dir(&fixture.0)
            .args(["add", "workers"])
            .status()
            .unwrap()
            .success()
    );
    let error = run(&fixture.0, vec!["--apply".into()]).unwrap_err();
    assert!(error.contains("tracked files"), "{error}");
    assert!(keep.exists());
}

#[cfg(unix)]
#[test]
fn development_cleanup_rejects_symlinked_roots_and_does_not_follow_child_links() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let keep = outside.write("private/model.json");
    let parent = fixture.0.join("workers/rust/target/debug");
    fs::create_dir_all(&parent).unwrap();
    symlink(&outside.0, parent.join("incremental")).unwrap();
    assert!(plan(&fixture.0, false).unwrap_err().contains("symlink"));
    fs::remove_file(parent.join("incremental")).unwrap();
    fs::create_dir(parent.join("incremental")).unwrap();
    symlink(&outside.0, parent.join("incremental/outside")).unwrap();
    let candidates = plan(&fixture.0, false).unwrap();
    apply(&fixture.0, &candidates).unwrap();
    assert_eq!(fs::read(keep).unwrap(), b"fixture");
}
