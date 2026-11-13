use kyuubiki_installer::{OperatorReleaseArtifact, prepare_operator_distribution};
use kyuubiki_operator_sdk::read_operator_package_distribution;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-release-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("manifest.json"), json!({
            "schema_version":"kyuubiki.operator-package/v1", "sdk_api_version":"kyuubiki.operator-sdk/v1",
            "execution_abi":"kyuubiki.operator-json-c/v1", "package_id":"operator.test.release", "package_version":"0.1.0",
            "minimum_host_version":"3.7.8", "validation_status":"partial", "validation_notes":"Header-only test fixture, not executable validation.",
            "runtime":"rust_crate", "entrypoint":"target/debug/{lib_prefix}example.{lib_extension}",
            "operators":[{"operator_id":"extract.release_test", "kind":"extract", "entry_symbol":"run_operator_json"}]
        }).to_string()).unwrap();
        Self(path)
    }
    fn artifact(&self, triple: &str, bytes: &[u8]) -> OperatorReleaseArtifact {
        let path = self.0.join(format!("{triple}.bin"));
        fs::write(&path, bytes).unwrap();
        OperatorReleaseArtifact {
            rust_target: triple.into(),
            library_path: path,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn elf(machine: u16) -> Vec<u8> {
    let mut b = vec![0; 64];
    b[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    b[16..18].copy_from_slice(&3_u16.to_le_bytes());
    b[18..20].copy_from_slice(&machine.to_le_bytes());
    b[20..24].copy_from_slice(&1_u32.to_le_bytes());
    b[52..54].copy_from_slice(&64_u16.to_le_bytes());
    b
}
fn macho(cpu: u32) -> Vec<u8> {
    let mut b = vec![0; 32];
    b[..4].copy_from_slice(&0xfeedfacf_u32.to_le_bytes());
    b[4..8].copy_from_slice(&cpu.to_le_bytes());
    b[12..16].copy_from_slice(&6_u32.to_le_bytes());
    b
}
fn pe() -> Vec<u8> {
    let mut b = vec![0; 512];
    b[..2].copy_from_slice(b"MZ");
    b[60..64].copy_from_slice(&128_u32.to_le_bytes());
    b[128..132].copy_from_slice(b"PE\0\0");
    b[132..134].copy_from_slice(&0x8664_u16.to_le_bytes());
    b[148..150].copy_from_slice(&240_u16.to_le_bytes());
    b[150..152].copy_from_slice(&0x2000_u16.to_le_bytes());
    b[152..154].copy_from_slice(&0x20b_u16.to_le_bytes());
    b
}

#[test]
fn prepares_deterministic_five_target_distribution_without_loading_code() {
    let f = Fixture::new();
    let mut artifacts = vec![
        f.artifact("aarch64-apple-darwin", &macho(0x0100000c)),
        f.artifact("x86_64-apple-darwin", &macho(0x01000007)),
        f.artifact("x86_64-unknown-linux-gnu", &elf(62)),
        f.artifact("aarch64-unknown-linux-gnu", &elf(183)),
        f.artifact("x86_64-pc-windows-msvc", &pe()),
    ];
    let one = f.0.join("one");
    let two = f.0.join("two");
    let result =
        prepare_operator_distribution(&f.0.join("manifest.json"), &artifacts, &one).unwrap();
    artifacts.reverse();
    prepare_operator_distribution(&f.0.join("manifest.json"), &artifacts, &two).unwrap();
    assert_eq!(result.artifacts.len(), 5);
    let name = "kyuubiki-operator-distribution.json";
    assert_eq!(
        fs::read(one.join(name)).unwrap(),
        fs::read(two.join(name)).unwrap()
    );
    assert_eq!(
        read_operator_package_distribution(one.join(name)).unwrap(),
        result
    );
    for a in result.artifacts {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(one.join(&a.entrypoint_path)).unwrap())
            ),
            a.entrypoint_sha256
        );
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(one.join(&a.manifest_path)).unwrap())
            ),
            a.manifest_sha256
        );
        assert_eq!(
            fs::metadata(one.join(&a.entrypoint_path)).unwrap().len(),
            a.entrypoint_size_bytes
        );
        let m: serde_json::Value =
            serde_json::from_slice(&fs::read(one.join(&a.manifest_path)).unwrap()).unwrap();
        assert_eq!(
            m["entrypoint"],
            a.entrypoint_path.split('/').next_back().unwrap()
        );
        assert_eq!(m["validation_status"], "partial");
    }
    let before = fs::read(one.join(name)).unwrap();
    assert!(prepare_operator_distribution(&f.0.join("manifest.json"), &artifacts, &one).is_err());
    assert_eq!(before, fs::read(one.join(name)).unwrap());
}

#[test]
fn rejects_executables_truncation_and_out_of_bounds_header_offsets() {
    let f = Fixture::new();
    let out = f.0.join("out");
    let mut executable = macho(0x0100000c);
    executable[12] = 2;
    let mut elf32 = elf(62);
    elf32[4] = 1;
    let mut pe_exe = pe();
    pe_exe[151] = 0;
    let mut pe_offset = pe();
    pe_offset[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut pe_short = pe();
    pe_short.truncate(155);
    for (triple, bytes) in [
        ("aarch64-apple-darwin", executable),
        ("x86_64-unknown-linux-gnu", elf32),
        ("x86_64-pc-windows-msvc", pe_exe),
        ("x86_64-pc-windows-msvc", pe_offset),
        ("x86_64-pc-windows-msvc", pe_short),
        ("aarch64-apple-darwin", vec![0; 3]),
    ] {
        let artifact = f.artifact(triple, &bytes);
        assert!(
            prepare_operator_distribution(&f.0.join("manifest.json"), &[artifact], &out).is_err()
        );
        assert!(!out.exists());
    }
}

#[test]
fn respects_publication_lock_size_limits_and_manifest_contract() {
    let f = Fixture::new();
    let out = f.0.join("out");
    let good = f.artifact("x86_64-unknown-linux-gnu", &elf(62));
    fs::write(f.0.join(".out.operator-release.lock"), b"other writer").unwrap();
    assert!(
        prepare_operator_distribution(
            &f.0.join("manifest.json"),
            std::slice::from_ref(&good),
            &out
        )
        .is_err()
    );
    assert_eq!(
        fs::read(f.0.join(".out.operator-release.lock")).unwrap(),
        b"other writer"
    );
    fs::remove_file(f.0.join(".out.operator-release.lock")).unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join("manifest.json")).unwrap()).unwrap();
    manifest["publish_hook"] = json!("must not be discarded or executed");
    fs::write(f.0.join("unknown.json"), manifest.to_string()).unwrap();
    assert!(
        prepare_operator_distribution(&f.0.join("unknown.json"), std::slice::from_ref(&good), &out)
            .is_err()
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&good.library_path)
        .unwrap()
        .set_len(512 * 1024 * 1024 + 1)
        .unwrap();
    assert!(prepare_operator_distribution(&f.0.join("manifest.json"), &[good], &out).is_err());
    assert!(!out.exists());
}

#[cfg(unix)]
#[test]
fn rejects_symlink_input_and_dangling_output_without_removing_either() {
    let f = Fixture::new();
    let out = f.0.join("out");
    let mut artifact = f.artifact("x86_64-unknown-linux-gnu", &elf(62));
    let link = f.0.join("link.so");
    std::os::unix::fs::symlink(&artifact.library_path, &link).unwrap();
    artifact.library_path = link.clone();
    assert!(prepare_operator_distribution(&f.0.join("manifest.json"), &[artifact], &out).is_err());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    std::os::unix::fs::symlink(f.0.join("missing"), &out).unwrap();
    let artifact = f.artifact("x86_64-unknown-linux-gnu", &elf(62));
    assert!(prepare_operator_distribution(&f.0.join("manifest.json"), &[artifact], &out).is_err());
    assert!(out.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn native_command_prepares_candidate_and_rejects_incomplete_arguments() {
    let f = Fixture::new();
    let artifact = f.artifact("x86_64-unknown-linux-gnu", &elf(62));
    let binary = env!("CARGO_BIN_EXE_kyuubiki-installer");
    let response = std::process::Command::new(binary)
        .arg("prepare-operator-distribution")
        .arg(f.0.join("manifest.json"))
        .arg(f.0.join("out"))
        .arg(format!(
            "{}={}",
            artifact.rust_target,
            artifact.library_path.display()
        ))
        .output()
        .unwrap();
    assert!(
        response.status.success(),
        "{}",
        String::from_utf8_lossy(&response.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&response.stdout).unwrap();
    assert_eq!(value["artifacts"][0]["target"], "linux-x86_64");
    assert!(
        !std::process::Command::new(binary)
            .arg("prepare-operator-distribution")
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn rejects_identities_that_cannot_be_installed_portably() {
    let f = Fixture::new();
    let good = f.artifact("x86_64-unknown-linux-gnu", &elf(62));
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join("manifest.json")).unwrap()).unwrap();
    for (field, value) in [
        ("package_id", "CON"),
        ("package_version", "nul"),
        ("package_id", "../escape"),
    ] {
        let mut m = original.clone();
        m[field] = json!(value);
        fs::write(f.0.join("bad.json"), m.to_string()).unwrap();
        assert!(
            prepare_operator_distribution(
                &f.0.join("bad.json"),
                std::slice::from_ref(&good),
                &f.0.join("out")
            )
            .is_err(),
            "{field}={value}"
        );
    }
}

#[test]
fn rejects_wrong_architecture_duplicate_target_and_unsupported_abi_without_residue() {
    let f = Fixture::new();
    let out = f.0.join("out");
    let wrong = f.artifact("aarch64-apple-darwin", &macho(0x01000007));
    assert!(prepare_operator_distribution(&f.0.join("manifest.json"), &[wrong], &out).is_err());
    let good = f.artifact("x86_64-unknown-linux-gnu", &elf(62));
    assert!(
        prepare_operator_distribution(
            &f.0.join("manifest.json"),
            &[good.clone(), good.clone()],
            &out
        )
        .is_err()
    );
    let unsupported = f.artifact("x86_64-unknown-linux-musl", &elf(62));
    assert!(
        prepare_operator_distribution(&f.0.join("manifest.json"), &[unsupported], &out).is_err()
    );
    let broken = f.artifact("x86_64-pc-windows-msvc", b"MZ");
    assert!(
        prepare_operator_distribution(&f.0.join("manifest.json"), &[good.clone(), broken], &out)
            .is_err()
    );
    assert!(!out.exists());
    assert!(
        !fs::read_dir(&f.0).unwrap().any(|p| p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".out"))
    );
    prepare_operator_distribution(&f.0.join("manifest.json"), &[good], &out).unwrap();
}
