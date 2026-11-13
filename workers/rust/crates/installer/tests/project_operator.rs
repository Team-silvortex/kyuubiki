#[path = "store_install.rs"]
mod store_fixture;

use kyuubiki_installer::{install_store_operator_into, project_operator_in};
use serde_json::{Value, json};

fn request(project: &str, action: Value) -> kyuubiki_protocol::ProjectOperatorRequest {
    serde_json::from_value(json!({"project_id":project,"scope":"a".repeat(64),"operation":action}))
        .unwrap()
}

#[test]
fn project_enablement_is_explicit_pinned_isolated_and_reversible() {
    let (f, install) = store_fixture::Fixture::new();
    let root = f.0.join("managed");
    let receipt = install_store_operator_into(&f.0.join("files"), &root, &install).unwrap();
    let list = project_operator_in(&root, request("project-a", json!({"action":"list"}))).unwrap();
    assert!(list.enabled.is_empty());
    let pin = serde_json::to_value(&list.installed[0]).unwrap();
    let prepare = json!({"action":"prepare","package":pin,"operator_id":"extract.store_fixture",
        "task_id":"research-1","input":{"values":[2,4,8]},"config":{}});
    assert!(project_operator_in(&root, request("project-a", prepare.clone())).is_err());
    let enable = json!({"action":"enable","package":pin});
    let enabled = project_operator_in(&root, request("project-a", enable.clone())).unwrap();
    assert_eq!(enabled.enabled[0].state, "ready");
    assert_eq!(
        project_operator_in(&root, request("project-a", enable))
            .unwrap()
            .enabled
            .len(),
        1
    );
    assert!(project_operator_in(&root, request("project-b", prepare.clone())).is_err());
    let mut other_scope = request("project-a", prepare.clone());
    other_scope.scope = "b".repeat(64);
    assert!(project_operator_in(&root, other_scope).is_err());
    let prepared = project_operator_in(&root, request("project-a", prepare.clone())).unwrap();
    let task = prepared.task.unwrap();
    kyuubiki_protocol::verify_operator_task_digest(&task).unwrap();
    assert_eq!(task["orchestration_context"]["project_id"], "project-a");
    assert_eq!(
        task["execution_program"]["package_integrity"]["digest"],
        receipt.entrypoint_sha256
    );
    assert_eq!(task["runtime_hints"]["authority_mode"], "agent_local");
    let mut changed = pin.clone();
    changed["package_version"] = json!("1.0.1");
    assert!(
        project_operator_in(
            &root,
            request("project-a", json!({"action":"enable","package":changed}))
        )
        .is_err()
    );
    let disabled = project_operator_in(
        &root,
        request("project-a", json!({"action":"disable","package":pin})),
    )
    .unwrap();
    assert!(disabled.enabled.is_empty());
    assert!(root.join(&receipt.relative_root).exists());
    assert!(project_operator_in(&root, request("project-a", prepare)).is_err());
}

#[test]
fn changed_installed_bytes_block_prepare_but_allow_explicit_disable() {
    let (f, install) = store_fixture::Fixture::new();
    let root = f.0.join("managed");
    let receipt = install_store_operator_into(&f.0.join("files"), &root, &install).unwrap();
    let list = project_operator_in(&root, request("p", json!({"action":"list"}))).unwrap();
    let pin = serde_json::to_value(&list.installed[0]).unwrap();
    project_operator_in(
        &root,
        request("p", json!({"action":"enable","package":pin})),
    )
    .unwrap();
    std::fs::write(
        root.join(receipt.relative_root)
            .join(receipt.entrypoint_relative_path),
        b"changed",
    )
    .unwrap();
    let status = project_operator_in(&root, request("p", json!({"action":"list"}))).unwrap();
    assert_eq!(status.enabled[0].state, "unavailable_or_changed");
    assert!(status.installed_error.is_some());
    assert!(
        project_operator_in(
            &root,
            request(
                "p",
                json!({"action":"prepare","package":pin,
        "operator_id":"extract.store_fixture","task_id":"t","input":{},"config":{}})
            )
        )
        .is_err()
    );
    assert!(
        project_operator_in(
            &root,
            request("p", json!({"action":"disable","package":pin}))
        )
        .unwrap()
        .enabled
        .is_empty()
    );
}

#[test]
fn project_operator_rejects_unknown_fields_corrupt_journal_and_unsafe_pins() {
    let (f, install) = store_fixture::Fixture::new();
    let root = f.0.join("managed");
    install_store_operator_into(&f.0.join("files"), &root, &install).unwrap();
    let listed = project_operator_in(&root, request("p", json!({"action":"list"}))).unwrap();
    let pin = serde_json::to_value(&listed.installed[0]).unwrap();
    for change in [
        json!({"package_id":"../escape"}),
        json!({"package_version":"latest"}),
        json!({"entrypoint_sha256":"bad"}),
        json!({"operators":[{"operator_id":"extract.fixture","kind":"unknown"}]}),
    ] {
        let mut bad = pin.clone();
        for (k, v) in change.as_object().unwrap() {
            bad[k] = v.clone();
        }
        assert!(
            project_operator_in(
                &root,
                request("p", json!({"action":"enable","package":bad}))
            )
            .is_err()
        );
    }
    assert!(
        serde_json::from_value::<kyuubiki_protocol::ProjectOperatorRequest>(
            json!({"project_id":"p","operation":{"action":"list"},"store_root":"/outside"})
        )
        .is_err()
    );
    project_operator_in(
        &root,
        request("p", json!({"action":"enable","package":pin})),
    )
    .unwrap();
    let file = std::fs::read_dir(root.join("projects"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .unwrap();
    std::fs::write(&file, b"broken").unwrap();
    assert!(project_operator_in(&root, request("p", json!({"action":"list"}))).is_err());
    assert!(
        project_operator_in(
            &root,
            request("p", json!({"action":"enable","package":pin}))
        )
        .is_err()
    );
    assert_eq!(std::fs::read(file).unwrap(), b"broken");
}

#[cfg(unix)]
#[test]
fn project_operator_refuses_symlinked_project_storage() {
    let (f, install) = store_fixture::Fixture::new();
    let root = f.0.join("managed");
    install_store_operator_into(&f.0.join("files"), &root, &install).unwrap();
    let outside = f.0.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("projects")).unwrap();
    assert!(project_operator_in(&root, request("p", json!({"action":"list"}))).is_err());
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
}

#[test]
fn project_operator_cli_persists_enablement_between_processes() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let (f, install) = store_fixture::Fixture::new();
    let root = f.0.join("managed");
    install_store_operator_into(&f.0.join("files"), &root, &install).unwrap();
    let invoke = |operation: Value| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-installer"))
            .args(["project-operator", "--store-root"])
            .arg(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&request("cli-project", operation)).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let listed = invoke(json!({"action":"list"}));
    let pin = listed["installed"][0].clone();
    assert!(listed["enabled"].as_array().unwrap().is_empty());
    invoke(json!({"action":"enable", "package":pin}));
    let prepared = invoke(json!({"action":"prepare", "package":pin,
        "operator_id":"extract.store_fixture", "task_id":"cli-task", "input":{}, "config":{}}));
    assert_eq!(prepared["scope"], "a".repeat(64));
    kyuubiki_protocol::verify_operator_task_digest(&prepared["task"]).unwrap();
    assert_eq!(
        prepared["task"]["orchestration_context"]["project_id"],
        "cli-project"
    );
    let disabled = invoke(json!({"action":"disable", "package":pin}));
    assert!(disabled["enabled"].as_array().unwrap().is_empty());
    assert!(
        invoke(json!({"action":"list"}))["enabled"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
