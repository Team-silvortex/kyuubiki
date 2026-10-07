use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{assert_bar, bar_task, batch, http_json};
use super::{LiveServer, Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{ServiceHeadlessExecutor, execute_batch_with_executor};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct LostReceipt {
    server: Option<LiveServer>,
    relay: AckLossProxy,
    owner: LiveAgent,
    peer: LiveAgent,
    data: Scratch,
    task: Value,
    attempt: String,
    original: Value,
    projects: Value,
    journal: Vec<u8>,
}

impl LostReceipt {
    fn start() -> Result<Self> {
        let data = Scratch::new()?;
        let owner = LiveAgent::start()?;
        let peer = LiveAgent::start()?;
        let relay = AckLossProxy::agent(owner.port(), true)?;
        let server =
            start_live_server_with_state(Some(relay.port), Some(peer.port()), Some(&data.0))?;
        let mut task = bar_task("unavailable-original-receipt", 0.01);
        task["runtime_hints"]["placement_tags"] = json!(["owned-live-owner"]);
        task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task)?);
        let projects = http_json(server.port, "/api/v1/projects", None)?.1;
        let client = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
        let observed = Self::execute(&client, &task);
        assert_eq!(observed.status, "failed");
        assert_eq!(observed.executed_step_count, 0);
        assert_eq!(
            observed.steps.len(),
            1,
            "downstream write attempted after lost transport"
        );
        wait_for_lifecycle(&owner, "accepting", 0)?;
        let dispatch = client
            .inspect_operator_task_dispatch(
                task["task_id"].as_str().unwrap(),
                task["integrity"]["task_digest"].as_str().unwrap(),
            )
            .map_err(|error| error.message)?;
        assert_eq!(dispatch["retained_attempt_count"], 1);
        assert_eq!(dispatch["attempts"][0]["state"], "outcome_unknown");
        let attempt = dispatch["attempts"][0]["attempt_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let original = client
            .fetch_operator_task_result(&task, &attempt)
            .map_err(|e| e.message)?;
        assert_eq!(original["status"], "receipt_recovered");
        assert_eq!(
            original["request_id"],
            dispatch["attempts"][0]["request_id"]
        );
        assert_bar(&original["completion"]);
        let journal = fs::read(
            data.0
                .join("operator-task-dispatches")
                .join(format!("{attempt}.json")),
        )?;
        Ok(Self {
            server: Some(server),
            relay,
            owner,
            peer,
            data,
            task,
            attempt,
            original,
            projects,
            journal,
        })
    }

    fn port(&self) -> u16 {
        self.server.as_ref().unwrap().port
    }

    fn client(&self) -> ServiceHeadlessExecutor {
        ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", self.port()))
    }

    fn execute(
        client: &ServiceHeadlessExecutor,
        task: &Value,
    ) -> kyuubiki_headless_sdk::HeadlessRunReport {
        let mut client = client.clone();
        execute_batch_with_executor(
            &batch(task.clone(), Some("forbidden-after-missing-receipt")),
            &mut client,
            false,
            false,
        )
    }

    fn fetch(&self, attempt: &str) -> Result<Value> {
        self.client()
            .fetch_operator_task_result(&self.task, attempt)
            .map_err(|e| e.message.into())
    }

    fn assert_unknown(&self, status: &str) -> Result<()> {
        for _ in 0..2 {
            let missing = self.fetch(&self.attempt)?;
            assert_eq!(missing["status"], status, "{missing}");
            assert_eq!(missing["outcome"], "unknown");
            assert!(missing["completion"].is_null());
            assert_eq!(missing["automatic_replay_authorized"], false);
            assert_eq!(missing["publication_performed"], false);
            assert_eq!(missing["task_id"], self.task["task_id"]);
            assert_eq!(
                missing["task_digest"],
                self.task["integrity"]["task_digest"]
            );
            assert_eq!(missing["attempt_id"], self.attempt);
        }
        let dispatch = self
            .client()
            .inspect_operator_task_dispatch(
                self.task["task_id"].as_str().unwrap(),
                self.task["integrity"]["task_digest"].as_str().unwrap(),
            )
            .map_err(|e| e.message)?;
        assert_eq!(
            dispatch["retained_attempt_count"], 1,
            "recovery created an attempt"
        );
        assert_eq!(dispatch["attempts"][0]["state"], "outcome_unknown");
        self.assert_unchanged()
    }

    fn assert_unchanged(&self) -> Result<()> {
        let path = self
            .data
            .0
            .join("operator-task-dispatches")
            .join(format!("{}.json", self.attempt));
        assert_eq!(
            fs::read(path)?,
            self.journal,
            "recovery mutated original journal"
        );
        assert_eq!(
            http_json(self.port(), "/api/v1/projects", None)?.1,
            self.projects
        );
        assert_eq!(count(&self.peer)?, 0, "peer got fallback execution");
        Ok(())
    }

    fn rerun(&self) -> Result<Value> {
        let report = Self::execute(&self.client(), &self.task);
        assert_eq!(report.status, "failed"); // Relay also discards the explicitly requested new reply.
        assert_eq!(report.executed_step_count, 0);
        assert_eq!(report.steps.len(), 1);
        wait_for_lifecycle(&self.owner, "accepting", 0)?;
        let dispatch = self
            .client()
            .inspect_operator_task_dispatch(
                self.task["task_id"].as_str().unwrap(),
                self.task["integrity"]["task_digest"].as_str().unwrap(),
            )
            .map_err(|e| e.message)?;
        assert_eq!(dispatch["retained_attempt_count"], 2);
        let next = dispatch["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["attempt_id"] != self.attempt)
            .unwrap();
        let recovered = self.fetch(next["attempt_id"].as_str().unwrap())?;
        assert_eq!(recovered["status"], "receipt_recovered");
        assert_bar(&recovered["completion"]);
        self.assert_unchanged()?;
        Ok(recovered)
    }

    fn assert_run_requests(&self, expected: usize) {
        let capture = self.relay.capture();
        assert!(capture.errors.is_empty(), "{capture:?}");
        assert_eq!(
            capture
                .requests
                .iter()
                .filter(|r| r["method"] == "run_operator_task_ir")
                .count(),
            expected
        );
        assert!(!capture.requests.iter().any(|r| matches!(
            r["method"].as_str(),
            Some("cancel_job" | "cancel_execution")
        )));
    }
}

fn count(agent: &LiveAgent) -> Result<u64> {
    Ok(agent.request("retention-execution-count", "describe_agent", json!({}))?
        ["result"]["watchdog"]["total_started_execution_count"].as_u64().unwrap())
}

#[test]
fn real_count_eviction_keeps_original_unknown_without_touching_peer_or_downstream() -> Result<()> {
    let fixture = LostReceipt::start()?;
    let state = fixture
        .owner
        .request("retention-policy", "describe_agent", json!({}))?;
    let policy = &state["result"]["task_result_retention"];
    assert_eq!(policy["max_entries"], 64, "{state}");
    // Explicit fresh admissions exercise the actual cache cap, not a fake clock or policy override.
    for n in 1..=64 {
        let reply = fixture.owner.request(&format!("explicit-capacity-{n}"), "run_operator_task_ir",
            json!({"mode":"execute", "task_ir":fixture.task, "dispatch_attempt_id":format!("{n:032x}")}))?;
        assert_eq!(reply["ok"], true, "{reply}");
        assert_bar(&json!({"status":"executed", "result":reply["result"]}));
        if n == 63 {
            assert_eq!(
                fixture.fetch(&fixture.attempt)?,
                fixture.original,
                "read must not promote reservation order"
            );
        }
    }
    wait_for_lifecycle(&fixture.owner, "accepting", 0)?;
    fixture.assert_unknown("not_retained")?;
    assert_eq!(
        count(&fixture.owner)?,
        65,
        "read recovery reran computation"
    );
    fixture.assert_run_requests(1);
    let new = fixture.rerun()?;
    assert!(new["generation"].as_u64().unwrap() > fixture.original["generation"].as_u64().unwrap());
    assert_eq!(count(&fixture.owner)?, 66);
    fixture.assert_run_requests(2);
    assert_eq!(fixture.fetch(&fixture.attempt)?["status"], "not_retained");
    Ok(())
}

#[test]
fn real_endpoint_removal_and_agent_restart_never_substitute_the_original_receipt() -> Result<()> {
    let mut fixture = LostReceipt::start()?;
    fixture.owner.stop_process()?;
    // The relay is still reachable, but the original Agent behind it is not.
    fixture.assert_unknown("original_endpoint_unreachable")?;
    assert_eq!(count(&fixture.peer)?, 0);

    drop(fixture.server.take());
    fixture.server = Some(start_live_server_with_state(
        Some(fixture.peer.port()),
        None,
        Some(&fixture.data.0),
    )?);
    let before = fixture.relay.capture().requests.len();
    fixture.assert_unknown("original_endpoint_not_configured")?;
    assert_eq!(
        fixture.relay.capture().requests.len(),
        before,
        "unconfigured owner was probed"
    );

    fixture.owner.start_process()?;
    drop(fixture.server.take());
    fixture.server = Some(start_live_server_with_state(
        Some(fixture.relay.port),
        Some(fixture.peer.port()),
        Some(&fixture.data.0),
    )?);
    fixture.assert_unknown("not_retained")?;
    assert_eq!(count(&fixture.owner)?, 0, "Agent restart caused replay");
    let recovered = fixture.rerun()?;
    assert_ne!(
        recovered["process_instance_id"],
        fixture.original["process_instance_id"]
    );
    assert_ne!(recovered["request_id"], fixture.original["request_id"]);
    assert_eq!(count(&fixture.owner)?, 1);
    assert_eq!(fixture.fetch(&fixture.attempt)?["status"], "not_retained");
    // Transport failures are expected only while the deliberately stopped owner is absent.
    assert_eq!(
        fixture
            .relay
            .capture()
            .requests
            .iter()
            .filter(|r| r["method"] == "run_operator_task_ir")
            .count(),
        2
    );
    fixture.assert_unchanged()?;
    Ok(())
}
