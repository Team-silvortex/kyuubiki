//! Manual two-phase qualification. The operator owns the remote fixture's managed restart.
#[path = "support/operator_cross_host_transport.rs"]
mod transport;
use kyuubiki_headless_sdk::deploy_operator_package;
use kyuubiki_installer::{project_operator_in, stage_store_operator_variant_into};
use kyuubiki_protocol::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    net::SocketAddr,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn loopback_endpoint(value: &str) -> Result<SocketAddr, String> {
    transport::endpoint(value, "ssh-loopback")
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-cross-host-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn qualification_only_accepts_explicit_loopback_tunnels() {
    for invalid in [
        "192.0.2.1:5001",
        "0.0.0.0:5001",
        "localhost:5001",
        "127.0.0.1:0",
        "[::]:5001",
    ] {
        assert!(loopback_endpoint(invalid).is_err());
    }
    assert!(loopback_endpoint("127.0.0.1:5001").is_ok());
    assert!(loopback_endpoint("[::1]:5001").is_ok());
}

#[test]
#[ignore = "requires owned Installer-managed Linux Agent, explicit SSH loopback or native TLS, and trusted template .so"]
fn staged_linux_variant_crosses_hosts_without_store_or_engine_coupling() {
    let phase = std::env::var("KYUUBIKI_CROSS_HOST_PHASE")
        .expect("deploy or verify after explicit managed restart");
    assert!(matches!(phase.as_str(), "deploy" | "verify"));
    let client = transport::Transport::from_env().unwrap();
    let tls = client.security.is_some();
    let state = tls.then(|| {
        PathBuf::from(
            std::env::var("KYUUBIKI_CROSS_HOST_STATE_FILE").expect("owned phase receipt path"),
        )
    });
    let descriptor = client.request("describe_agent", json!({})).unwrap();
    if tls {
        assert!(
            !client.endpoint.ip().is_loopback(),
            "cross-host TLS proof requires a real network endpoint"
        );
        let access = &descriptor["connection_access"];
        assert_eq!(access["authenticated"], true);
        assert_eq!(access["encrypted"], true);
        assert_eq!(access["peer_scope"], "non_loopback");
        assert_eq!(access["control_mode"], "tls_api_token");
        let pem = transport::read_material(
            &PathBuf::from(std::env::var("KYUUBIKI_CROSS_HOST_CA_FILE").unwrap()),
            false,
        )
        .unwrap();
        let token = transport::read_material(
            &PathBuf::from(std::env::var("KYUUBIKI_CROSS_HOST_TOKEN_FILE").unwrap()),
            true,
        )
        .unwrap();
        let server_name = std::env::var("KYUUBIKI_CROSS_HOST_SERVER_NAME").unwrap();
        client.assert_denials(&pem, &server_name, &token);
        if phase == "verify" {
            let old_token = transport::read_material(
                &PathBuf::from(
                    std::env::var("KYUUBIKI_CROSS_HOST_OLD_TOKEN_FILE")
                        .expect("previous test token"),
                ),
                true,
            )
            .unwrap();
            let old = transport::Transport::configured(
                &client.endpoint.to_string(),
                "native-tls",
                Some(&pem),
                Some(&server_name),
                Some(old_token),
            )
            .unwrap();
            assert_eq!(
                old.request("describe_agent", json!({})).unwrap_err(),
                "cross_host_authentication_failed",
                "rotated token must be rejected during authentication"
            );
        }
    }
    let platform =
        std::env::var("KYUUBIKI_CROSS_HOST_PLATFORM").unwrap_or_else(|_| "linux-x86_64".into());
    assert!(matches!(
        platform.as_str(),
        "linux-x86_64" | "linux-aarch64"
    ));
    let library = PathBuf::from(
        std::env::var("KYUUBIKI_CROSS_HOST_LIBRARY").expect("trusted template library"),
    );
    assert!(fs::metadata(&library).unwrap().len() <= OPERATOR_DEPLOYMENT_MAX_BYTES);
    let fixture = Fixture::new();
    let files = fixture.0.join("files");
    fs::create_dir_all(files.join(&platform)).unwrap();
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates/operator-crate-template/kyuubiki-operator.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
    manifest["entrypoint"] = json!("operator.so");
    let mut entries = Vec::new();
    for (name, bytes) in [
        (
            "kyuubiki-operator.json",
            serde_json::to_vec(&manifest).unwrap(),
        ),
        ("operator.so", fs::read(library).unwrap()),
    ] {
        let path = format!("{platform}/{name}");
        fs::write(files.join(&path), &bytes).unwrap();
        entries.push(StoreInstallFile {
            path,
            size_bytes: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        });
    }
    let handoff = StoreOperatorInstallRequest {
        schema_version: "kyuubiki.store-operator-install/v1".into(),
        package_id: manifest["package_id"].as_str().unwrap().into(),
        package_version: manifest["package_version"].as_str().unwrap().into(),
        platform: platform.clone(),
        files: entries,
    };
    let store = fixture.0.join("operator-store");
    let staged = stage_store_operator_variant_into(&files, &store, &handoff).unwrap();
    assert!(
        !store.join("packages").exists(),
        "foreign code must not enter the controller's local runtime"
    );
    let pin = &staged.package;
    let project = |operation| {
        project_operator_in(
            &store,
            ProjectOperatorRequest {
                platform: Some(platform.clone()),
                project_id: "cross-host-research".into(),
                scope: "a".repeat(64),
                operation,
            },
        )
    };
    project(ProjectOperatorOperation::Enable {
        package: pin.clone(),
    })
    .unwrap();
    let mut lose_commit_reply = tls && phase == "deploy";
    let mut commits = 0;
    let mut exchange = |request: OperatorDeploymentRequest| {
        let commit = matches!(request.operation, OperatorDeploymentAction::Commit);
        let reply = client.request(
            "manage_operator_package",
            serde_json::to_value(request).unwrap(),
        )?;
        if commit {
            commits += 1;
            if lose_commit_reply {
                lose_commit_reply = false;
                return Err("test_commit_response_lost".into());
            }
        }
        serde_json::from_value::<OperatorDeploymentStatus>(reply).map_err(|e| e.to_string())
    };
    let inspected = exchange(OperatorDeploymentRequest {
        package: pin.clone(),
        expected_process_instance_id: None,
        operation: OperatorDeploymentAction::Inspect,
    })
    .unwrap();
    assert_eq!(inspected.platform, platform);
    if phase == "deploy" {
        assert_eq!(
            inspected.state, "missing",
            "use an empty owned test Agent, never overwrite an existing deployment"
        );
        let mut reads = 0;
        let result = deploy_operator_package(
            pin,
            &platform,
            &mut exchange,
            |offset| {
                reads += 1;
                project(ProjectOperatorOperation::ReadDeployment {
                    package: pin.clone(),
                    offset,
                })?
                .deployment
                .ok_or("missing_chunk".into())
            },
            || true,
            |_| {},
        );
        if tls {
            assert_eq!(result.unwrap_err(), "test_commit_response_lost");
        } else {
            assert_eq!(result.unwrap().state, "installed_restart_required");
        }
        assert!(reads > 0);
        let repeated = deploy_operator_package(
            pin,
            &platform,
            &mut exchange,
            |_| panic!("installed bytes must not be replayed"),
            || true,
            |_| {},
        )
        .unwrap();
        assert_eq!(repeated.state, "installed_restart_required");
        assert_eq!(commits, 1);
        if let Some(path) = &state {
            let mut receipt = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            receipt
                .write_all(
                    &serde_json::to_vec(
                        &json!({"package":pin,"process_instance_id":inspected.process_instance_id}),
                    )
                    .unwrap(),
                )
                .unwrap();
            receipt.sync_all().unwrap();
        }
    } else {
        assert_eq!(
            inspected.state, "ready",
            "the operator must explicitly restart the managed fixture before verification"
        );
        if let Some(path) = &state {
            let previous: Value =
                serde_json::from_str(&transport::read_material(path, false).unwrap()).unwrap();
            assert_eq!(previous["package"], serde_json::to_value(pin).unwrap());
            assert_ne!(
                previous["process_instance_id"].as_str(),
                inspected.process_instance_id.as_deref()
            );
            let rejected = client
                .request(
                    "manage_operator_package",
                    serde_json::to_value(OperatorDeploymentRequest {
                        package: pin.clone(),
                        expected_process_instance_id: Some(
                            previous["process_instance_id"].as_str().unwrap().into(),
                        ),
                        operation: OperatorDeploymentAction::Commit,
                    })
                    .unwrap(),
                )
                .unwrap_err();
            assert!(rejected.contains("deployment_agent_changed"));
        }
        let prepared = project(ProjectOperatorOperation::Prepare {
            package: pin.clone(),
            operator_id: "extract.template_summary".into(),
            task_id: "cross-host-template-probe".into(),
            input: json!({"values":[2,4,8]}),
            config: json!({}),
        })
        .unwrap();
        let result = client
            .request(
                "run_operator_task_ir",
                json!({"mode":"execute","task_ir":prepared.task.unwrap()}),
            )
            .unwrap();
        assert_eq!(
            result["execution_runtime_status"],
            "external_operator_package_executed"
        );
        assert_eq!(
            result["operator_package_execution"]["entrypoint_sha256"],
            pin.entrypoint_sha256
        );
        assert_eq!(result["result"]["summary"]["sum"], 14.0);
        let mut conflict = pin.clone();
        conflict.package_version = "99.0.0".into();
        let result = deploy_operator_package(
            &conflict,
            &platform,
            &mut exchange,
            |_| panic!("conflict must not read or replace bytes"),
            || true,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.state, "version_conflict");
        assert_eq!(result.installed.as_ref(), Some(pin));
    }
    println!(
        "cross-host qualification: controller={}-{}, target={}, phase={}, package={}, transport={}, passed=true",
        std::env::consts::OS,
        std::env::consts::ARCH,
        platform,
        phase,
        pin.package_id,
        if tls { "native-tls" } else { "ssh-loopback" }
    );
}
