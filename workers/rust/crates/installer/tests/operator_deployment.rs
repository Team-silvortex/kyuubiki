#[path = "store_install.rs"]
mod fixture;
use kyuubiki_installer::{
    install_store_operator_into, operator_deployment_in, project_operator_in,
};
use kyuubiki_protocol::*;
use serde_json::json;

#[test]
fn remote_deployment_is_resumable_verified_and_never_replaces_a_conflict() {
    let (f, install) = fixture::Fixture::new();
    let source = f.0.join("source");
    let receipt = install_store_operator_into(&f.0.join("files"), &source, &install).unwrap();
    let status = project_operator_in(
        &source,
        serde_json::from_value(
            json!({"project_id":"p","scope":"a".repeat(64),"operation":{"action":"list"}}),
        )
        .unwrap(),
    )
    .unwrap();
    let pin = status.installed[0].clone();
    let base = source.join(receipt.relative_root);
    let spec = OperatorDeploymentSpec {
        package: pin.clone(),
        platform: kyuubiki_operator_sdk::current_platform_target_id(),
        manifest_json: std::fs::read_to_string(base.join("kyuubiki-operator.json")).unwrap(),
        size_bytes: receipt.entrypoint_size_bytes,
    };
    let bytes = std::fs::read(base.join(receipt.entrypoint_relative_path)).unwrap();
    let remote = f.0.join("remote");
    let call = |operation| operator_deployment_in(&remote, &pin, operation);
    assert_eq!(
        call(OperatorDeploymentAction::Inspect).unwrap().state,
        "missing"
    );
    assert_eq!(
        call(OperatorDeploymentAction::Begin { spec: spec.clone() })
            .unwrap()
            .received_bytes,
        0
    );
    assert!(call(OperatorDeploymentAction::Commit).is_err());
    assert!(
        call(OperatorDeploymentAction::Chunk {
            offset: 1,
            data: bytes.clone()
        })
        .is_err()
    );
    let sent = call(OperatorDeploymentAction::Chunk {
        offset: 0,
        data: bytes.clone(),
    })
    .unwrap();
    assert_eq!(sent.received_bytes, bytes.len() as u64);
    assert_eq!(
        call(OperatorDeploymentAction::Begin { spec: spec.clone() })
            .unwrap()
            .received_bytes,
        sent.received_bytes
    );
    assert_eq!(
        call(OperatorDeploymentAction::Chunk {
            offset: 0,
            data: bytes
        })
        .unwrap()
        .received_bytes,
        sent.received_bytes
    );
    assert_eq!(
        call(OperatorDeploymentAction::Commit).unwrap().state,
        "installed_restart_required"
    );
    assert_eq!(
        call(OperatorDeploymentAction::Commit).unwrap().state,
        "installed_restart_required"
    );
    let mut other = pin.clone();
    other.package_version = "2.0.0".into();
    let conflict =
        operator_deployment_in(&remote, &other, OperatorDeploymentAction::Inspect).unwrap();
    assert_eq!(conflict.state, "version_conflict");
    assert_eq!(conflict.installed.unwrap(), pin);
    assert_eq!(
        operator_deployment_in(&remote, &other, OperatorDeploymentAction::Commit)
            .unwrap()
            .state,
        "version_conflict"
    );
}

#[test]
fn remote_deployment_rejects_tampering_and_cleans_only_its_upload() {
    let (f, install) = fixture::Fixture::new();
    let source = f.0.join("source");
    let receipt = install_store_operator_into(&f.0.join("files"), &source, &install).unwrap();
    let pin = project_operator_in(
        &source,
        serde_json::from_value(
            json!({"project_id":"p","scope":"a".repeat(64),"operation":{"action":"list"}}),
        )
        .unwrap(),
    )
    .unwrap()
    .installed
    .remove(0);
    let spec = OperatorDeploymentSpec {
        package: pin.clone(),
        platform: kyuubiki_operator_sdk::current_platform_target_id(),
        manifest_json: std::fs::read_to_string(
            source
                .join(receipt.relative_root)
                .join("kyuubiki-operator.json"),
        )
        .unwrap(),
        size_bytes: receipt.entrypoint_size_bytes,
    };
    let remote = f.0.join("remote");
    operator_deployment_in(&remote, &pin, OperatorDeploymentAction::Begin { spec }).unwrap();
    operator_deployment_in(
        &remote,
        &pin,
        OperatorDeploymentAction::Chunk {
            offset: 0,
            data: vec![42; receipt.entrypoint_size_bytes as usize],
        },
    )
    .unwrap();
    assert!(operator_deployment_in(&remote, &pin, OperatorDeploymentAction::Commit).is_err());
    assert!(!remote.join("packages").join(&pin.package_id).exists());
    assert_eq!(
        operator_deployment_in(&remote, &pin, OperatorDeploymentAction::Abort)
            .unwrap()
            .state,
        "missing"
    );
    assert_eq!(
        std::fs::read_dir(remote.join("deployments"))
            .unwrap()
            .count(),
        1
    ); // Only the shared lock remains.
}

#[test]
fn remote_deployment_bounds_platform_disk_slots_and_scoped_abort() {
    use sha2::{Digest, Sha256};
    let (f, install) = fixture::Fixture::new();
    let source = f.0.join("source");
    let receipt = install_store_operator_into(&f.0.join("files"), &source, &install).unwrap();
    let pin = project_operator_in(
        &source,
        serde_json::from_value(json!({
            "project_id":"p", "scope":"a".repeat(64), "operation":{"action":"list"}
        }))
        .unwrap(),
    )
    .unwrap()
    .installed
    .remove(0);
    let spec = OperatorDeploymentSpec {
        package: pin.clone(),
        platform: kyuubiki_operator_sdk::current_platform_target_id(),
        manifest_json: std::fs::read_to_string(
            source
                .join(receipt.relative_root)
                .join("kyuubiki-operator.json"),
        )
        .unwrap(),
        size_bytes: receipt.entrypoint_size_bytes,
    };
    let remote = f.0.join("remote");
    for invalid in [
        OperatorDeploymentSpec {
            platform: "different-platform".into(),
            ..spec.clone()
        },
        OperatorDeploymentSpec {
            size_bytes: OPERATOR_DEPLOYMENT_MAX_BYTES + 1,
            ..spec.clone()
        },
        OperatorDeploymentSpec {
            manifest_json: "{}".into(),
            ..spec.clone()
        },
    ] {
        assert!(
            operator_deployment_in(
                &remote,
                &pin,
                OperatorDeploymentAction::Begin { spec: invalid }
            )
            .is_err()
        );
    }
    let mut pins = Vec::new();
    for index in 0..5 {
        let mut spec = spec.clone();
        spec.package.package_version = format!("1.0.{index}");
        let mut manifest: serde_json::Value = serde_json::from_str(&spec.manifest_json).unwrap();
        manifest["package_version"] = json!(spec.package.package_version);
        spec.manifest_json = serde_json::to_string(&manifest).unwrap();
        spec.package.manifest_sha256 =
            format!("{:x}", Sha256::digest(spec.manifest_json.as_bytes()));
        let pin = spec.package.clone();
        let result =
            operator_deployment_in(&remote, &pin, OperatorDeploymentAction::Begin { spec });
        if index == 4 {
            assert_eq!(result.unwrap_err(), "deployment_capacity");
        } else {
            assert_eq!(result.unwrap().state, "uploading");
            pins.push(pin);
        }
    }
    operator_deployment_in(&remote, &pins[0], OperatorDeploymentAction::Abort).unwrap();
    for pin in &pins[1..] {
        assert_eq!(
            operator_deployment_in(&remote, pin, OperatorDeploymentAction::Inspect)
                .unwrap()
                .state,
            "uploading"
        );
    }
    assert_eq!(
        std::fs::read_dir(remote.join("deployments"))
            .unwrap()
            .count(),
        4
    );
    assert!(!remote.join("packages").exists());
}
