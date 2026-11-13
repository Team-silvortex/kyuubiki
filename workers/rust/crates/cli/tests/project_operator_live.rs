mod support;
use kyuubiki_installer::{install_operator_package_into, project_operator_in};
use kyuubiki_protocol::{ProjectOperatorOperation as Op, ProjectOperatorRequest};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use support::operator_package::{LiveAgent, template_package_root};

#[test]
#[ignore = "requires prebuilt trusted template cdylib and local Agent sockets"]
fn installed_project_operator_executes_on_agent_and_recovers_without_replay() {
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-project-operator-live-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _cleanup = Cleanup(root.clone());
    let store = root.join("packages");
    install_operator_package_into(&template_package_root(), &store).unwrap();
    let request = |operation| ProjectOperatorRequest {
        project_id: "material-research".into(),
        scope: "a".repeat(64),
        operation,
    };
    let listed = project_operator_in(&store, request(Op::List)).unwrap();
    let pin = listed.installed[0].clone();
    let prepare = || {
        request(Op::Prepare {
            package: pin.clone(),
            operator_id: "extract.template_summary".into(),
            task_id: "project-research".into(),
            input: json!({"values":[2,4,8]}),
            config: json!({}),
        })
    };
    assert!(project_operator_in(&store, prepare()).is_err());
    project_operator_in(
        &store,
        request(Op::Enable {
            package: pin.clone(),
        }),
    )
    .unwrap();
    let task = project_operator_in(&store, prepare())
        .unwrap()
        .task
        .unwrap();
    let state = root.join("executions");
    let agent = LiveAgent::start_durable(&PathBuf::from(&listed.packages_root), &state).unwrap();
    let descriptor = agent
        .request("describe", "describe_agent", json!({}))
        .unwrap();
    let owner = descriptor["result"]["durable_execution"]["store_id"].clone();
    assert!(owner.is_string());
    let id = "a".repeat(32);
    let submitted=agent.request("submit","submit_durable_agent_task",json!({"execution_id":id,
        "expected_store_id":owner,"method":"run_operator_task_ir","params":{"mode":"execute","task_ir":task}})).unwrap();
    assert_eq!(submitted["ok"], true, "{submitted}");
    let deadline = Instant::now() + Duration::from_secs(15);
    let completed = loop {
        let value = agent
            .request(
                "inspect",
                "fetch_durable_agent_execution",
                json!({"execution_id":id,"expected_store_id":owner}),
            )
            .unwrap();
        assert_eq!(value["ok"], true, "{value}");
        if value["result"]["status"] != "running" {
            break value["result"].clone();
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(completed["status"], "completed", "{completed}");
    let result = &completed["response"]["result"];
    assert_eq!(
        result["execution_runtime_status"],
        "external_operator_package_executed"
    );
    assert_eq!(
        result["operator_package_execution"]["entrypoint_sha256"],
        pin.entrypoint_sha256
    );
    assert_eq!(result["result"]["summary"]["sum"], 14.0);
    // Kill the real Agent, retain its own durable store and retrieve the original receipt.
    drop(agent);
    let restarted =
        LiveAgent::start_durable(&PathBuf::from(&listed.packages_root), &state).unwrap();
    let recovered = restarted
        .request(
            "recover",
            "fetch_durable_agent_execution",
            json!({"execution_id":id,"expected_store_id":owner}),
        )
        .unwrap();
    assert_eq!(recovered["result"], completed);
    project_operator_in(
        &store,
        request(Op::Disable {
            package: pin.clone(),
        }),
    )
    .unwrap();
    assert!(project_operator_in(&store, prepare()).is_err());
    let still_available: Value = restarted
        .request(
            "recover-disabled",
            "fetch_durable_agent_execution",
            json!({"execution_id":id,"expected_store_id":owner}),
        )
        .unwrap();
    assert_eq!(still_available["result"], completed);
    drop(restarted);
}
struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
