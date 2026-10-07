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
fn invalid_research_plans_leave_real_projects_jobs_and_agent_calculations_untouched()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let project = executor
        .execute_step("project_create", 1, &json!({"name":"preflight-parent"}))
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({
                "project_id":project["project_id"],"name":"preflight-bar","kind":"axial_bar_1d",
                "payload":{"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
                    "name":"preflight-bar","material":"steel","length":1.0,"area":0.01,
                    "youngs_modulus_gpa":210.0,"elements":64,"tip_force":1000.0}
            }),
        )
        .map_err(|error| error.message)?
        .result;
    let source = write_temp_json(
        "research-preflight",
        &json!({
            "schema_version":"kyuubiki.headless-workflow/v1","language":"en",
            "exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"research-preflight","steps":[
                {"action":"project_create","payload":{"name":"must-not-be-created"}},
                {"action":"solve_and_wait_from_model_version","payload":{
                    "project_id":project["project_id"],"model_version_id":model["latest_version_id"],
                    "endpoints":[format!("127.0.0.1:{}",owner.port())],"timeout_ms":5000,"interval_ms":10
                }}
            ]}
        }),
    );
    let root = source.parent().unwrap();
    let source_bytes = fs::read(&*source)?;
    let spec_path = root.join("spec.json");
    let patch_path = root.join("patch.json");
    let evidence_path = root.join("evidence.json");
    let report_path = root.join("run.json");
    let retained_evidence = b"{\"old_output\":true}";
    fs::write(&evidence_path, retained_evidence)?;
    let patch = json!({
        "schema_version":"kyuubiki.headless-parameter-patch/v1","patch_id":"invalid-baseline-patch",
        "workflow_id":"research-preflight","changes":[{
            "path":"/steps/0/payload/name","expected":"must-not-be-created","value":"also-must-not-be-created"
        }]
    });
    fs::write(&patch_path, serde_json::to_vec_pretty(&patch)?)?;
    let patch_bytes = fs::read(&patch_path)?;
    let projects_before = http_json(server.port, "/api/v1/projects", None)?.1;
    let jobs_before = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs_before["jobs"].as_array().unwrap().len(), 0);

    for (pointer, patched, message) in [
        (
            "/steps/2/result_preview/result/result/tip_displacement",
            false,
            "missing batch step 2",
        ),
        (
            "/steps/1/result_preview/result/result/tip_displacement",
            true,
            "effective baseline",
        ),
    ] {
        let spec = json!({
            "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"preflight-round-1",
            "workflow_id":"research-preflight","iteration":1,"primary_metric_ids":["tip_displacement"],
            "metrics":[{"metric_id":"tip_displacement","pointer":pointer,"unit":"m","objective":"observe"}]
        });
        fs::write(&spec_path, serde_json::to_vec_pretty(&spec)?)?;
        let mut args = vec![
            "run",
            source.to_str().unwrap(),
            "--execute",
            "--executor",
            "service",
            "--execution-posture",
            "research",
            "--api-base-url",
            &url,
            "--json",
            "--report-out",
            report_path.to_str().unwrap(),
            "--research-round-spec",
            spec_path.to_str().unwrap(),
            "--research-round-out",
            evidence_path.to_str().unwrap(),
        ];
        if patched {
            args.extend(["--parameter-patch", patch_path.to_str().unwrap()]);
        }
        let output = run_headless_command(&args);
        assert!(!output.status.success(), "{}", server.logs());
        let report = parse_json_output(&output);
        assert_eq!(report["status"], "invalid", "{report}");
        assert_eq!(report["executed_step_count"], 0);
        assert_eq!(report["steps"], json!([]));
        assert_eq!(report["execution_summary"]["job_count"], 0);
        assert_eq!(
            report["execution_summary"]["failure"]["error_code"],
            "kyuubiki.headless.research_round_validation"
        );
        let diagnostic: Value = serde_json::from_slice(&output.stderr)?;
        assert_eq!(diagnostic["error"]["code"], "research_round_validation");
        assert_eq!(diagnostic["error"]["stage"], "research_round");
        assert_eq!(diagnostic["error"]["retryable"], false);
        assert!(
            diagnostic["error"]["message"]
                .as_str()
                .unwrap()
                .contains(message),
            "{diagnostic}"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&report_path)?)?,
            report
        );
        assert_eq!(fs::read(&*source)?, source_bytes);
        assert_eq!(fs::read(&patch_path)?, patch_bytes);
        assert_eq!(fs::read(&evidence_path)?, retained_evidence);
        assert_eq!(fs::read_dir(root)?.count(), 5);
        assert_eq!(
            http_json(server.port, "/api/v1/projects", None)?.1,
            projects_before
        );
        assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs_before);
        wait_for_lifecycle(&owner, "accepting", 0)?;
        wait_for_lifecycle(&peer, "accepting", 0)?;
        assert_eq!(
            count(&owner)? + count(&peer)?,
            0,
            "invalid research plan dispatched calculation"
        );
    }
    Ok(())
}
