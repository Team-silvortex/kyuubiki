use kyuubiki_installer::{StoreOperatorInstallRequest, install_store_operator_into};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

pub struct Fixture(pub PathBuf);
impl Fixture {
    pub fn new() -> (Self, StoreOperatorInstallRequest) {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-store-install-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let platform = kyuubiki_operator_sdk::current_platform_target_id();
        fs::create_dir_all(root.join("files").join(&platform)).unwrap();
        let (name, bytes) = if platform.starts_with("macos-") {
            let mut bytes = vec![0; 32];
            bytes[..4].copy_from_slice(&0xfeedfacf_u32.to_le_bytes());
            bytes[4..8].copy_from_slice(
                &(if platform.ends_with("aarch64") {
                    0x0100000c_u32
                } else {
                    0x01000007_u32
                })
                .to_le_bytes(),
            );
            bytes[12..16].copy_from_slice(&6_u32.to_le_bytes());
            ("operator.dylib", bytes)
        } else if platform.starts_with("linux-") {
            let mut bytes = vec![0; 64];
            bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
            bytes[16..18].copy_from_slice(&3_u16.to_le_bytes());
            bytes[18..20].copy_from_slice(
                &(if platform.ends_with("aarch64") {
                    183_u16
                } else {
                    62_u16
                })
                .to_le_bytes(),
            );
            bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
            bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
            ("operator.so", bytes)
        } else {
            let mut bytes = vec![0; 512];
            bytes[..2].copy_from_slice(b"MZ");
            bytes[60..64].copy_from_slice(&128_u32.to_le_bytes());
            bytes[128..132].copy_from_slice(b"PE\0\0");
            bytes[132..134].copy_from_slice(&0x8664_u16.to_le_bytes());
            bytes[148..150].copy_from_slice(&240_u16.to_le_bytes());
            bytes[150..152].copy_from_slice(&0x2000_u16.to_le_bytes());
            bytes[152..154].copy_from_slice(&0x20b_u16.to_le_bytes());
            ("operator.dll", bytes)
        };
        let manifest = json!({"schema_version":"kyuubiki.operator-package/v1","sdk_api_version":"kyuubiki.operator-sdk/v1",
            "execution_abi":"kyuubiki.operator-json-c/v1","package_id":"operator.store.fixture","package_version":"1.0.0",
            "minimum_host_version":"3.7.8","validation_status":"partial","validation_notes":"Header fixture; never execute.",
            "runtime":"rust_crate","entrypoint":name,"operators":[{"operator_id":"extract.store_fixture","kind":"extract","entry_symbol":"run_operator_json"}]});
        let mut files = vec![];
        for (name, data) in [
            (
                "kyuubiki-operator.json",
                serde_json::to_vec(&manifest).unwrap(),
            ),
            (name, bytes),
        ] {
            let path = format!("{platform}/{name}");
            fs::write(root.join("files").join(&path), &data).unwrap();
            files.push(json!({"path":path,"sha256":format!("{:x}",Sha256::digest(&data)),"size_bytes":data.len()}));
        }
        let request = serde_json::from_value(json!({"schema_version":"kyuubiki.store-operator-install/v1", "package_id":"operator.store.fixture", "package_version":"1.0.0", "platform":platform,"files":files})).unwrap();
        (Self(root), request)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_store_install_binds_identity_hashes_and_replays_without_replacement() {
    let (f, request) = Fixture::new();
    let store = f.0.join("managed");
    let receipt = install_store_operator_into(&f.0.join("files"), &store, &request).unwrap();
    assert_eq!(receipt.package_id, request.package_id);
    assert_eq!(
        receipt,
        install_store_operator_into(&f.0.join("files"), &store, &request).unwrap()
    );
    let mut wrong = serde_json::to_value(&request).unwrap();
    wrong["package_version"] = json!("1.0.1");
    assert!(
        install_store_operator_into(
            &f.0.join("files"),
            &store,
            &serde_json::from_value(wrong).unwrap()
        )
        .is_err()
    );
    assert_eq!(
        kyuubiki_installer::verify_managed_operator_package(&store.join(&receipt.relative_root))
            .unwrap(),
        receipt
    );
}

#[test]
fn native_store_install_rejects_tamper_cross_platform_extra_files_and_manifest_aliases() {
    for mode in [
        "hash",
        "platform",
        "extra",
        "id",
        "entrypoint",
        "unknown",
        "header",
        "abi",
        "host",
    ] {
        let (f, request) = Fixture::new();
        let mut raw = serde_json::to_value(request).unwrap();
        let files = f.0.join("files");
        match mode {
            "hash" => raw["files"][0]["sha256"] = json!("0".repeat(64)),
            "platform" => raw["platform"] = json!("any"),
            "extra" => {
                let extra = raw["files"][0].clone();
                raw["files"].as_array_mut().unwrap().push(extra);
            }
            "id" => raw["package_id"] = json!("other"),
            _ => {
                let index = if mode == "header" { 1 } else { 0 };
                let path = files.join(raw["files"][index]["path"].as_str().unwrap());
                let bytes = if mode == "header" {
                    b"not a shared library".to_vec()
                } else {
                    let mut manifest: Value =
                        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                    if mode == "entrypoint" {
                        manifest["entrypoint"] = json!("../elsewhere.dylib");
                    } else if mode == "abi" {
                        manifest["execution_abi"] = json!("unsupported");
                    } else if mode == "host" {
                        manifest["minimum_host_version"] = json!("999.0.0");
                    } else {
                        manifest["install_hook"] = json!("never execute this");
                    }
                    serde_json::to_vec(&manifest).unwrap()
                };
                fs::write(path, &bytes).unwrap();
                raw["files"][index]["sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
                raw["files"][index]["size_bytes"] = json!(bytes.len());
            }
        }
        assert!(
            install_store_operator_into(
                &files,
                &f.0.join("managed"),
                &serde_json::from_value(raw).unwrap()
            )
            .is_err(),
            "{mode}"
        );
        assert!(!f.0.join("managed/packages/operator.store.fixture").exists());
    }
}

#[cfg(unix)]
#[test]
fn store_install_does_not_follow_symbolic_package_files() {
    let (f, request) = Fixture::new();
    let path = f.0.join("files").join(&request.files[0].path);
    let original = fs::read(&path).unwrap();
    let outside = f.0.join("outside.json");
    fs::rename(&path, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert!(
        install_store_operator_into(&f.0.join("files"), &f.0.join("managed"), &request).is_err()
    );
    assert_eq!(fs::read(outside).unwrap(), original);
    assert!(!f.0.join("managed").exists());
}

#[test]
fn native_store_command_reads_bounded_handoff_and_returns_real_receipt() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let (f, request) = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-installer"))
        .arg("install-store-operator")
        .arg(f.0.join("files"))
        .arg("--store-root")
        .arg(f.0.join("managed"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["package_id"], request.package_id);
    assert_eq!(
        receipt["schema_version"],
        "kyuubiki.managed-operator-package-receipt/v1"
    );
}
