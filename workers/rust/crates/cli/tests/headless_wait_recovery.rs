use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct WaitFixture {
    url: String,
    directory: PathBuf,
    requests: Arc<Mutex<Vec<String>>>,
    stop: mpsc::Sender<()>,
    server: Option<thread::JoinHandle<()>>,
}

impl WaitFixture {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&requests);
        let (stop, stopped) = mpsc::channel();
        let server = thread::spawn(move || {
            while stopped.try_recv().is_err() {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(error) => panic!("accept status request: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 4096];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                }
                let status = {
                    let mut requests = observed.lock().unwrap();
                    requests.push(String::from_utf8(request).unwrap());
                    if requests.len() == 1 {
                        "running"
                    } else {
                        "completed"
                    }
                };
                let body = json!({"job": {"job_id": "job-retained", "status": status}}).to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "kyuubiki-wait-recovery-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        Self {
            url,
            directory,
            requests,
            stop,
            server: Some(server),
        }
    }

    fn run(&self, timeout_ms: u64) -> Output {
        let workflow = json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "2026-09-20T00:00:00Z", "language": "en",
            "workflow": {"id": "workflow.observe-existing-job", "steps": [
                {"action": "job_wait", "payload": {
                    "job_id": "job-retained", "interval_ms": timeout_ms, "timeout_ms": timeout_ms
                }}
            ]}
        });
        let path = self.directory.join("workflow.json");
        let report = self.directory.join("report.json");
        fs::write(&path, serde_json::to_vec(&workflow).unwrap()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
            .arg("run")
            .arg(path)
            .args([
                "--json",
                "--execute",
                "--executor",
                "service",
                "--execution-posture",
                "research",
                "--api-base-url",
            ])
            .arg(&self.url)
            .arg("--report-out")
            .arg(report)
            .output()
            .expect("run native Headless CLI")
    }
}

impl Drop for WaitFixture {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        self.server.take().unwrap().join().unwrap();
        let _ = fs::remove_file(self.directory.join("workflow.json"));
        let _ = fs::remove_file(self.directory.join("report.json"));
        let _ = fs::remove_dir(&self.directory);
    }
}

#[test]
fn native_cli_persists_wait_failure_and_resumes_without_resubmitting() {
    let fixture = WaitFixture::new();
    let output = fixture.run(100);
    assert!(
        !output.status.success(),
        "late extra poll must not turn a timeout into success"
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "failed");
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(failure["error_code"], "kyuubiki.headless.job_wait_timeout");
    assert_eq!(failure["stage"], "job_wait");
    assert_eq!(failure["retryable"], true);
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains("job-retained"));
    assert!(message.contains("timeout_reason=client_total_budget_exhausted"));
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    let persisted: Value =
        serde_json::from_slice(&fs::read(fixture.directory.join("report.json")).unwrap()).unwrap();
    assert_eq!(persisted, report);
    assert_eq!(fixture.requests.lock().unwrap().len(), 1);

    let output = fixture.run(1_000);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert_eq!(
        report["execution_summary"]["jobs"][0]["job_id"],
        "job-retained"
    );
    assert_eq!(
        report["execution_summary"]["jobs"][0]["status"],
        "completed"
    );
    let requests = fixture.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| request.starts_with("GET /api/v1/jobs/job-retained/status HTTP/1.1\r\n")));
}
