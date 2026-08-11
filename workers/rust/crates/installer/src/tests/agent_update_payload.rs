use crate::agent_update_payload::{
    active_agent_binary_in, agent_update_status_in, install_agent_update_package_into,
    rollback_agent_update_in,
};
use crate::{
    Platform, prepare_agent_update_package, seal_agent_update_package, verify_agent_update_package,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn agent_update_schemas_match_runtime_contracts() {
    let package: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/agent-update-package.schema.json"
    ))
    .unwrap();
    let activation: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/agent-update-activation.schema.json"
    ))
    .unwrap();
    let qualification: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/agent-update-qualification-report.schema.json"
    ))
    .unwrap();
    assert_eq!(
        package["properties"]["schema_version"]["const"],
        crate::AGENT_UPDATE_PACKAGE_SCHEMA_VERSION
    );
    assert_eq!(
        activation["properties"]["schema_version"]["const"],
        crate::AGENT_UPDATE_ACTIVATION_SCHEMA_VERSION
    );
    assert_eq!(
        qualification["properties"]["schema_version"]["const"],
        crate::AGENT_UPDATE_QUALIFICATION_SCHEMA_VERSION
    );
}

#[test]
fn prepares_a_sealed_package_from_a_rust_agent_binary() {
    let root = fixture_root("prepare");
    let binary = root.join("built-agent");
    fs::create_dir_all(&root).unwrap();
    fs::write(&binary, "built-rust-agent").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let platform = Platform::current();
    let manifest =
        prepare_agent_update_package(&binary, &root.join("package"), "2.11.7", platform).unwrap();
    assert_eq!(manifest.version, "2.11.7");
    verify_agent_update_package(&root.join("package"), platform).unwrap();
    let error = prepare_agent_update_package(&binary, &root.join("package"), "2.11.8", platform)
        .unwrap_err();
    assert!(error.contains("must be empty"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

fn fixture_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kyuubiki-agent-update-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn write_package(root: &Path, version: &str, body: &str, platform: Platform) {
    let binary = if platform == Platform::Windows {
        root.join("bin/kyuubiki-agent.exe")
    } else {
        root.join("bin/kyuubiki-agent")
    };
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    }
    seal_agent_update_package(root, version, platform).unwrap();
}

#[test]
fn installs_activates_and_rolls_back_agent_versions() {
    let root = fixture_root("lifecycle");
    let first = root.join("first");
    let second = root.join("second");
    let store = root.join("store");
    let platform = Platform::current();
    write_package(&first, "2.11.7", "agent-v1", platform);
    write_package(&second, "2.11.8", "agent-v2", platform);

    let first_activation = install_agent_update_package_into(&first, &store, platform).unwrap();
    assert_eq!(first_activation.version, "2.11.7");
    assert_eq!(first_activation.generation, 1);
    let second_activation = install_agent_update_package_into(&second, &store, platform).unwrap();
    assert_eq!(second_activation.version, "2.11.8");
    assert_eq!(
        second_activation.previous_version.as_deref(),
        Some("2.11.7")
    );
    assert!(
        active_agent_binary_in(&store, platform)
            .unwrap()
            .starts_with(store.join("versions/2.11.8"))
    );

    let rollback = rollback_agent_update_in(&store, platform).unwrap();
    assert_eq!(rollback.version, "2.11.7");
    assert_eq!(rollback.previous_version.as_deref(), Some("2.11.8"));
    let status = agent_update_status_in(&store).unwrap();
    assert_eq!(status.active_version.as_deref(), Some("2.11.7"));
    assert_eq!(status.installed_versions, ["2.11.7", "2.11.8"]);
    assert!(
        active_agent_binary_in(&store, platform)
            .unwrap()
            .starts_with(store.join("versions/2.11.7"))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn active_agent_resolver_rejects_tampered_activation() {
    let root = fixture_root("activation-tamper");
    let package = root.join("package");
    let store = root.join("store");
    let platform = Platform::current();
    write_package(&package, "2.11.7", "agent-v1", platform);
    let activation = install_agent_update_package_into(&package, &store, platform).unwrap();
    let path = store
        .join("activations")
        .join(format!("{:020}.json", activation.generation));
    let mut payload: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    payload["relative_path"] = "../outside".into();
    fs::write(&path, serde_json::to_vec_pretty(&payload).unwrap()).unwrap();

    let error = active_agent_binary_in(&store, platform).unwrap_err();
    assert!(error.contains("path is not canonical"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_tampered_agent_package_before_staging() {
    let root = fixture_root("tamper");
    let package = root.join("package");
    let store = root.join("store");
    let platform = Platform::current();
    write_package(&package, "2.11.7", "agent-v1", platform);
    let binary = if platform == Platform::Windows {
        package.join("bin/kyuubiki-agent.exe")
    } else {
        package.join("bin/kyuubiki-agent")
    };
    fs::write(binary, "tampered").unwrap();

    let error = install_agent_update_package_into(&package, &store, platform).unwrap_err();
    assert!(error.contains("size mismatch") || error.contains("digest mismatch"));
    assert!(!store.join("versions/2.11.7").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_cross_platform_and_undeclared_payloads() {
    let root = fixture_root("boundaries");
    let package = root.join("package");
    let platform = Platform::current();
    write_package(&package, "2.11.7", "agent-v1", platform);
    fs::write(package.join("unexpected.txt"), "not declared").unwrap();
    let error = verify_agent_update_package(&package, platform).unwrap_err();
    assert!(error.contains("undeclared files"), "{error}");
    fs::remove_file(package.join("unexpected.txt")).unwrap();

    let other = if platform == Platform::Linux {
        Platform::Macos
    } else {
        Platform::Linux
    };
    let error = verify_agent_update_package(&package, other).unwrap_err();
    assert!(error.contains("current platform"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_lock_blocks_parallel_agent_activation() {
    let root = fixture_root("lock");
    let package = root.join("package");
    let store = root.join("store");
    let platform = Platform::current();
    write_package(&package, "2.11.7", "agent-v1", platform);
    fs::create_dir_all(&store).unwrap();
    fs::write(store.join(".update.lock"), "pid=fixture").unwrap();

    let error = install_agent_update_package_into(&package, &store, platform).unwrap_err();
    assert!(error.contains("update lock is unavailable"), "{error}");
    assert!(!store.join("versions/2.11.7").exists());
    fs::remove_dir_all(root).unwrap();
}
