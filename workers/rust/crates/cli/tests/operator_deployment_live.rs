mod support;
use kyuubiki_headless_sdk::deploy_operator_package;
use kyuubiki_installer::{
    Platform, active_agent_binary_in, install_agent_update_package_into,
    install_operator_package_into, prepare_agent_update_package, project_operator_in,
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
#[test]
#[ignore = "requires trusted prebuilt cdylib and local Agent sockets"]
fn deploy_to_empty_agent_resume_after_kill_confirm_loaded_and_reject_conflict() {
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-deployment-live-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _cleanup = Cleanup(root.clone());
    let agent_package = root.join("agent-package");
    let agent_store = root.join("agent-store");
    let sealed = prepare_agent_update_package(
        Path::new(env!("CARGO_BIN_EXE_kyuubiki-cli")),
        &agent_package,
        env!("CARGO_PKG_VERSION"),
        Platform::current(),
    )
    .unwrap();
    let activation =
        install_agent_update_package_into(&agent_package, &agent_store, Platform::current())
            .unwrap();
    assert_eq!(activation.entrypoint_sha256, sealed.entrypoint_sha256);
    assert!(
        active_agent_binary_in(&agent_store, Platform::current())
            .unwrap()
            .starts_with(&agent_store)
    );
    let source = root.join("source");
    fs::create_dir_all(&source).unwrap();
    let mut manifest: Value = serde_json::from_slice(
        &fs::read(template_package_root().join("kyuubiki-operator.json")).unwrap(),
    )
    .unwrap();
    let filename = format!("operator{}", std::env::consts::DLL_SUFFIX);
    manifest["entrypoint"] = json!(filename);
    fs::write(
        source.join("kyuubiki-operator.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::copy(
        template_library_path(&template_packages_root()),
        source.join(&filename),
    )
    .unwrap();
    let local = root.join("local");
    install_operator_package_into(&source, &local).unwrap();
    let req = |operation| ProjectOperatorRequest {
        platform: None,
        project_id: "research".into(),
        scope: "a".repeat(64),
        operation,
    };
    let pin = project_operator_in(&local, req(ProjectOperatorOperation::List))
        .unwrap()
        .installed
        .remove(0);
    project_operator_in(
        &local,
        req(ProjectOperatorOperation::Enable {
            package: pin.clone(),
        }),
    )
    .unwrap();
    let chunk = |offset| {
        project_operator_in(
            &local,
            req(ProjectOperatorOperation::ReadDeployment {
                package: pin.clone(),
                offset,
            }),
        )
        .unwrap()
        .deployment
        .unwrap()
    };
    let remote = root.join("remote/packages");
    fs::create_dir_all(&remote).unwrap();
    let agent = LiveAgent::start_managed_deployment(&remote, &agent_store).unwrap();
    let call = |agent: &LiveAgent,
                pin: &ProjectOperatorPin,
                process: Option<&str>,
                operation: OperatorDeploymentAction| {
        agent
            .request(
                "deployment",
                "manage_operator_package",
                serde_json::to_value(OperatorDeploymentRequest {
                    package: pin.clone(),
                    expected_process_instance_id: process.map(str::to_owned),
                    operation,
                })
                .unwrap(),
            )
            .unwrap()
    };
    let inspect = call(&agent, &pin, None, OperatorDeploymentAction::Inspect);
    assert_eq!(inspect["ok"], true, "{inspect}");
    assert_eq!(inspect["result"]["state"], "missing");
    let old_process = inspect["result"]["process_instance_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = chunk(0);
    assert_eq!(
        call(
            &agent,
            &pin,
            Some(&old_process),
            OperatorDeploymentAction::Begin {
                spec: first.spec.clone()
            }
        )["ok"],
        true
    );
    assert_eq!(
        call(
            &agent,
            &pin,
            Some(&old_process),
            OperatorDeploymentAction::Chunk {
                offset: 0,
                data: first.data.clone()
            }
        )["ok"],
        true
    );
    drop(agent); // Real process termination, not an in-memory recreation.
    let agent = LiveAgent::start_managed_deployment(&remote, &agent_store).unwrap();
    assert_eq!(
        call(
            &agent,
            &pin,
            Some(&old_process),
            OperatorDeploymentAction::Commit
        )["error"]["code"],
        "deployment_agent_changed"
    );
    let resumed = call(&agent, &pin, None, OperatorDeploymentAction::Inspect);
    assert_eq!(resumed["result"]["received_bytes"], first.data.len());
    let mut commits = 0;
    let mut exchange =
        |request: OperatorDeploymentRequest| -> Result<OperatorDeploymentStatus, String> {
            let commit = matches!(request.operation, OperatorDeploymentAction::Commit);
            let reply = call(
                &agent,
                &request.package,
                request.expected_process_instance_id.as_deref(),
                request.operation,
            );
            assert_eq!(reply["ok"], true, "{reply}");
            if commit {
                commits += 1;
                // The Agent committed to disk; deliberately discard its transport response.
                return Err("test_commit_response_lost".into());
            }
            serde_json::from_value(reply["result"].clone()).map_err(|e| e.to_string())
        };
    let mut progress = Vec::new();
    let uncertain = deploy_operator_package(
        &pin,
        &first.spec.platform,
        &mut exchange,
        |offset| Ok(chunk(offset)),
        || true,
        |status| progress.push(status.received_bytes),
    );
    assert_eq!(uncertain.unwrap_err(), "test_commit_response_lost");
    assert_eq!(progress[0], first.data.len() as u64);
    assert_eq!(progress.last(), Some(&first.spec.size_bytes));
    let committed = deploy_operator_package(
        &pin,
        &first.spec.platform,
        &mut exchange,
        |_| panic!("committed package must not be re-uploaded"),
        || true,
        |_| {},
    )
    .unwrap();
    assert_eq!(committed.state, "installed_restart_required");
    assert_eq!(commits, 1);
    assert_eq!(
        call(&agent, &pin, None, OperatorDeploymentAction::Inspect)["result"]["state"],
        "installed_restart_required"
    );
    drop(agent);
    let agent = LiveAgent::start_managed_deployment(&remote, &agent_store).unwrap();
    let ready = call(&agent, &pin, None, OperatorDeploymentAction::Inspect);
    assert_eq!(ready["result"]["state"], "ready", "{ready}");
    let task = build_project_operator_task(
        "research",
        &pin,
        "extract.template_summary",
        "remote-run",
        json!({"values":[2,4,8]}),
        json!({}),
    )
    .unwrap();
    let executed = agent
        .request(
            "execute",
            "run_operator_task_ir",
            json!({"mode":"execute","task_ir":task}),
        )
        .unwrap();
    assert_eq!(
        executed["result"]["result"]["summary"]["sum"], 14.0,
        "{executed}"
    );
    let mut changed = pin.clone();
    changed.package_version = "99.0.0".into();
    assert_eq!(
        call(&agent, &changed, None, OperatorDeploymentAction::Inspect)["result"]["state"],
        "version_conflict"
    );
    drop(agent);
    let protected = LiveAgent::start(&remote).unwrap();
    let status = call(&protected, &pin, None, OperatorDeploymentAction::Inspect);
    let process = status["result"]["process_instance_id"].as_str().unwrap();
    assert_eq!(status["result"]["deployment_allowed"], false);
    assert_eq!(
        call(
            &protected,
            &pin,
            Some(process),
            OperatorDeploymentAction::Abort
        )["error"]["code"],
        "deployment_disabled"
    );
}
struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
