#[path = "store_install.rs"]
mod fixture;
use kyuubiki_installer::{project_operator_in, stage_store_operator_variant_into};
use kyuubiki_protocol::{ProjectOperatorRequest, StoreOperatorInstallRequest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;

fn linux_fixture(platform: &str) -> (fixture::Fixture, StoreOperatorInstallRequest) {
    let (f, local) = fixture::Fixture::new();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(f.0.join("files").join(&local.files[0].path)).unwrap())
            .unwrap();
    manifest["entrypoint"] = json!("operator.so");
    let mut elf = vec![0; 64];
    elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    elf[16..18].copy_from_slice(&3_u16.to_le_bytes());
    elf[18..20].copy_from_slice(
        &(if platform == "linux-aarch64" {
            183_u16
        } else {
            62
        })
        .to_le_bytes(),
    );
    elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
    elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
    fs::create_dir_all(f.0.join("linux-files").join(platform)).unwrap();
    let mut files = vec![];
    for (name, bytes) in [
        (
            "kyuubiki-operator.json",
            serde_json::to_vec(&manifest).unwrap(),
        ),
        ("operator.so", elf),
    ] {
        let path = format!("{platform}/{name}");
        fs::write(f.0.join("linux-files").join(&path), &bytes).unwrap();
        files.push(json!({"path":path,"sha256":format!("{:x}",Sha256::digest(&bytes)),"size_bytes":bytes.len()}));
    }
    let request=serde_json::from_value(json!({"schema_version":"kyuubiki.store-operator-install/v1",
        "package_id":local.package_id,"package_version":local.package_version,"platform":platform,"files":files})).unwrap();
    (f, request)
}
fn request(platform: Option<&str>, operation: Value) -> ProjectOperatorRequest {
    serde_json::from_value(json!({"project_id":"research","scope":"a".repeat(64),"platform":platform,"operation":operation})).unwrap()
}
#[test]
fn linux_variants_stage_enable_and_prepare_without_polluting_local_library() {
    let (cache, _) = fixture::Fixture::new();
    let root = cache.0.join("managed");
    for platform in ["linux-x86_64", "linux-aarch64"] {
        let (f, install) = linux_fixture(platform);
        let staged =
            stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).unwrap();
        assert_eq!(staged.platform, platform);
        assert!(
            !root.join("packages").exists(),
            "never install foreign code into this host library"
        );
        assert_eq!(
            stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).unwrap(),
            staged
        );
        assert!(
            project_operator_in(&root, request(None, json!({"action":"list"})))
                .unwrap()
                .installed
                .is_empty()
        );
        let list =
            project_operator_in(&root, request(Some(platform), json!({"action":"list"}))).unwrap();
        assert_eq!(list.installed, vec![staged.package.clone()]);
        assert!(
            list.enabled.is_empty(),
            "another architecture cannot enable this variant"
        );
        let op = |action| json!({"action":action,"package":staged.package});
        project_operator_in(&root, request(Some(platform), op("enable"))).unwrap();
        let read = || json!({"action":"read_deployment","package":staged.package,"offset":0});
        assert!(project_operator_in(&root, request(None, read())).is_err());
        let bytes = project_operator_in(&root, request(Some(platform), read()))
            .unwrap()
            .deployment
            .unwrap();
        assert_eq!(bytes.spec.platform, platform);
        assert_eq!(&bytes.data[..4], b"\x7fELF");
        let prepared=project_operator_in(&root,request(Some(platform),json!({"action":"prepare","package":staged.package,
            "operator_id":"extract.store_fixture","task_id":"research-task","input":{},"config":{}}))).unwrap();
        assert_eq!(
            prepared.task.unwrap()["execution_program"]["package_integrity"]["digest"],
            staged.package.entrypoint_sha256
        );
        assert!(
            project_operator_in(
                &root,
                request(Some("../packages"), json!({"action":"list"}))
            )
            .is_err()
        );
    }
}
#[test]
fn foreign_variant_rejects_wrong_architecture_tamper_and_conflict() {
    let (f, install) = linux_fixture("linux-x86_64");
    let root = f.0.join("managed");
    let old = stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).unwrap();
    let path = f.0.join("linux-files").join(&install.files[1].path);
    let mut bytes = fs::read(&path).unwrap();
    bytes[18..20].copy_from_slice(&183_u16.to_le_bytes());
    fs::write(&path, &bytes).unwrap();
    assert!(stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).is_err());
    let mut wrong = install.clone();
    wrong.files[1].sha256 = format!("{:x}", Sha256::digest(&bytes));
    assert!(stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &wrong).is_err());
    bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
    bytes[63] = 1;
    fs::write(&path, &bytes).unwrap();
    wrong.files[1].sha256 = format!("{:x}", Sha256::digest(&bytes));
    assert!(stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &wrong).is_err());
    assert_eq!(
        project_operator_in(
            &root,
            request(Some("linux-x86_64"), json!({"action":"list"}))
        )
        .unwrap()
        .installed,
        vec![old.package]
    );
}
#[test]
fn variant_native_command_has_a_distinct_receipt_and_bounded_cache() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let (f, install) = linux_fixture("linux-x86_64");
    let root = f.0.join("managed");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-installer"))
        .arg("stage-store-operator-variant")
        .arg(f.0.join("linux-files"))
        .arg("--store-root")
        .arg(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&install).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["schema_version"], "kyuubiki.operator-variant/v1");
    assert!(!root.join("packages").exists());
    let target = root.join("variants/linux-x86_64");
    fs::create_dir(target.join("variant.pending")).unwrap();
    fs::write(target.join("variant.pending/unconfirmed"), b"partial").unwrap();
    // Different IDs exercise bounded capacity, not replacement of an existing release.
    for index in 1..=8 {
        let mut next = install.clone();
        next.package_id = format!("fixture-{index}");
        let path = f.0.join("linux-files").join(&next.files[0].path);
        let mut m: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        m["package_id"] = json!(next.package_id);
        let bytes = serde_json::to_vec(&m).unwrap();
        fs::write(path, &bytes).unwrap();
        next.files[0].sha256 = format!("{:x}", Sha256::digest(&bytes));
        next.files[0].size_bytes = bytes.len() as u64;
        let result = stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &next);
        if index < 8 {
            result.unwrap();
        } else {
            assert_eq!(result.err().unwrap(), "operator_variant_capacity");
        }
        assert!(!target.join("variant.pending").exists());
    }
}
#[cfg(unix)]
#[test]
fn variant_cache_rejects_symlink_and_detects_changed_pinned_bytes() {
    for mode in ["root", "package", "receipt", "library"] {
        let (f, install) = linux_fixture("linux-aarch64");
        let root = f.0.join("managed");
        let receipt =
            stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).unwrap();
        let path = root.join(&receipt.relative_root);
        project_operator_in(
            &root,
            request(
                Some("linux-aarch64"),
                json!({"action":"enable","package":receipt.package}),
            ),
        )
        .unwrap();
        let target = match mode {
            "root" => root.join("variants"),
            "package" => path.clone(),
            "receipt" => path.join("variant.json"),
            _ => path.join("operator.so"),
        };
        let outside = f.0.join("untouched");
        fs::rename(&target, &outside).unwrap();
        std::os::unix::fs::symlink(&outside, &target).unwrap();
        assert!(
            stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).is_err()
        );
        assert!(
            project_operator_in(
                &root,
                request(
                    Some("linux-aarch64"),
                    json!({"action":"prepare","package":receipt.package,
            "operator_id":"extract.store_fixture","task_id":"test","input":{},"config":{}})
                )
            )
            .is_err()
        );
        assert!(outside.exists());
    }
}

#[test]
fn changed_variant_cannot_retain_project_readiness_or_prepare_a_task() {
    let (f, install) = linux_fixture("linux-x86_64");
    let root = f.0.join("managed");
    let receipt =
        stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).unwrap();
    project_operator_in(
        &root,
        request(
            Some("linux-x86_64"),
            json!({"action":"enable","package":receipt.package}),
        ),
    )
    .unwrap();
    let path = root.join(&receipt.relative_root).join("operator.so");
    let mut bytes = fs::read(&path).unwrap();
    bytes[63] ^= 1;
    fs::write(&path, bytes).unwrap();
    let status = project_operator_in(
        &root,
        request(Some("linux-x86_64"), json!({"action":"list"})),
    )
    .unwrap();
    assert!(status.installed_error.is_some());
    assert_eq!(status.enabled[0].state, "unavailable_or_changed");
    assert!(
        project_operator_in(
            &root,
            request(
                Some("linux-x86_64"),
                json!({"action":"prepare","package":receipt.package,
        "operator_id":"extract.store_fixture","task_id":"test","input":{},"config":{}})
            )
        )
        .is_err()
    );
    assert!(
        stage_store_operator_variant_into(&f.0.join("linux-files"), &root, &install).is_err(),
        "do not overwrite an unconfirmed local cache"
    );
}
