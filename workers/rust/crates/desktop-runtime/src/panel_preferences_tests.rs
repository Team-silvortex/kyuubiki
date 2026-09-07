use super::*;
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-panel-layout-{}-{}",
            std::process::id(),
            NEXT_WRITE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join(FILE_NAME)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn layout() -> WorkbenchPanelLayout {
    WorkbenchPanelLayout {
        version: 1,
        sizes: WorkbenchPanelSizes {
            sidebar: Some(327.5),
            inspector: Some(267.0),
            report: Some(274.0),
        },
    }
}

#[test]
fn panel_layout_reopens_updates_and_resets_without_leaving_staging_files() {
    let fixture = Fixture::new();
    let path = fixture.path();
    assert_eq!(read_at(&path).unwrap(), None);
    write_at(&path, &layout()).unwrap();
    assert_eq!(read_at(&path).unwrap(), Some(layout()));
    let mut updated = layout();
    updated.sizes.sidebar = Some(400.0);
    write_at(&path, &updated).unwrap();
    assert_eq!(read_at(&path).unwrap(), Some(updated));
    let reset = WorkbenchPanelLayout {
        version: 1,
        sizes: WorkbenchPanelSizes::default(),
    };
    write_at(&path, &reset).unwrap();
    assert_eq!(read_at(&path).unwrap(), Some(reset));
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn panel_layout_rejects_invalid_input_without_replacing_committed_data() {
    let fixture = Fixture::new();
    let path = fixture.path();
    write_at(&path, &layout()).unwrap();
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 5000.1] {
        let mut invalid = layout();
        invalid.sizes.sidebar = Some(value);
        assert!(write_at(&path, &invalid).is_err());
        assert_eq!(read_at(&path).unwrap(), Some(layout()));
    }
    let mut future = layout();
    future.version = 2;
    assert!(write_at(&path, &future).is_err());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn panel_layout_reads_are_bounded_and_strict_without_destroying_bad_input() {
    let fixture = Fixture::new();
    let path = fixture.path();
    for raw in [
        "{".into(),
        "x".repeat(1025),
        r#"{"version":2,"sizes":{}}"#.into(),
        r#"{"version":1,"sizes":{"sidebar":1e999}}"#.into(),
        r#"{"version":1,"sizes":{"sidebar":-10}}"#.into(),
        r#"{"version":1,"sizes":{},"path":"elsewhere"}"#.into(),
        r#"{"version":1,"sizes":{"unrelated":1}}"#.into(),
    ] {
        fs::write(&path, &raw).unwrap();
        assert!(read_at(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
    }
    write_at(&path, &layout()).unwrap();
    assert_eq!(read_at(&path).unwrap(), Some(layout()));
}

#[test]
fn panel_layout_does_not_replace_a_directory() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.path()).unwrap();
    assert!(read_at(&fixture.path()).is_err());
    assert!(write_at(&fixture.path(), &layout()).is_err());
    assert!(fixture.path().is_dir());
}

#[cfg(unix)]
#[test]
fn panel_layout_rejects_symlink_file_and_parent() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let original = fixture.0.join("original.json");
    fs::write(&original, "unchanged").unwrap();
    symlink(&original, fixture.path()).unwrap();
    assert!(read_at(&fixture.path()).is_err());
    assert!(write_at(&fixture.path(), &layout()).is_err());
    assert_eq!(fs::read_to_string(original).unwrap(), "unchanged");
    let alias = fixture.0.join("alias");
    symlink(&fixture.0, &alias).unwrap();
    assert!(read_at(&alias.join(FILE_NAME)).is_err());
    assert!(write_at(&alias.join(FILE_NAME), &layout()).is_err());
}
