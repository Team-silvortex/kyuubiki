use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Platform;
use crate::desktop_bundle_package::{
    DESKTOP_BUNDLE_SET_SCHEMA_VERSION, prepare_desktop_bundle_set, verify_desktop_bundle_set,
};
use crate::desktop_bundle_qualification::DESKTOP_BUNDLE_QUALIFICATION_SCHEMA_VERSION;
use crate::desktop_bundle_store::{
    DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION, active_desktop_bundle_entrypoints_in,
    active_desktop_bundle_manifest_in, active_desktop_bundle_root_in, desktop_bundle_set_status_in,
    install_desktop_bundle_set_into, rollback_desktop_bundle_set_in,
};

#[test]
fn desktop_bundle_schemas_match_runtime_contracts() {
    let bundle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/desktop-bundle-set.schema.json"
    ))
    .unwrap();
    let activation: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/desktop-bundle-activation.schema.json"
    ))
    .unwrap();
    let qualification: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/desktop-bundle-update-qualification-report.schema.json"
    ))
    .unwrap();
    assert_eq!(
        bundle["properties"]["schema_version"]["const"],
        DESKTOP_BUNDLE_SET_SCHEMA_VERSION
    );
    assert_eq!(
        activation["properties"]["schema_version"]["const"],
        DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION
    );
    assert_eq!(
        qualification["properties"]["schema_version"]["const"],
        DESKTOP_BUNDLE_QUALIFICATION_SCHEMA_VERSION
    );
    assert_eq!(
        qualification["$defs"]["activation"]["properties"]["schema_version"]["const"],
        DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION
    );
}

#[test]
fn installs_upgrades_and_restores_all_three_desktop_components() {
    for platform in [Platform::Macos, Platform::Linux, Platform::Windows] {
        let root = fixture_root(&format!("round-trip-{}", platform.as_str()));
        let first_source = root.join("first-source");
        let second_source = root.join("second-source");
        let first_package = root.join("first-package");
        let second_package = root.join("second-package");
        let store = root.join("store");
        write_source(&first_source, platform, "first");
        write_source(&second_source, platform, "second");
        let first =
            prepare_desktop_bundle_set(&first_source, &first_package, "2.16.9", platform).unwrap();
        let second =
            prepare_desktop_bundle_set(&second_source, &second_package, "2.17.0", platform)
                .unwrap();
        assert_ne!(first.payload_sha256, second.payload_sha256);

        let initial = install_desktop_bundle_set_into(&first_package, &store, platform).unwrap();
        let upgrade = install_desktop_bundle_set_into(&second_package, &store, platform).unwrap();
        assert_eq!(initial.generation, 1);
        assert_eq!(upgrade.generation, 2);
        assert_eq!(upgrade.previous_version.as_deref(), Some("2.16.9"));
        assert_eq!(
            active_desktop_bundle_entrypoints_in(&store, platform)
                .unwrap()
                .len(),
            3
        );

        let rollback = rollback_desktop_bundle_set_in(&store, platform).unwrap();
        let restored = active_desktop_bundle_manifest_in(&store, platform).unwrap();
        assert_eq!(rollback.generation, 3);
        assert_eq!(rollback.version, "2.16.9");
        assert_eq!(rollback.payload_sha256, first.payload_sha256);
        assert_eq!(restored, first);
        let status = desktop_bundle_set_status_in(&store).unwrap();
        assert_eq!(status.active_version.as_deref(), Some("2.16.9"));
        assert_eq!(status.previous_version.as_deref(), Some("2.17.0"));
        assert_eq!(status.installed_versions, ["2.16.9", "2.17.0"]);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn rejects_tampered_active_content_and_same_version_rewrites() {
    let root = fixture_root("tamper");
    let platform = Platform::current();
    let source = root.join("source");
    let changed_source = root.join("changed-source");
    let package = root.join("package");
    let changed_package = root.join("changed-package");
    let store = root.join("store");
    write_source(&source, platform, "original");
    write_source(&changed_source, platform, "rewritten");
    prepare_desktop_bundle_set(&source, &package, "2.17.0", platform).unwrap();
    prepare_desktop_bundle_set(&changed_source, &changed_package, "2.17.0", platform).unwrap();
    install_desktop_bundle_set_into(&package, &store, platform).unwrap();
    let error = install_desktop_bundle_set_into(&changed_package, &store, platform).unwrap_err();
    assert!(error.contains("already exists with different content"));

    let entrypoint = active_desktop_bundle_entrypoints_in(&store, platform)
        .unwrap()
        .remove(0)
        .executable_path;
    fs::write(&entrypoint, "tampered").unwrap();
    let error = active_desktop_bundle_root_in(&store, platform).unwrap_err();
    assert!(error.contains("inventory") || error.contains("digest"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn package_verification_rejects_extra_files_and_platform_drift() {
    let root = fixture_root("shape");
    let source = root.join("source");
    let package = root.join("package");
    write_source(&source, Platform::Linux, "linux");
    prepare_desktop_bundle_set(&source, &package, "2.17.0", Platform::Linux).unwrap();
    assert!(verify_desktop_bundle_set(&package, Platform::Macos).is_err());
    fs::write(package.join("unmanaged.txt"), "not declared").unwrap();
    let error = verify_desktop_bundle_set(&package, Platform::Linux).unwrap_err();
    assert!(error.contains("unmanaged file"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_lock_and_stale_staging_are_fail_safe() {
    let root = fixture_root("lock");
    let platform = Platform::current();
    let first_source = root.join("first-source");
    let second_source = root.join("second-source");
    let first_package = root.join("first-package");
    let second_package = root.join("second-package");
    let store = root.join("store");
    write_source(&first_source, platform, "first");
    write_source(&second_source, platform, "second");
    prepare_desktop_bundle_set(&first_source, &first_package, "2.16.9", platform).unwrap();
    prepare_desktop_bundle_set(&second_source, &second_package, "2.17.0", platform).unwrap();
    install_desktop_bundle_set_into(&first_package, &store, platform).unwrap();
    fs::write(store.join("update.lock"), "other-controller").unwrap();
    let error = install_desktop_bundle_set_into(&second_package, &store, platform).unwrap_err();
    assert!(error.contains("already locked"));
    assert_eq!(
        active_desktop_bundle_manifest_in(&store, platform)
            .unwrap()
            .version,
        "2.16.9"
    );
    fs::remove_file(store.join("update.lock")).unwrap();
    fs::create_dir_all(store.join("staging/abandoned")).unwrap();
    fs::write(store.join("staging/abandoned/partial"), "partial").unwrap();
    install_desktop_bundle_set_into(&second_package, &store, platform).unwrap();
    assert!(!store.join("staging/abandoned").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn activation_identity_tampering_fails_closed() {
    let root = fixture_root("activation-tamper");
    let platform = Platform::current();
    let source = root.join("source");
    let package = root.join("package");
    let store = root.join("store");
    write_source(&source, platform, "original");
    prepare_desktop_bundle_set(&source, &package, "2.17.0", platform).unwrap();
    install_desktop_bundle_set_into(&package, &store, platform).unwrap();
    let activation = store.join("activations/00000000000000000001.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&activation).unwrap()).unwrap();
    value["payload_sha256"] = serde_json::Value::String("0".repeat(64));
    fs::write(&activation, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert!(active_desktop_bundle_root_in(&store, platform).is_err());
    assert!(
        store
            .join("versions/2.17.0/manifests/desktop-bundle-set.json")
            .is_file()
    );
    fs::remove_dir_all(root).unwrap();
}

fn write_source(root: &Path, platform: Platform, marker: &str) {
    for (bundle, entrypoint) in definitions(platform) {
        let bundle_path = root.join(bundle);
        let executable = root.join(entrypoint);
        if bundle != entrypoint {
            fs::create_dir_all(&bundle_path).unwrap();
        }
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, format!("{marker}:{entrypoint}")).unwrap();
        make_executable(&executable);
        if platform == Platform::Macos {
            fs::write(bundle_path.join("Contents/Info.plist"), marker).unwrap();
        }
    }
}

fn definitions(platform: Platform) -> [(&'static str, &'static str); 3] {
    match platform {
        Platform::Macos => [
            (
                "Kyuubiki Hub.app",
                "Kyuubiki Hub.app/Contents/MacOS/kyuubiki-hub-gui",
            ),
            (
                "Kyuubiki Installer.app",
                "Kyuubiki Installer.app/Contents/MacOS/kyuubiki-installer-gui",
            ),
            (
                "Kyuubiki Workbench.app",
                "Kyuubiki Workbench.app/Contents/MacOS/kyuubiki-workbench-gui",
            ),
        ],
        Platform::Linux => [
            ("kyuubiki-hub-gui", "kyuubiki-hub-gui"),
            ("kyuubiki-installer-gui", "kyuubiki-installer-gui"),
            ("kyuubiki-workbench-gui", "kyuubiki-workbench-gui"),
        ],
        Platform::Windows => [
            ("Kyuubiki Hub", "Kyuubiki Hub/kyuubiki-hub-gui.exe"),
            (
                "Kyuubiki Installer",
                "Kyuubiki Installer/kyuubiki-installer-gui.exe",
            ),
            (
                "Kyuubiki Workbench",
                "Kyuubiki Workbench/kyuubiki-workbench-gui.exe",
            ),
        ],
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

fn fixture_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-desktop-bundle-{name}-{}-{nonce}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}
