use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

pub(super) const PACKAGE_ID: &str = "extract.template_summary";
pub(super) const PACKAGE_VERSION: &str = "0.1.0";
pub(super) const TARGET: &str = "linux-x86_64";
pub(super) const ENTRYPOINT_NAME: &str = "libkyuubiki_operator_template.so";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DistributionEvidence {
    pub(super) package_id: String,
    pub(super) package_version: String,
    pub(super) target: String,
    pub(super) sdk_api_version: String,
    pub(super) execution_abi: String,
    pub(super) entrypoint_sha256: String,
    pub(super) entrypoint_size_bytes: u64,
    pub(super) manifest_sha256: String,
    pub(super) distribution_sha256: String,
    pub(super) authority_mode: String,
    pub(super) source_copy_count: u64,
}

pub(super) fn entrypoint_path(root: &Path) -> PathBuf {
    root.join(PACKAGE_ID)
        .join(PACKAGE_VERSION)
        .join(TARGET)
        .join(ENTRYPOINT_NAME)
}

pub(super) fn seal(root: &Path) -> RunnerResult<DistributionEvidence> {
    let target_root = root.join(PACKAGE_ID).join(PACKAGE_VERSION).join(TARGET);
    fs::create_dir_all(&target_root)
        .map_err(|error| format!("failed to create distribution target: {error}"))?;
    let entrypoint = entrypoint_path(root);
    let entrypoint_size_bytes = regular_file_size(&entrypoint)?;
    let entrypoint_sha256 = sha256_file(&entrypoint)?;

    let manifest = json!({
        "schema_version": "kyuubiki.operator-package/v1",
        "sdk_api_version": "kyuubiki.operator-sdk/v1",
        "execution_abi": "kyuubiki.operator-json-c/v1",
        "package_id": PACKAGE_ID,
        "package_version": PACKAGE_VERSION,
        "minimum_host_version": "2.19.0",
        "validation_status": "verified",
        "validation_notes": "Two-host central acquisition operational fixture.",
        "runtime": "rust_crate",
        "entrypoint": "{lib_prefix}kyuubiki_operator_template.{lib_extension}",
        "operators": [{
            "operator_id": PACKAGE_ID,
            "kind": "extract",
            "entry_symbol": "run_template_operator_json"
        }]
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("failed to encode operator manifest: {error}"))?;
    let manifest_sha256 = sha256_bytes(&manifest_bytes);
    fs::write(target_root.join("kyuubiki-operator.json"), &manifest_bytes)
        .map_err(|error| format!("failed to write operator manifest: {error}"))?;

    let distribution = json!({
        "schema_version": "kyuubiki.operator-package-distribution/v1",
        "sdk_api_version": "kyuubiki.operator-sdk/v1",
        "execution_abi": "kyuubiki.operator-json-c/v1",
        "package_id": PACKAGE_ID,
        "package_version": PACKAGE_VERSION,
        "artifacts": [{
            "target": TARGET,
            "manifest_path": format!("{TARGET}/kyuubiki-operator.json"),
            "manifest_sha256": manifest_sha256,
            "manifest_size_bytes": manifest_bytes.len(),
            "entrypoint_path": format!("{TARGET}/{ENTRYPOINT_NAME}"),
            "entrypoint_sha256": entrypoint_sha256,
            "entrypoint_size_bytes": entrypoint_size_bytes
        }]
    });
    let distribution_bytes = serde_json::to_vec_pretty(&distribution)
        .map_err(|error| format!("failed to encode operator distribution: {error}"))?;
    let distribution_sha256 = sha256_bytes(&distribution_bytes);
    fs::write(
        root.join(PACKAGE_ID)
            .join(PACKAGE_VERSION)
            .join("kyuubiki-operator-distribution.json"),
        distribution_bytes,
    )
    .map_err(|error| format!("failed to write operator distribution: {error}"))?;

    Ok(DistributionEvidence {
        package_id: PACKAGE_ID.to_string(),
        package_version: PACKAGE_VERSION.to_string(),
        target: TARGET.to_string(),
        sdk_api_version: "kyuubiki.operator-sdk/v1".to_string(),
        execution_abi: "kyuubiki.operator-json-c/v1".to_string(),
        entrypoint_sha256,
        entrypoint_size_bytes,
        manifest_sha256,
        distribution_sha256,
        authority_mode: "bound_orchestra".to_string(),
        source_copy_count: 1,
    })
}

pub(super) fn expected_paths() -> [String; 3] {
    let base = format!("/api/v1/central/operator-packages/{PACKAGE_ID}/{PACKAGE_VERSION}/{TARGET}");
    [
        format!("{base}/resolve"),
        format!("{base}/manifest"),
        format!("{base}/entrypoint"),
    ]
}

fn regular_file_size(path: &Path) -> RunnerResult<u64> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        Err("remote operator entrypoint is not a non-empty regular file".to_string())
    } else {
        Ok(metadata.len())
    }
}

fn sha256_file(path: &Path) -> RunnerResult<String> {
    let mut file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_operator_sdk::{
        operator_package_manifest_readiness, read_operator_package_manifest,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn package_paths_are_canonical_and_target_specific() {
        let paths = expected_paths();
        assert!(paths.iter().all(|path| path.contains(TARGET)));
        assert!(paths[0].ends_with("/resolve"));
        assert!(paths[2].ends_with("/entrypoint"));
    }

    #[test]
    fn sealed_manifest_passes_operator_sdk_readiness() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-package-acquisition-manifest-{}-{nonce}",
            std::process::id()
        ));
        let entrypoint = entrypoint_path(&root);
        fs::create_dir_all(entrypoint.parent().expect("entrypoint parent"))
            .expect("create fixture root");
        fs::write(&entrypoint, b"qualification-entrypoint").expect("write fixture entrypoint");
        seal(&root).expect("seal distribution");
        let manifest = read_operator_package_manifest(
            root.join(PACKAGE_ID)
                .join(PACKAGE_VERSION)
                .join(TARGET)
                .join("kyuubiki-operator.json"),
        )
        .expect("read sealed manifest");
        let readiness = operator_package_manifest_readiness(&manifest);
        let _ = fs::remove_dir_all(&root);
        assert!(readiness.ok, "issues: {:?}", readiness.issues);
    }
}
