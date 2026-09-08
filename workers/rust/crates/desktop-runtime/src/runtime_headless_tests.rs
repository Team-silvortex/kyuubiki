use super::{Platform, installed_paths};
use serde_json::json;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn installed_headless_layout_requires_an_explicit_consistent_profile() {
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-headless-layout-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("manifests")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("bin/agent"), "agent").unwrap();
    fs::write(root.join("bin/orchestra"), "orchestra").unwrap();
    fs::write(
        root.join("manifests/runtime-payload.json"),
        serde_json::to_vec(&json!({
            "schema_version": "kyuubiki.runtime-payload/v1", "version": "3.1.0",
            "platform": Platform::current().as_str()
        }))
        .unwrap(),
    )
    .unwrap();
    let manifest_path = root.join("manifests/service-launch.json");
    let mut manifest = json!({"schema_version": "kyuubiki.service-launch/v1", "services": [
        {"id": "agent", "command": "bin/agent", "cwd": "."},
        {"id": "orchestrator", "command": "bin/orchestra", "cwd": "."}
    ]});
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(installed_paths(root.clone()).is_err());
    manifest["profile"] = "headles".into();
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(installed_paths(root.clone()).is_err());
    manifest["profile"] = "headless".into();
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let paths = installed_paths(root.clone()).unwrap();
    assert!(paths.is_headless());
    assert!(paths.service("agent", &[]).is_ok());
    assert!(paths.service("orchestrator", &[]).is_ok());
    assert!(paths.service("frontend", &[]).is_err());
    manifest["services"].as_array_mut().unwrap().push(json!({
        "id": "frontend", "command": "bin/agent", "cwd": "."
    }));
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(installed_paths(root.clone()).is_err());
    fs::remove_dir_all(root).unwrap();
}
