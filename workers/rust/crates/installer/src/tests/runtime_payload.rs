use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runtime_payload::{
    install_runtime_payload_into, rollback_runtime_payload_in, runtime_payload_content_digest_in,
    runtime_payload_status_in,
};
use crate::{Platform, seal_runtime_payload};

fn fixture_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kyuubiki-runtime-payload-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn write_payload(root: &Path, version: &str) {
    for relative in [
        "bin",
        "manifests",
        "services/orchestrator/bin",
        "services/frontend",
        "runtimes/macos/node/bin",
    ] {
        fs::create_dir_all(root.join(relative)).unwrap();
    }
    for relative in [
        "bin/kyuubiki-cli",
        "services/orchestrator/bin/kyuubiki_web",
        "services/frontend/server.js",
        "runtimes/macos/node/bin/node",
    ] {
        fs::write(root.join(relative), format!("{relative}:{version}")).unwrap();
    }
    fs::write(
        root.join("manifests/service-launch.json"),
        r#"{
          "schema_version":"kyuubiki.service-launch/v1",
          "services":[
            {"id":"agent","command":"bin/kyuubiki-cli","cwd":".","args":[]},
            {"id":"orchestrator","command":"services/orchestrator/bin/kyuubiki_web","cwd":"services/orchestrator","args":[]},
            {"id":"frontend","command":"runtimes/macos/node/bin/node","cwd":".","args":[]}
          ]
        }"#,
    )
    .unwrap();
    seal_runtime_payload(root, version, Platform::Macos).unwrap();
}

#[test]
fn installs_activates_and_rolls_back_versioned_payloads() {
    let root = fixture_root("lifecycle");
    let first = root.join("first");
    let second = root.join("second");
    let store = root.join("store");
    write_payload(&first, "2.7.0");
    write_payload(&second, "2.7.1");

    let active = install_runtime_payload_into(&first, &store, Platform::Macos).unwrap();
    assert_eq!(active.version, "2.7.0");
    assert_eq!(active.generation, 1);
    let active = install_runtime_payload_into(&second, &store, Platform::Macos).unwrap();
    assert_eq!(active.previous_version.as_deref(), Some("2.7.0"));
    assert_eq!(active.generation, 2);

    let first_digest =
        runtime_payload_content_digest_in(&store.join("versions/2.7.0"), Platform::Macos).unwrap();
    let second_digest =
        runtime_payload_content_digest_in(&store.join("versions/2.7.1"), Platform::Macos).unwrap();
    assert_ne!(first_digest, second_digest);

    let rolled_back = rollback_runtime_payload_in(&store, Platform::Macos).unwrap();
    assert_eq!(rolled_back.version, "2.7.0");
    assert_eq!(rolled_back.previous_version.as_deref(), Some("2.7.1"));
    assert_eq!(rolled_back.generation, 3);
    let status = runtime_payload_status_in(&store).unwrap();
    assert_eq!(status.active_version.as_deref(), Some("2.7.0"));
    assert_eq!(status.installed_versions, ["2.7.0", "2.7.1"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_lock_blocks_parallel_runtime_activation() {
    let root = fixture_root("update-lock");
    let payload = root.join("payload");
    let store = root.join("store");
    write_payload(&payload, "2.7.0");
    fs::create_dir_all(&store).unwrap();
    fs::write(store.join(".update.lock"), "pid=other").unwrap();

    let error = install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap_err();
    assert!(error.contains("update lock is unavailable"), "{error}");
    assert!(!store.join("versions/2.7.0").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_an_existing_version_with_rewritten_manifest_identity() {
    let root = fixture_root("manifest-identity");
    let payload = root.join("payload");
    let store = root.join("store");
    write_payload(&payload, "2.7.0");
    install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap();
    let installed_manifest = store.join("versions/2.7.0/manifests/runtime-payload.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&installed_manifest).unwrap()).unwrap();
    value["version"] = serde_json::Value::String("2.7.9".to_string());
    fs::write(
        &installed_manifest,
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();

    let error = install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap_err();
    assert!(error.contains("different content"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rollback_rejects_a_rewritten_runtime_identity() {
    let root = fixture_root("rollback-identity");
    let first = root.join("first");
    let second = root.join("second");
    let store = root.join("store");
    write_payload(&first, "2.7.0");
    write_payload(&second, "2.7.1");
    install_runtime_payload_into(&first, &store, Platform::Macos).unwrap();
    install_runtime_payload_into(&second, &store, Platform::Macos).unwrap();
    let installed_manifest = store.join("versions/2.7.0/manifests/runtime-payload.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&installed_manifest).unwrap()).unwrap();
    value["version"] = serde_json::Value::String("2.7.9".to_string());
    fs::write(
        &installed_manifest,
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();

    let error = rollback_runtime_payload_in(&store, Platform::Macos).unwrap_err();
    assert!(error.contains("activation identity mismatch"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn rejects_a_symlinked_runtime_version_target() {
    use std::os::unix::fs::symlink;

    let root = fixture_root("symlink-target");
    let payload = root.join("payload");
    let store = root.join("store");
    let outside = root.join("outside");
    write_payload(&payload, "2.7.0");
    fs::create_dir_all(store.join("versions")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, store.join("versions/2.7.0")).unwrap();

    let error = install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap_err();
    assert!(
        error.contains("version target must not be a symlink"),
        "{error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_tampered_payload_before_installation() {
    let root = fixture_root("tamper");
    let payload = root.join("payload");
    let store = root.join("store");
    write_payload(&payload, "2.7.0");
    fs::write(payload.join("bin/kyuubiki-cli"), "tampered").unwrap();

    let error = install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap_err();
    assert!(error.contains("digest mismatch"), "{error}");
    assert!(!store.join("versions/2.7.0").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_cross_platform_payload_activation() {
    let root = fixture_root("platform");
    let payload = root.join("payload");
    write_payload(&payload, "2.7.0");
    let error =
        install_runtime_payload_into(&payload, &root.join("store"), Platform::Linux).unwrap_err();
    assert!(error.contains("targets macos"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ignores_mutable_runtime_state_when_sealing_and_installing() {
    let root = fixture_root("mutable-state");
    let payload = root.join("payload");
    let store = root.join("store");
    write_payload(&payload, "2.7.0");
    for relative in [
        "data/runtime.sqlite3",
        "exports/result.json",
        "logs/orchestrator.log",
        "run/frontend.log",
    ] {
        let path = payload.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "created after sealing").unwrap();
    }

    let active = install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap();
    assert_eq!(active.version, "2.7.0");
    for root_name in ["data", "exports", "logs", "run"] {
        assert!(
            !store.join("versions/2.7.0").join(root_name).exists(),
            "{root_name} should not be copied into an immutable version"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mutable_state_present_before_sealing_is_not_signed() {
    let root = fixture_root("mutable-before-seal");
    let payload = root.join("payload");
    fs::create_dir_all(payload.join("run")).unwrap();
    fs::write(payload.join("run/frontend.log"), "before").unwrap();
    write_payload(&payload, "2.7.0");
    fs::write(payload.join("run/frontend.log"), "after").unwrap();

    install_runtime_payload_into(&payload, &root.join("store"), Platform::Macos).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_mutable_state_in_an_installed_version() {
    let root = fixture_root("installed-mutable-state");
    let payload = root.join("payload");
    let store = root.join("store");
    write_payload(&payload, "2.7.0");
    install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap();
    fs::create_dir_all(store.join("versions/2.7.0/run")).unwrap();

    let error = install_runtime_payload_into(&payload, &store, Platform::Macos).unwrap_err();
    assert!(error.contains("contains mutable `run` state"), "{error}");
    fs::remove_dir_all(root).unwrap();
}
