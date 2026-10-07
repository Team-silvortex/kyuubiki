use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{assert_failure, count, document, run_cli};
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    HeadlessExecutor, ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::error::Error;

#[test]
fn real_submission_association_loss_stops_sdk_and_cli_without_replaying_computation()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let project = direct
        .execute_step(
            "project_create",
            1,
            &json!({"name":"submit-context-parent"}),
        )
        .map_err(|e| e.message)?
        .result;
    let model = direct
        .execute_step(
            "model_create",
            1,
            &json!({"project_id":project["project_id"],
        "name":"submission-context-bar","kind":"axial_bar_1d","payload":{
        "model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d","name":"bar",
        "material":"steel","length":1.0,"area":0.01,"youngs_modulus_gpa":210.0,
        "elements":4,"tip_force":1000.0}}),
        )
        .map_err(|e| e.message)?
        .result;
    let version_id = &model["latest_version_id"];
    let version = http_json(
        server.port,
        &format!("/api/v1/model-versions/{}", version_id.as_str().unwrap()),
        None,
    )?
    .1;

    let mut wrong = version["version"]["payload"].clone();
    wrong["project_id"] = json!("different-project");
    wrong["model_version_id"] = version_id.clone();
    let rejected = http_json(server.port, "/api/v1/fem/axial-bar/jobs", Some(wrong))?;
    assert_eq!(
        rejected,
        (422, json!({"error":":model_version_project_mismatch"}))
    );
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"],
        json!([])
    );
    assert_eq!(count(&owner)? + count(&peer)?, 0);

    let mut ids = Vec::new();
    for (index, missing) in ["project_id", "model_version_id"].into_iter().enumerate() {
        let proxy = AckLossProxy::http_without_record_identity(
            server.port,
            "POST",
            "/api/v1/fem/axial-bar/jobs",
            "job",
            missing,
        )?;
        let url = format!("http://127.0.0.1:{}", proxy.port);
        let force = 1000.0 * (index + 1) as f64;
        let mut bar = version["version"]["payload"].clone();
        bar["tip_force"] = json!(force);
        let batch = document(
            "solve_bar_1d",
            json!({"model":bar,"project_id":project["project_id"],"model_version_id":version_id}),
        );
        if index == 0 {
            let report = execute_batch_with_executor(
                &batch,
                &mut ServiceHeadlessExecutor::new(&url),
                false,
                false,
            );
            let report = serde_json::to_value(report)?;
            assert_failure(&report);
            assert!(
                report["execution_summary"]["failure"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("job_receipt_invalid:")
            );
        } else {
            // A different force is a separate explicit experiment, not a retry of the unknown write.
            run_cli(&batch, &proxy)?;
        }
        let capture = proxy.capture();
        assert!(capture.errors.is_empty(), "{capture:?}");
        assert_eq!(capture.requests.len(), 1);
        assert_eq!(capture.discarded.len(), 1);
        assert_eq!(capture.replies.len(), 1);
        let original = &capture.discarded[0];
        assert_eq!(original["job"]["project_id"], project["project_id"]);
        assert_eq!(original["job"]["model_version_id"], *version_id);
        let mut expected = original.clone();
        expected["job"].as_object_mut().unwrap().remove(missing);
        assert_eq!(capture.replies[0], expected);
        // Discarded replies are independent test oracles, not a recovery API exposed to SDK users.
        let id = original["job"]["job_id"].as_str().unwrap().to_owned();
        direct
            .execute_step(
                "job_wait",
                1,
                &json!({"job_id":id,"project_id":project["project_id"],
            "model_version_id":version_id,"timeout_ms":5000,"interval_ms":10}),
            )
            .map_err(|e| e.message)?;
        let result = direct
            .execute_step(
                "result_fetch",
                1,
                &json!({"job_id":id,"project_id":project["project_id"],
            "model_version_id":version_id}),
            )
            .map_err(|e| e.message)?
            .result;
        let displacement = result["result"]["tip_displacement"].as_f64().unwrap();
        assert!((displacement / (force / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
        ids.push(id);
        wait_for_lifecycle(&owner, "accepting", 0)?;
        wait_for_lifecycle(&peer, "accepting", 0)?;
        assert_eq!(count(&owner)? + count(&peer)?, (index + 1) as u64);
    }
    assert_ne!(ids[0], ids[1]);
    let jobs: Value = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs["jobs"].as_array().unwrap().len(), 2);
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
    assert!(!projects.to_string().contains("must-not-follow-lost-ack"));
    assert_eq!(
        http_json(
            server.port,
            &format!("/api/v1/model-versions/{}", version_id.as_str().unwrap()),
            None
        )?
        .1,
        version
    );
    Ok(())
}
