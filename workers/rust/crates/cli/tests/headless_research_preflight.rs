use kyuubiki_headless_sdk::{
    HeadlessWorkflowDocument, build_headless_research_round_evidence, normalize_workflow_document,
    run_batch_dry,
};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct HealthSpy {
    url: String,
    calls: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HealthSpy {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_calls = calls.clone();
        let thread_stop = stop.clone();
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while !thread_stop.load(Ordering::SeqCst) && Instant::now() < deadline {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("health spy accept: {error}"),
                };
                thread_calls.fetch_add(1, Ordering::SeqCst);
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 4096];
                let count = stream.read(&mut request).unwrap();
                assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /api/health "));
                let body = r#"{"status":"ok","result":{"research_metric":8.0}}"#;
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        Self {
            url,
            calls,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for HealthSpy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let result = thread.join();
            if !std::thread::panicking() {
                result.expect("health spy thread");
            }
        }
    }
}

struct Fixture {
    root: PathBuf,
    spy: HealthSpy,
    source: Value,
}

impl Fixture {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-research-preflight-{}-{timestamp}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let source = json!({
            "schema_version":"kyuubiki.headless-workflow/v1","language":"en",
            "exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"research.preflight","steps":[
                {"action":"service_health","payload":{"research_input":1.0}}
            ]}
        });
        let fixture = Self {
            root,
            spy: HealthSpy::start(),
            source,
        };
        fixture.write("workflow.json", &fixture.source);
        fixture.write("evidence.json", &json!({"retained_previous_output":true}));
        fixture
    }

    fn write(&self, name: &str, value: &Value) {
        fs::write(
            self.root.join(name),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }

    fn spec(&self, iteration: u64) -> Value {
        json!({
            "schema_version":"kyuubiki.headless-research-round-spec/v1",
            "round_id":format!("preflight-round-{iteration}"),"workflow_id":"research.preflight",
            "iteration":iteration,"primary_metric_ids":["research_metric"],
            "metrics":[{"metric_id":"research_metric",
                "pointer":"/steps/0/result_preview/result/research_metric","unit":"1","objective":"observe"}]
        })
    }

    fn patch(&self) -> Value {
        json!({
            "schema_version":"kyuubiki.headless-parameter-patch/v1","patch_id":"preflight-patch",
            "workflow_id":"research.preflight","changes":[{
                "path":"/steps/0/payload/research_input","expected":1.0,"value":2.0
            }]
        })
    }

    fn previous(&self) -> Value {
        // Synthetic evidence exercises static contract checks, not scientific provenance.
        let document: HeadlessWorkflowDocument =
            serde_json::from_value(self.source.clone()).unwrap();
        let batch = normalize_workflow_document(&document).unwrap();
        let mut report = run_batch_dry(&batch, false, false);
        report.mode = "execute:service".into();
        report.steps[0].status = "executed".into();
        report.steps[0].result_preview = json!({"result":{"research_metric":10.0}});
        serde_json::to_value(
            build_headless_research_round_evidence(
                &batch,
                &report,
                &serde_json::from_value(self.spec(1)).unwrap(),
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn run(&self, later: bool, patch: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"));
        command.current_dir(&self.root).args([
            "run",
            "workflow.json",
            "--json",
            "--execute",
            "--executor",
            "service",
            "--execution-posture",
            "research",
            "--api-base-url",
            &self.spy.url,
            "--research-round-spec",
            "spec.json",
            "--research-round-out",
            "evidence.json",
            "--report-out",
            "run.json",
        ]);
        if later {
            command.args(["--previous-round-evidence", "previous.json"]);
        }
        if patch {
            command.args(["--parameter-patch", "patch.json"]);
        }
        command.output().unwrap()
    }

    fn assert_preflight_failure(&self, output: Output, message: &str) {
        assert!(!output.status.success(), "unexpected successful run");
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(report["status"], "invalid", "{report}");
        assert_eq!(report["executed_step_count"], 0);
        assert_eq!(report["steps"], json!([]));
        assert_eq!(report["execution_summary"]["job_count"], 0);
        assert_eq!(
            report["execution_summary"]["failure"]["error_code"],
            "kyuubiki.headless.research_round_validation"
        );
        assert_eq!(
            report["execution_summary"]["failure"]["stage"],
            "research_round"
        );
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
            serde_json::from_slice::<Value>(&fs::read(self.root.join("run.json")).unwrap())
                .unwrap(),
            report
        );
        assert_eq!(
            self.spy.calls.load(Ordering::SeqCst),
            0,
            "preflight reached service"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(self.root.join("workflow.json")).unwrap())
                .unwrap(),
            self.source
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(self.root.join("evidence.json")).unwrap())
                .unwrap(),
            json!({"retained_previous_output":true})
        );
        assert!(fs::read_dir(&self.root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp")
        }));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn nonexistent_metric_step_is_rejected_before_any_service_request_even_with_timeout_text() {
    let fixture = Fixture::new();
    for (pointer, message) in [
        (
            "/steps/1/result_preview/result/timed out waiting for job",
            "missing batch step 1",
        ),
        (
            "/steps/0/result_preview/result/timed out waiting for job~2",
            "may only read",
        ),
        (
            "/steps/01/result_preview/result/research_metric",
            "may only read",
        ),
        (
            "/steps/184467440737095516160/result_preview/result/research_metric",
            "may only read",
        ),
    ] {
        let mut spec = fixture.spec(1);
        spec["metrics"][0]["pointer"] = json!(pointer);
        fixture.write("spec.json", &spec);
        fixture.assert_preflight_failure(fixture.run(false, false), message);
    }
}

#[test]
fn first_round_parameter_patch_is_rejected_before_service_execution() {
    let fixture = Fixture::new();
    fixture.write("spec.json", &fixture.spec(1));
    fixture.write("patch.json", &fixture.patch());
    fixture.assert_preflight_failure(fixture.run(false, true), "effective baseline");
}

#[test]
fn invalid_previous_evidence_or_disconnected_lineage_never_reaches_service() {
    let fixture = Fixture::new();
    fixture.write("patch.json", &fixture.patch());
    for (field, value, message) in [
        (
            "qualified",
            json!(false),
            "previous evidence is not qualified",
        ),
        (
            "run_mode",
            json!("dry_run"),
            "previous evidence is not qualified",
        ),
        (
            "workflow_id",
            json!("research.other"),
            "cross workflow boundaries",
        ),
        (
            "batch_content_sha256",
            json!("a".repeat(64)),
            "does not start from the previous batch",
        ),
        ("iteration", json!(2), "incomplete lineage"),
        (
            "round_id",
            json!("preflight-round-2"),
            "round_id must change",
        ),
    ] {
        let mut previous = fixture.previous();
        previous[field] = value;
        fixture.write("previous.json", &previous);
        fixture.write("spec.json", &fixture.spec(2));
        let original_previous = fs::read(fixture.root.join("previous.json")).unwrap();
        fixture.assert_preflight_failure(fixture.run(true, true), message);
        assert_eq!(
            fs::read(fixture.root.join("previous.json")).unwrap(),
            original_previous
        );
    }
    fixture.write("previous.json", &fixture.previous());
    fixture.write("spec.json", &fixture.spec(3));
    fixture.assert_preflight_failure(fixture.run(true, true), "iteration is not contiguous");
}

#[test]
fn valid_first_and_patched_contiguous_rounds_still_execute_once_each() {
    let fixture = Fixture::new();
    fixture.write("spec.json", &fixture.spec(1));
    let first = fixture.run(false, false);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_evidence: Value =
        serde_json::from_slice(&fs::read(fixture.root.join("evidence.json")).unwrap()).unwrap();
    fixture.write("previous.json", &first_evidence);
    fixture.write("spec.json", &fixture.spec(2));
    fixture.write("patch.json", &fixture.patch());
    let second = fixture.run(true, true);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(fixture.spy.calls.load(Ordering::SeqCst), 2);
    let evidence: Value =
        serde_json::from_slice(&fs::read(fixture.root.join("evidence.json")).unwrap()).unwrap();
    assert_eq!(evidence["iteration"], 2);
    assert_eq!(evidence["metrics"][0]["value"], 8.0);
    assert_eq!(evidence["previous_round"]["round_id"], "preflight-round-1");
    assert_eq!(
        evidence["patch_receipt"]["before_sha256"],
        first_evidence["batch_content_sha256"]
    );
    assert_eq!(
        evidence["patch_receipt"]["after_sha256"],
        evidence["batch_content_sha256"]
    );
}
