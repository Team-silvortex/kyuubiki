use kyuubiki_headless_sdk::{build_template_document, material_study_descriptors};
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

struct Fixture {
    root: PathBuf,
    url: String,
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    server: Option<JoinHandle<()>>,
}

impl Fixture {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-material-preflight-{}-{timestamp}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let calls = requests.clone();
        let stopping = stop.clone();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("material spy: {error}"),
                };
                calls.fetch_add(1, Ordering::SeqCst);
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 4096];
                let _ = stream.read(&mut request);
                let body = r#"{"error":"preflight must not issue any request"}"#;
                let _ = write!(
                    stream,
                    "HTTP/1.1 500 Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Self {
            root,
            url,
            requests,
            stop,
            server: Some(server),
        }
    }

    fn write(&self, value: &Value) {
        fs::write(
            self.root.join("workflow.json"),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
        fs::write(
            self.root.join("material.json"),
            b"{\"retained_material\":true}",
        )
        .unwrap();
    }

    fn run(&self, study: &str, executor: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
            .current_dir(&self.root)
            .args([
                "run",
                "workflow.json",
                "--json",
                "--execute",
                "--executor",
                executor,
                "--api-base-url",
                &self.url,
                "--material-report",
                study,
                "--material-report-out",
                "material.json",
                "--report-out",
                "run.json",
            ])
            .output()
            .unwrap()
    }

    fn assert_rejected(&self, output: Output, before: &[u8], empty_document: bool) {
        assert!(!output.status.success());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(report["status"], "invalid", "{report}");
        assert_eq!(report["executed_step_count"], 0);
        assert_eq!(report["steps"], json!([]));
        assert_eq!(report["execution_summary"]["job_count"], 0);
        let (code, stage) = if empty_document {
            ("headless_command_failed", "command_validation")
        } else {
            (
                "material_report_input_contract_mismatch",
                "material_report_validation",
            )
        };
        assert_eq!(
            report["execution_summary"]["failure"]["error_code"],
            format!("kyuubiki.headless.{code}")
        );
        assert_eq!(report["execution_summary"]["failure"]["retryable"], false);
        assert_eq!(diagnostic["error"]["code"], code);
        assert_eq!(diagnostic["error"]["stage"], stage);
        if empty_document {
            assert_eq!(
                diagnostic["error"]["message"],
                "Headless workflow document does not contain a valid workflow draft."
            );
        }
        assert_eq!(diagnostic["error"]["retryable"], false);
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(self.root.join("run.json")).unwrap())
                .unwrap(),
            report
        );
        assert_eq!(fs::read(self.root.join("workflow.json")).unwrap(), before);
        assert_eq!(
            fs::read(self.root.join("material.json")).unwrap(),
            b"{\"retained_material\":true}"
        );
        assert_eq!(
            self.requests.load(Ordering::SeqCst),
            0,
            "material preflight connected to service"
        );
        assert_eq!(fs::read_dir(&self.root).unwrap().count(), 3);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(server) = self.server.take() {
            server.join().unwrap();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn incomplete_builtin_material_plans_stop_with_zero_requests_and_retained_outputs() {
    for study in material_study_descriptors() {
        let fixture = Fixture::new();
        let original =
            serde_json::to_value(build_template_document(study.template_id, None).unwrap())
                .unwrap();
        for retained in [0, 2, 3, 6, 8] {
            let mut value = original.clone();
            value["workflow"]["steps"]
                .as_array_mut()
                .unwrap()
                .truncate(retained);
            fixture.write(&value);
            let before = fs::read(fixture.root.join("workflow.json")).unwrap();
            fixture.assert_rejected(
                fixture.run(study.aliases[0], "service"),
                &before,
                retained == 0,
            );
        }
        let mut extra = original.clone();
        let duplicate = extra["workflow"]["steps"][8].clone();
        extra["workflow"]["steps"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        fixture.write(&extra);
        let before = fs::read(fixture.root.join("workflow.json")).unwrap();
        fixture.assert_rejected(fixture.run(study.id, "service"), &before, false);
    }
}

#[test]
fn wrong_candidate_sources_and_repeated_readbacks_stop_before_service_execution() {
    for study in material_study_descriptors() {
        let fixture = Fixture::new();
        let original =
            serde_json::to_value(build_template_document(study.template_id, None).unwrap())
                .unwrap();
        for edit in ["candidate", "readback", "wait"] {
            let mut value = original.clone();
            match edit {
                "candidate" => {
                    value["workflow"]["steps"][0]["payload"]["research"]["candidate_id"] =
                        json!("timed out waiting for job")
                }
                "readback" => {
                    value["workflow"]["steps"][5]["payload"]["job_id"] =
                        json!("{{steps.1.result.job_id}}")
                }
                _ => value["workflow"]["steps"][1]["payload"]["job_id"] = json!("unrelated-job"),
            }
            fixture.write(&value);
            let before = fs::read(fixture.root.join("workflow.json")).unwrap();
            fixture.assert_rejected(fixture.run(study.id, "service"), &before, false);
        }
    }
}

#[test]
fn intact_builtin_material_templates_still_run_as_explicit_mock_previews() {
    for study in material_study_descriptors() {
        let fixture = Fixture::new();
        let value = serde_json::to_value(build_template_document(study.template_id, None).unwrap())
            .unwrap();
        fixture.write(&value);
        let before = fs::read(fixture.root.join("workflow.json")).unwrap();
        let output = fixture.run(study.id, "mock");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"], "ok");
        assert_eq!(report["executed_step_count"], 9);
        assert_eq!(
            fs::read(fixture.root.join("workflow.json")).unwrap(),
            before
        );
        assert_eq!(fixture.requests.load(Ordering::SeqCst), 0);
        let material: Value =
            serde_json::from_slice(&fs::read(fixture.root.join("material.json")).unwrap()).unwrap();
        assert_eq!(material["schema_version"], study.schema_version);
        assert_eq!(material["candidates"].as_array().unwrap().len(), 3);
    }
}

#[test]
fn changed_physical_profiles_stop_before_requests_and_keep_prior_material_outputs() {
    for study in material_study_descriptors() {
        let fixture = Fixture::new();
        let original =
            serde_json::to_value(build_template_document(study.template_id, None).unwrap())
                .unwrap();
        let key = if study.id == "material_composite_thermo_electric_panel" {
            "heat_model"
        } else {
            "model"
        };
        for edit in ["model", "thickness", "geometry", "metadata", "unknown"] {
            let mut value = original.clone();
            let steps = value["workflow"]["steps"].as_array_mut().unwrap();
            match edit {
                "model" => steps[0]["payload"][key] = steps[3]["payload"][key].clone(),
                "thickness" => steps[0]["payload"][key]["elements"][0]["thickness"] = json!(0.004),
                "geometry" => {
                    let x = &mut steps[0]["payload"][key]["nodes"][1]["x"];
                    *x = json!(x.as_f64().unwrap() * 1.1);
                }
                "metadata" => steps[0]["payload"]["research"]["density_kg_m3"] = json!(9999.0),
                _ => steps[0]["payload"]["timed out waiting for job"] = json!({"override":true}),
            }
            if edit == "model" && study.id == "material_dielectric_screening" {
                steps[0]["payload"]["research"]["relative_permittivity"] =
                    steps[3]["payload"]["research"]["relative_permittivity"].clone();
            }
            fixture.write(&value);
            let before = fs::read(fixture.root.join("workflow.json")).unwrap();
            let output = fixture.run(study.id, "service");
            let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert!(
                diagnostic["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("fixed report input profile"),
                "{diagnostic}"
            );
            assert!(
                diagnostic["error"]["recommended_action"]
                    .as_str()
                    .unwrap()
                    .contains("custom research"),
                "{diagnostic}"
            );
            fixture.assert_rejected(output, &before, false);
        }
    }
}

#[test]
fn coordinated_dielectric_si_edits_cannot_silently_use_original_report_constants() {
    let fixture = Fixture::new();
    let mut value = serde_json::to_value(
        build_template_document("material_dielectric_screening", None).unwrap(),
    )
    .unwrap();
    value["workflow"]["steps"][0]["payload"]["research"]["relative_permittivity"] = json!(4.7);
    value["workflow"]["steps"][0]["payload"]["model"]["elements"][0]["permittivity"] =
        json!(8.854_187_812_8e-12 * 4.7);
    fixture.write(&value);
    let before = fs::read(fixture.root.join("workflow.json")).unwrap();
    fixture.assert_rejected(
        fixture.run("dielectric-screening", "service"),
        &before,
        false,
    );
}
