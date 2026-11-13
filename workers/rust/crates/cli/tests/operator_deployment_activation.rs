mod support;
use kyuubiki_installer::{
    Platform, install_agent_update_package_into, install_operator_package_into,
    prepare_agent_update_package, project_operator_in, uninstall_operator_package_from,
};
use kyuubiki_protocol::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use support::operator_package::{
    LiveAgent, template_library_path, template_package_root, template_packages_root,
};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires trusted template cdylib and local Agent sockets"]
fn failed_activation_keeps_control_available_and_recovers_only_after_explicit_repair() {
    let f = Fixture(std::env::temp_dir().join(format!(
            "kyuubiki-activation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    let agent_package = f.0.join("agent-package");
    let agent_store = f.0.join("agent-store");
    prepare_agent_update_package(
        Path::new(env!("CARGO_BIN_EXE_kyuubiki-cli")),
        &agent_package,
        env!("CARGO_PKG_VERSION"),
        Platform::current(),
    )
    .unwrap();
    install_agent_update_package_into(&agent_package, &agent_store, Platform::current()).unwrap();
    let source = f.0.join("source");
    fs::create_dir_all(&source).unwrap();
    let mut manifest: Value = serde_json::from_slice(
        &fs::read(template_package_root().join("kyuubiki-operator.json")).unwrap(),
    )
    .unwrap();
    let file = format!("operator{}", std::env::consts::DLL_SUFFIX);
    manifest["entrypoint"] = json!(file);
    let real_symbol = manifest["operators"][0]["entry_symbol"].clone();
    manifest["operators"][0]["entry_symbol"] = json!("missing_qualification_symbol");
    fs::write(
        source.join("kyuubiki-operator.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::copy(
        template_library_path(&template_packages_root()),
        source.join(file),
    )
    .unwrap();
    let store = f.0.join("operator-store");
    install_operator_package_into(&source, &store).unwrap();
    let pin = || {
        project_operator_in(
            &store,
            ProjectOperatorRequest {
                platform: None,
                project_id: "recovery".into(),
                scope: "a".repeat(64),
                operation: ProjectOperatorOperation::List,
            },
        )
        .unwrap()
        .installed
        .remove(0)
    };
    let broken = pin();
    let packages = store.join("packages");
    let bound = std::process::Command::new(env!("CARGO_BIN_EXE_kyuubiki-cli"))
        .env("KYUUBIKI_AGENT_PACKAGE_DEPLOYMENT", "true")
        .args([
            "agent",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            "--orchestrator-url",
            "http://127.0.0.1:1",
            "--operator-packages-root",
            packages.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!bound.status.success());
    assert!(
        String::from_utf8_lossy(&bound.stderr)
            .contains("bound Orchestra agents fetch from their controller")
    );
    assert!(
        LiveAgent::start(&packages).is_err(),
        "default startup must remain strict"
    );
    let agent = LiveAgent::start_managed_deployment(&packages, &agent_store)
        .expect("explicit deployment opt-in must retain a diagnostic control endpoint");
    let describe = agent
        .request("describe", "describe_agent", json!({}))
        .unwrap();
    assert_eq!(
        describe["result"]["operator_package_runtime"]["ready"],
        false
    );
    assert_eq!(
        describe["result"]["operator_package_runtime"]["status"],
        "activation_failed"
    );
    assert_eq!(describe["result"]["deployment_readiness"]["ready"], false);
    let exchange = |agent: &LiveAgent, request: OperatorDeploymentRequest| {
        let reply = agent
            .request(
                "package",
                "manage_operator_package",
                serde_json::to_value(request).unwrap(),
            )
            .unwrap();
        assert_eq!(reply["ok"], true, "{reply}");
        serde_json::from_value::<OperatorDeploymentStatus>(reply["result"].clone())
            .map_err(|e| e.to_string())
    };
    let failed = kyuubiki_headless_sdk::deploy_operator_package(
        &broken,
        &kyuubiki_operator_sdk::current_platform_target_id(),
        |r| exchange(&agent, r),
        |_| panic!("failed activation is not permission to upload or replace"),
        || true,
        |_| {},
    )
    .unwrap();
    assert_eq!(failed.state, "activation_failed");
    assert_eq!(
        serde_json::to_value(&failed).unwrap()["activation_error"],
        "operator_package_activation_failed"
    );
    assert_eq!(failed.installed.as_ref(), Some(&broken));
    assert!(failed.active.is_none());
    let task = build_project_operator_task(
        "recovery",
        &broken,
        "extract.template_summary",
        "activation-probe",
        json!({"values":[2,4,8]}),
        json!({}),
    )
    .unwrap();
    let rejected = agent
        .request(
            "run",
            "run_operator_task_ir",
            json!({"mode":"execute","task_ir":task}),
        )
        .unwrap();
    assert_eq!(rejected["ok"], false, "{rejected}");
    assert_eq!(
        rejected["error"]["code"],
        "operator_package_activation_failed"
    );
    let original_process = failed.process_instance_id.clone();
    drop(agent);
    let entrypoint = packages
        .join(&broken.package_id)
        .join(manifest["entrypoint"].as_str().unwrap());
    let bytes = fs::read(&entrypoint).unwrap();
    fs::write(&entrypoint, b"tampered before restart").unwrap();
    let damaged = LiveAgent::start_managed_deployment(&packages, &agent_store).unwrap();
    let diagnostic = damaged
        .request("describe", "describe_agent", json!({}))
        .unwrap();
    assert_eq!(
        diagnostic["result"]["operator_package_runtime"]["activation_error"],
        "operator_package_integrity_failed"
    );
    assert_eq!(
        diagnostic["result"]["deployment_readiness"]["operator_package_runtime_ready"],
        false
    );
    // Restoring disk bytes cannot silently clear this process's startup failure.
    fs::write(&entrypoint, bytes).unwrap();
    let still_failed = exchange(
        &damaged,
        OperatorDeploymentRequest {
            package: broken.clone(),
            expected_process_instance_id: None,
            operation: OperatorDeploymentAction::Inspect,
        },
    )
    .unwrap();
    assert_eq!(still_failed.state, "activation_failed");
    assert_eq!(
        still_failed.activation_error.as_deref(),
        Some("operator_package_integrity_failed")
    );
    drop(damaged);
    // Repair requires explicit stopped removal and reinstall; no remote overwrite or retry loop.
    uninstall_operator_package_from(&store, &broken.package_id).unwrap();
    manifest["operators"][0]["entry_symbol"] = real_symbol;
    fs::write(
        source.join("kyuubiki-operator.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    install_operator_package_into(&source, &store).unwrap();
    let repaired = pin();
    assert_ne!(repaired.manifest_sha256, broken.manifest_sha256);
    let agent = LiveAgent::start_managed_deployment(&packages, &agent_store).unwrap();
    let ready = exchange(
        &agent,
        OperatorDeploymentRequest {
            package: repaired.clone(),
            expected_process_instance_id: None,
            operation: OperatorDeploymentAction::Inspect,
        },
    )
    .unwrap();
    assert_eq!(ready.state, "ready");
    assert_ne!(ready.process_instance_id, original_process);
    let stale = exchange(
        &agent,
        OperatorDeploymentRequest {
            package: broken,
            expected_process_instance_id: None,
            operation: OperatorDeploymentAction::Inspect,
        },
    )
    .unwrap();
    assert_eq!(stale.state, "version_conflict");
    let task = build_project_operator_task(
        "recovery",
        &repaired,
        "extract.template_summary",
        "repaired-probe",
        json!({"values":[2,4,8]}),
        json!({}),
    )
    .unwrap();
    let result = agent
        .request(
            "run",
            "run_operator_task_ir",
            json!({"mode":"execute","task_ir":task}),
        )
        .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["result"]["result"]["summary"]["sum"], 14.0);
}
