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
fn real_job_with_corrupted_submission_identity_does_not_replay_or_advance()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let proxy = AckLossProxy::http_without_job_identity(server.port, "/api/v1/fem/axial-bar/jobs")?;
    let url = format!("http://127.0.0.1:{}", proxy.port);
    let payload = json!({"model":{"length":1.0,"area":0.01,"youngs_modulus_gpa":210,
        "elements":4,"tip_force":1000}});
    let report = execute_batch_with_executor(
        &document("solve_bar_1d", payload.clone()),
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
    // The original reply is a test oracle, not a product recovery mechanism.
    let first_id = proxy.capture().discarded[0]["job"]["job_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    direct
        .execute_step(
            "job_wait",
            1,
            &json!({"job_id":first_id,"timeout_ms":5000,"interval_ms":10}),
        )
        .map_err(|error| error.message)?;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);

    let mut second_payload = payload;
    second_payload["model"]["tip_force"] = json!(2000);
    run_cli(&document("solve_bar_1d", second_payload), &proxy)?;
    let captured = proxy.capture();
    assert!(captured.errors.is_empty(), "{captured:?}");
    assert_eq!(captured.requests.len(), 2);
    assert_eq!(captured.discarded.len(), 2);
    assert_eq!(captured.replies.len(), 2);
    for (original, corrupted) in captured.discarded.iter().zip(&captured.replies) {
        assert!(original["job"]["job_id"].is_string());
        assert!(corrupted["job"].get("job_id").is_none());
        let mut expected = original.clone();
        expected["job"].as_object_mut().unwrap().remove("job_id");
        assert_eq!(
            expected, *corrupted,
            "fault changed more than the job identity"
        );
    }
    let second_id = captured.discarded[1]["job"]["job_id"].as_str().unwrap();
    assert_ne!(first_id, second_id);
    direct
        .execute_step(
            "job_wait",
            1,
            &json!({"job_id":second_id,"timeout_ms":5000,"interval_ms":10}),
        )
        .map_err(|error| error.message)?;
    for (id, force) in [(first_id.as_str(), 1000.0), (second_id, 2000.0)] {
        let result = direct
            .execute_step("result_fetch", 1, &json!({"job_id":id}))
            .map_err(|error| error.message)?;
        let displacement = result.result["result"]["tip_displacement"]
            .as_f64()
            .unwrap();
        assert!((displacement / (force / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 2);
    let jobs: Value = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs["jobs"].as_array().unwrap().len(), 2);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1["projects"],
        json!([])
    );
    Ok(())
}
