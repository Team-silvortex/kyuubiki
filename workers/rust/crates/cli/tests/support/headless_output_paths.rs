use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::{error::Error, fs};

#[test]
fn conflicting_outputs_stop_before_project_mutation_and_real_agent_dispatch()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let project = executor
        .execute_step("project_create", 1, &json!({"name":"path-guard-parent"}))
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({
                "project_id":project["project_id"],"name":"path-guard-bar","kind":"axial_bar_1d",
                "payload":{"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
                    "name":"path-guard-bar","material":"steel","length":1.0,"area":0.01,
                    "youngs_modulus_gpa":210.0,"elements":64,"tip_force":1000.0}
            }),
        )
        .map_err(|error| error.message)?
        .result;
    let source = write_temp_json(
        "path-guard-run",
        &json!({
            "schema_version":"kyuubiki.headless-workflow/v1","language":"en",
            "exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"path-guard-run","steps":[
                {"action":"project_create","payload":{"name":"must-not-be-created"}},
                {"action":"solve_and_wait_from_model_version","payload":{
                    "project_id":project["project_id"],"model_version_id":model["latest_version_id"],
                    "endpoints":[format!("127.0.0.1:{}",owner.port())],"timeout_ms":5000,"interval_ms":10
                }}
            ]}
        }),
    );
    let original = fs::read(&*source)?;
    let projects_before = http_json(server.port, "/api/v1/projects", None)?.1;
    let jobs_before = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs_before["jobs"].as_array().unwrap().len(), 0);
    let shared = source.parent().unwrap().join("new/shared.json");
    let alias = source.parent().unwrap().join("new/./shared.json");
    for args in [
        vec!["--report-out", source.to_str().unwrap()],
        vec![
            "--report-out",
            shared.to_str().unwrap(),
            "--material-report-out",
            alias.to_str().unwrap(),
        ],
    ] {
        let mut command = vec![
            "run",
            source.to_str().unwrap(),
            "--execute",
            "--executor",
            "service",
            "--api-base-url",
            &url,
            "--json",
        ];
        command.extend(args);
        let output = run_headless_command(&command);
        assert!(!output.status.success(), "{}", server.logs());
        let report = parse_json_output(&output);
        assert_eq!(report["status"], "invalid");
        assert_eq!(report["executed_step_count"], 0);
        assert_eq!(report["steps"], json!([]));
        let diagnostic: Value = serde_json::from_slice(&output.stderr)?;
        assert_eq!(diagnostic["error"]["code"], "output_path_conflict");
        assert_eq!(diagnostic["error"]["stage"], "command_validation");
        assert_eq!(diagnostic["error"]["retryable"], false);
        assert_eq!(fs::read(&*source)?, original);
        assert_eq!(
            http_json(server.port, "/api/v1/projects", None)?.1,
            projects_before
        );
        assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs_before);
        assert!(!source.parent().unwrap().join("new").exists());
        assert_eq!(fs::read_dir(source.parent().unwrap())?.count(), 1);
        wait_for_lifecycle(&owner, "accepting", 0)?;
        wait_for_lifecycle(&peer, "accepting", 0)?;
        assert_eq!(
            count(&owner)? + count(&peer)?,
            0,
            "path conflict dispatched computation"
        );
    }
    Ok(())
}
