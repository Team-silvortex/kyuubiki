//! Manual two-phase qualification. The operator owns the remote fixture's managed restart.
use kyuubiki_headless_sdk::deploy_operator_package;
use kyuubiki_installer::{project_operator_in, stage_store_operator_variant_into};
use kyuubiki_protocol::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn loopback_endpoint(value: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value
        .parse()
        .map_err(|_| "explicit_loopback_tunnel_required")?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err("explicit_loopback_tunnel_required".into());
    }
    Ok(address)
}

fn rpc(endpoint: SocketAddr, method: &str, params: Value) -> Result<Value, String> {
    let io = |e: std::io::Error| e.to_string();
    let mut stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(5)).map_err(io)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(io)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(io)?;
    let bytes = serde_json::to_vec(
        &json!({"rpc_version":1,"id":"cross-host-probe","method":method,"params":params}),
    )
    .unwrap();
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(io)?;
    stream.write_all(&bytes).map_err(io)?;
    for _ in 0..32 {
        let mut size = [0; 4];
        stream.read_exact(&mut size).map_err(io)?;
        let len = u32::from_be_bytes(size) as usize;
        if len == 0 || len > 16 * 1024 * 1024 {
            return Err("invalid_rpc_frame".into());
        }
        let mut bytes = vec![0; len];
        stream.read_exact(&mut bytes).map_err(io)?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_rpc_json")?;
        if value.get("ok").is_some() {
            if value["id"] != "cross-host-probe" || value["ok"] != true {
                return Err(format!("RPC rejected: {}", value["error"]["code"]));
            }
            return Ok(value["result"].clone());
        }
    }
    Err("rpc_event_limit".into())
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
#[ignore = "requires an owned Installer-managed Linux Agent over an SSH loopback tunnel and trusted template .so"]
fn staged_linux_variant_crosses_hosts_without_store_or_engine_coupling() {
    let phase = std::env::var("KYUUBIKI_CROSS_HOST_PHASE")
        .expect("deploy or verify after explicit managed restart");
    assert!(matches!(phase.as_str(), "deploy" | "verify"));
    let endpoint = loopback_endpoint(
        &std::env::var("KYUUBIKI_CROSS_HOST_ENDPOINT").expect("owned SSH loopback endpoint"),
    )
    .unwrap();
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
    let mut exchange = |request: OperatorDeploymentRequest| {
        serde_json::from_value::<OperatorDeploymentStatus>(rpc(
            endpoint,
            "manage_operator_package",
            serde_json::to_value(request).unwrap(),
        )?)
        .map_err(|e| e.to_string())
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
        )
        .unwrap();
        assert_eq!(result.state, "installed_restart_required");
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
    } else {
        assert_eq!(
            inspected.state, "ready",
            "the operator must explicitly restart the managed fixture before verification"
        );
        let prepared = project(ProjectOperatorOperation::Prepare {
            package: pin.clone(),
            operator_id: "extract.template_summary".into(),
            task_id: "cross-host-template-probe".into(),
            input: json!({"values":[2,4,8]}),
            config: json!({}),
        })
        .unwrap();
        let result = rpc(
            endpoint,
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
        "cross-host qualification: controller={}-{}, target={}, phase={}, package={}, passed=true",
        std::env::consts::OS,
        std::env::consts::ARCH,
        platform,
        phase,
        pin.package_id
    );
}
