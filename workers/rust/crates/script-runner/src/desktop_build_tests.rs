use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use super::asset_io::{SyncStats, copy_changed};
use super::{
    DESKTOP_APPS, INSTALLER_PRIMARY_BUTTON_CSS, SHARED_UI_FILES, files_in_tree, sync_shared_assets,
    verify_shared_assets,
};
use crate::desktop::bundle_staging::BundleStaging;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-desktop-build-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn put(&self, relative: &str, bytes: &[u8]) {
        put(&self.0.join(relative), bytes);
    }

    fn assets() -> Self {
        let fixture = Self::new();
        for name in SHARED_UI_FILES {
            fixture.put(&format!("apps/desktop-shared/ui/{name}"), name.as_bytes());
        }
        fixture.put("assets/brand/brand.json", b"brand");
        fixture.put("language-packs/catalog.json", b"catalog");
        for surface in ["hub", "workbench"] {
            fixture.put(
                &format!("language-packs/{surface}/en/pack.json"),
                b"english",
            );
            fixture.put(
                &format!("language-packs/{surface}/zh/pack.json"),
                b"chinese",
            );
        }
        fixture
    }

    fn bundle(&self) -> PathBuf {
        self.0.join("cache/release/bundle")
    }
    fn staging(&self) -> PathBuf {
        self.0.join("artifacts")
    }
    fn begin(&self) -> BundleStaging {
        fs::create_dir_all(self.bundle().parent().unwrap()).unwrap();
        BundleStaging::begin(self.bundle(), self.staging()).unwrap()
    }

    fn previous(&self) {
        put(
            &self.bundle().join("macos/old.app/payload"),
            b"previous good bundle",
        );
    }

    fn capture_all(&self, staging: &mut BundleStaging) {
        for (app, _) in DESKTOP_APPS {
            for (format, suffix) in [
                ("macos", "app/payload"),
                ("nsis", "exe"),
                ("deb", "deb"),
                ("rpm", "rpm"),
                ("appimage", "AppImage"),
            ] {
                put(
                    &self.bundle().join(format).join(format!("{app}.{suffix}")),
                    app.as_bytes(),
                );
            }
            staging.capture(app).unwrap();
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn put(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn pin_mtime(path: &Path) -> SystemTime {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(123456789)),
        )
        .unwrap();
    fs::metadata(path).unwrap().modified().unwrap()
}

#[test]
fn unchanged_asset_sync_preserves_every_mirror_timestamp() {
    let fixture = Fixture::assets();
    sync_shared_assets(&fixture.0, &mut SyncStats::default()).unwrap();
    let mut before = Vec::new();
    for (app, _) in DESKTOP_APPS {
        let root = fixture.0.join("apps").join(app).join("ui");
        for relative in files_in_tree(&root).unwrap() {
            let path = root.join(relative);
            before.push((path.clone(), pin_mtime(&path)));
        }
    }
    let mut stats = SyncStats::default();
    sync_shared_assets(&fixture.0, &mut stats).unwrap();
    assert_eq!(stats.checked, before.len());
    assert_eq!(stats.written, 0);
    assert_eq!(stats.removed, 0);
    for (path, modified) in before {
        assert_eq!(
            fs::metadata(&path).unwrap().modified().unwrap(),
            modified,
            "{}",
            path.display()
        );
    }
    verify_shared_assets(&fixture.0, &fixture.0.join("apps/desktop-shared/ui")).unwrap();
}

#[test]
fn changed_language_pack_updates_only_its_two_surface_mirrors() {
    let fixture = Fixture::assets();
    sync_shared_assets(&fixture.0, &mut SyncStats::default()).unwrap();
    let untouched = fixture
        .0
        .join("apps/workbench-gui/ui/language-packs/workbench/en/pack.json");
    let before = pin_mtime(&untouched);
    fixture.put("language-packs/hub/en/pack.json", b"updated translation");
    let mut stats = SyncStats::default();
    sync_shared_assets(&fixture.0, &mut stats).unwrap();
    assert_eq!(stats.written, 2);
    assert_eq!(stats.removed, 0);
    assert_eq!(fs::metadata(untouched).unwrap().modified().unwrap(), before);
    verify_shared_assets(&fixture.0, &fixture.0.join("apps/desktop-shared/ui")).unwrap();
}

#[test]
fn obsolete_mirrors_are_pruned_without_recreating_current_language_packs() {
    let fixture = Fixture::assets();
    sync_shared_assets(&fixture.0, &mut SyncStats::default()).unwrap();
    fs::remove_dir_all(fixture.0.join("language-packs/hub/zh")).unwrap();
    fixture.put(
        "apps/hub-gui/ui/language-packs/old-surface/obsolete.json",
        b"stale",
    );
    fixture.put("apps/hub-gui/ui/shared/obsolete.js", b"stale");
    let mut stats = SyncStats::default();
    sync_shared_assets(&fixture.0, &mut stats).unwrap();
    assert_eq!(stats.written, 0);
    assert_eq!(stats.removed, 4);
    verify_shared_assets(&fixture.0, &fixture.0.join("apps/desktop-shared/ui")).unwrap();
}

#[test]
fn installer_stylesheet_suffix_is_composed_once_before_comparing() {
    let fixture = Fixture::assets();
    sync_shared_assets(&fixture.0, &mut SyncStats::default()).unwrap();
    fixture.put(
        "apps/desktop-shared/ui/desktop-shell.css",
        b"new stylesheet",
    );
    let mut stats = SyncStats::default();
    sync_shared_assets(&fixture.0, &mut stats).unwrap();
    assert_eq!(stats.written, 3);
    let css = fs::read_to_string(
        fixture
            .0
            .join("apps/installer-gui/ui/shared/desktop-shell.css"),
    )
    .unwrap();
    assert_eq!(css, format!("new stylesheet{INSTALLER_PRIMARY_BUTTON_CSS}"));
    let mut stats = SyncStats::default();
    sync_shared_assets(&fixture.0, &mut stats).unwrap();
    assert_eq!(stats.written, 0);
}

#[test]
fn equal_length_content_changes_are_not_hidden_by_timestamps() {
    let fixture = Fixture::new();
    fixture.put("source", b"correct");
    fixture.put("target", b"changed");
    pin_mtime(&fixture.0.join("source"));
    pin_mtime(&fixture.0.join("target"));
    let mut stats = SyncStats::default();
    copy_changed(
        &fixture.0.join("source"),
        &fixture.0.join("target"),
        &mut stats,
    )
    .unwrap();
    assert_eq!(stats.written, 1);
    assert_eq!(fs::read(fixture.0.join("target")).unwrap(), b"correct");
}

#[test]
fn missing_language_source_does_not_erase_previous_mirror() {
    let fixture = Fixture::assets();
    sync_shared_assets(&fixture.0, &mut SyncStats::default()).unwrap();
    fs::remove_dir_all(fixture.0.join("language-packs/hub")).unwrap();
    assert!(sync_shared_assets(&fixture.0, &mut SyncStats::default()).is_err());
    assert_eq!(
        fs::read(
            fixture
                .0
                .join("apps/hub-gui/ui/language-packs/hub/en/pack.json")
        )
        .unwrap(),
        b"english"
    );
}

#[cfg(unix)]
#[test]
fn asset_sync_rejects_symlink_targets_without_writing_through_them() {
    let fixture = Fixture::new();
    fixture.put("source", b"new");
    fixture.put("external", b"untouched");
    std::os::unix::fs::symlink("external", fixture.0.join("target")).unwrap();
    assert!(
        copy_changed(
            &fixture.0.join("source"),
            &fixture.0.join("target"),
            &mut SyncStats::default()
        )
        .is_err()
    );
    assert_eq!(fs::read(fixture.0.join("external")).unwrap(), b"untouched");
}

#[test]
fn bundle_publication_preserves_all_three_shells_and_platform_formats() {
    let fixture = Fixture::new();
    fixture.previous();
    let mut staging = fixture.begin();
    assert!(!fixture.bundle().exists());
    fixture.capture_all(&mut staging);
    staging.commit().unwrap();
    assert!(!fixture.staging().exists());
    assert!(!fixture.bundle().join("macos/old.app").exists());
    for (app, _) in DESKTOP_APPS {
        for (format, suffix) in [
            ("macos", "app/payload"),
            ("nsis", "exe"),
            ("deb", "deb"),
            ("rpm", "rpm"),
            ("appimage", "AppImage"),
        ] {
            assert_eq!(
                fs::read(
                    fixture
                        .bundle()
                        .join(format)
                        .join(format!("{app}.{suffix}"))
                )
                .unwrap(),
                app.as_bytes()
            );
        }
    }
}

#[test]
fn failed_build_restores_previous_packages_after_any_shell() {
    for completed in 0..3 {
        let fixture = Fixture::new();
        fixture.previous();
        let before = pin_mtime(&fixture.bundle().join("macos/old.app/payload"));
        let mut staging = fixture.begin();
        for (app, _) in DESKTOP_APPS.into_iter().take(completed) {
            put(&fixture.bundle().join(app), b"new");
            staging.capture(app).unwrap();
        }
        put(&fixture.bundle().join("failed-partial"), b"incomplete");
        staging.rollback().unwrap();
        assert_eq!(
            files_in_tree(&fixture.bundle()).unwrap(),
            vec![PathBuf::from("macos/old.app/payload")]
        );
        assert_eq!(
            fs::read(fixture.bundle().join("macos/old.app/payload")).unwrap(),
            b"previous good bundle"
        );
        assert_eq!(
            fs::metadata(fixture.bundle().join("macos/old.app/payload"))
                .unwrap()
                .modified()
                .unwrap(),
            before
        );
        assert!(!fixture.staging().exists());
    }
}

#[test]
fn initial_failed_build_restores_absent_bundle_state() {
    let fixture = Fixture::new();
    let mut staging = fixture.begin();
    put(&fixture.bundle().join("partial"), b"incomplete");
    staging.rollback().unwrap();
    assert!(!fixture.bundle().exists());
    assert!(!fixture.staging().exists());
}

#[test]
fn existing_staging_is_never_deleted_by_another_build() {
    let fixture = Fixture::new();
    fixture.previous();
    let mut staging = fixture.begin();
    assert!(BundleStaging::begin(fixture.bundle(), fixture.staging()).is_err());
    assert_eq!(
        fs::read(fixture.staging().join("previous/macos/old.app/payload")).unwrap(),
        b"previous good bundle"
    );
    staging.rollback().unwrap();
}

#[test]
fn bundle_collisions_fail_closed_and_previous_set_remains_recoverable() {
    let fixture = Fixture::new();
    fixture.previous();
    let mut staging = fixture.begin();
    for (app, _) in DESKTOP_APPS {
        put(
            &fixture.bundle().join("dmg/common-resource"),
            app.as_bytes(),
        );
        staging.capture(app).unwrap();
    }
    assert!(staging.commit().unwrap_err().contains("conflicting"));
    staging.rollback().unwrap();
    assert_eq!(
        fs::read(fixture.bundle().join("macos/old.app/payload")).unwrap(),
        b"previous good bundle"
    );
}

#[test]
fn identical_bundler_helpers_can_be_shared_without_overwriting() {
    let fixture = Fixture::new();
    let mut staging = fixture.begin();
    for (app, _) in DESKTOP_APPS {
        put(
            &fixture.bundle().join("dmg/common-resource"),
            b"same helper",
        );
        put(
            &fixture.bundle().join(format!("dmg/{app}.dmg")),
            app.as_bytes(),
        );
        staging.capture(app).unwrap();
    }
    staging.commit().unwrap();
    assert_eq!(files_in_tree(&fixture.bundle()).unwrap().len(), 4);
    assert_eq!(
        fs::read(fixture.bundle().join("dmg/common-resource")).unwrap(),
        b"same helper"
    );
}

#[test]
fn same_named_native_packages_cannot_be_deduplicated_as_helpers() {
    for relative in [
        "macos/duplicate.app/payload",
        "nsis/duplicate.exe",
        "deb/duplicate.deb",
    ] {
        let fixture = Fixture::new();
        fixture.previous();
        let mut staging = fixture.begin();
        for (app, _) in DESKTOP_APPS {
            put(
                &fixture.bundle().join(relative),
                b"identical but different owners",
            );
            staging.capture(app).unwrap();
        }
        assert!(staging.commit().unwrap_err().contains("conflicting"));
        staging.rollback().unwrap();
        assert!(fixture.bundle().join("macos/old.app/payload").is_file());
    }
}

#[test]
fn missing_or_duplicate_shells_cannot_publish_a_partial_set() {
    let fixture = Fixture::new();
    let mut staging = fixture.begin();
    assert!(staging.capture("hub-gui").is_err());
    fs::create_dir_all(fixture.bundle()).unwrap();
    assert!(
        staging
            .capture("hub-gui")
            .unwrap_err()
            .contains("no bundle artifacts")
    );
    put(&fixture.bundle().join("hub"), b"new");
    staging.capture("hub-gui").unwrap();
    assert!(staging.capture("hub-gui").is_err());
    assert!(staging.capture("../previous").is_err());
    assert!(staging.commit().is_err());
    staging.rollback().unwrap();
}

#[test]
fn a_finished_bundle_transaction_cannot_remove_published_artifacts() {
    let fixture = Fixture::new();
    let mut staging = fixture.begin();
    fixture.capture_all(&mut staging);
    staging.commit().unwrap();
    assert!(staging.rollback().is_err());
    assert!(staging.commit().is_err());
    assert!(staging.capture("hub-gui").is_err());
    assert!(fixture.bundle().join("macos/hub-gui.app/payload").is_file());
}

#[cfg(unix)]
#[test]
fn bundle_moves_preserve_inode_executable_mode_and_relative_symlinks() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    let fixture = Fixture::new();
    let mut staging = fixture.begin();
    let mut inodes = Vec::new();
    for (app, _) in DESKTOP_APPS {
        let tree = fixture.bundle().join("macos").join(format!("{app}.app"));
        put(&tree.join("Versions/A/binary"), b"signed executable");
        fs::set_permissions(
            tree.join("Versions/A/binary"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        symlink("A", tree.join("Versions/Current")).unwrap();
        symlink("Versions/Current/binary", tree.join("binary")).unwrap();
        inodes.push((app, fs::metadata(tree.join("binary")).unwrap().ino()));
        staging.capture(app).unwrap();
    }
    staging.commit().unwrap();
    for (app, inode) in inodes {
        let tree = fixture.bundle().join("macos").join(format!("{app}.app"));
        assert_eq!(fs::metadata(tree.join("binary")).unwrap().ino(), inode);
        assert_eq!(
            fs::metadata(tree.join("binary")).unwrap().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::read_link(tree.join("Versions/Current")).unwrap(),
            PathBuf::from("A")
        );
        assert_eq!(fs::read(tree.join("binary")).unwrap(), b"signed executable");
    }
}
