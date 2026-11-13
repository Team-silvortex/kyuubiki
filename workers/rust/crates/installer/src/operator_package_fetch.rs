use crate::{
    ManagedOperatorPackageReceipt, install_operator_package_into, operator_package_store_root,
};
use kyuubiki_operator_sdk::{
    OPERATOR_JSON_ABI_SCHEMA_VERSION, OPERATOR_PACKAGE_MANIFEST_FILE,
    OPERATOR_PACKAGE_SCHEMA_VERSION, OPERATOR_SDK_API_VERSION, OperatorPackageManifest,
    current_platform_target_id, expand_platform_library_template,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const OPERATOR_PACKAGE_RESOLUTION_SCHEMA_VERSION: &str =
    "kyuubiki.operator-package-resolution/v1";
const MAX_RESOLUTION_BYTES: u64 = 1_048_576;
const MAX_MANIFEST_BYTES: u64 = 1_048_576;
const MAX_ENTRYPOINT_BYTES: u64 = 536_870_912;

#[derive(Debug, Deserialize)]
struct OperatorPackageResolution {
    schema_version: String,
    package_ref: String,
    package_id: String,
    package_version: String,
    sdk_api_version: String,
    execution_abi: String,
    target: String,
    authority_mode: String,
    cache_scope: String,
    distribution_sha256: String,
    manifest: ResolvedArtifact,
    entrypoint: ResolvedArtifact,
}

#[derive(Debug, Deserialize)]
struct ResolvedArtifact {
    path: String,
    sha256: String,
    size_bytes: u64,
    download_path: String,
}

pub fn fetch_operator_package(
    central_url: &str,
    package_id: &str,
    package_version: &str,
    bearer_token: Option<&str>,
) -> Result<ManagedOperatorPackageReceipt, String> {
    fetch_operator_package_into(
        central_url,
        package_id,
        package_version,
        &operator_package_store_root()?,
        bearer_token,
    )
}

pub fn fetch_operator_package_into(
    central_url: &str,
    package_id: &str,
    package_version: &str,
    store_root: &Path,
    bearer_token: Option<&str>,
) -> Result<ManagedOperatorPackageReceipt, String> {
    let central_url = validate_central_url(central_url)?;
    validate_portable_token(package_id, "package_id")?;
    validate_portable_token(package_version, "package_version")?;
    validate_bearer_token(bearer_token)?;
    let target = current_platform_target_id();
    let expected_base_path =
        format!("/api/v1/central/operator-packages/{package_id}/{package_version}/{target}");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .max_redirects(0)
        .build()
        .into();

    let resolution_url = format!("{central_url}{expected_base_path}/resolve");
    let resolution_bytes = get_bytes(
        &agent,
        &resolution_url,
        bearer_token,
        MAX_RESOLUTION_BYTES,
        "operator package resolution",
    )?;
    let resolution: OperatorPackageResolution = serde_json::from_slice(&resolution_bytes)
        .map_err(|error| format!("failed to decode operator package resolution: {error}"))?;
    validate_resolution(
        &resolution,
        package_id,
        package_version,
        &target,
        &expected_base_path,
    )?;

    let manifest_bytes = download_artifact(
        &agent,
        &central_url,
        bearer_token,
        &resolution.manifest,
        MAX_MANIFEST_BYTES,
        "operator package manifest",
    )?;
    let manifest: OperatorPackageManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            format!("failed to decode downloaded operator package manifest: {error}")
        })?;
    validate_downloaded_manifest(&manifest, package_id, package_version)?;

    let temporary = DownloadedPackage::prepare(&manifest, &manifest_bytes)?;
    download_entrypoint(
        &agent,
        &central_url,
        bearer_token,
        &resolution.entrypoint,
        &temporary
            .root
            .join(expand_platform_library_template(&manifest.entrypoint)),
    )?;
    install_operator_package_into(&temporary.root, store_root)
}

fn download_entrypoint(
    agent: &ureq::Agent,
    central_url: &str,
    bearer_token: Option<&str>,
    artifact: &ResolvedArtifact,
    path: &Path,
) -> Result<(), String> {
    let mut request = agent.get(format!("{central_url}{}", artifact.download_path));
    if let Some(token) = bearer_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request
        .call()
        .map_err(|error| format!("failed to fetch operator package entrypoint: {error}"))?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(artifact.size_bytes + 1)
        .reader();
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("failed to create operator package entrypoint: {error}"))?;
    copy_verified_entrypoint(
        &mut reader,
        &mut file,
        artifact.size_bytes,
        &artifact.sha256,
    )?;
    file.sync_all()
        .map_err(|error| format!("failed to sync operator package entrypoint: {error}"))
}

fn copy_verified_entrypoint(
    reader: &mut impl Read,
    writer: &mut impl Write,
    expected_size: u64,
    expected_digest: &str,
) -> Result<(), String> {
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        // Read at most one extra byte, but never persist bytes beyond the declared size.
        let limit = (expected_size - total + 1).min(buffer.len() as u64) as usize;
        let count = match reader.read(&mut buffer[..limit]) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(format!(
                    "failed to read operator package entrypoint: {error}"
                ));
            }
        };
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > expected_size {
            return Err("operator package entrypoint size mismatch".into());
        }
        writer
            .write_all(&buffer[..count])
            .map_err(|error| format!("failed to write operator package entrypoint: {error}"))?;
        digest.update(&buffer[..count]);
    }
    if total != expected_size {
        return Err("operator package entrypoint size mismatch".into());
    }
    if format!("{:x}", digest.finalize()) != expected_digest {
        return Err("operator package entrypoint digest mismatch".into());
    }
    Ok(())
}

fn validate_resolution(
    resolution: &OperatorPackageResolution,
    package_id: &str,
    package_version: &str,
    target: &str,
    expected_base_path: &str,
) -> Result<(), String> {
    if resolution.schema_version != OPERATOR_PACKAGE_RESOLUTION_SCHEMA_VERSION
        || resolution.sdk_api_version != OPERATOR_SDK_API_VERSION
        || resolution.execution_abi != OPERATOR_JSON_ABI_SCHEMA_VERSION
    {
        return Err("unsupported operator package resolution contract".to_string());
    }
    if resolution.package_id != package_id
        || resolution.package_version != package_version
        || resolution.target != target
        || resolution.package_ref != format!("orchestra://operator-package/{package_id}")
    {
        return Err("operator package resolution identity mismatch".to_string());
    }
    if resolution.authority_mode != "bound_orchestra"
        || resolution.cache_scope != "task_required_disposable"
    {
        return Err(
            "operator package resolution violates the Agent authority boundary".to_string(),
        );
    }
    validate_digest(&resolution.distribution_sha256, "distribution_sha256")?;
    validate_resolved_artifact(
        &resolution.manifest,
        &format!("{expected_base_path}/manifest"),
        MAX_MANIFEST_BYTES,
        "manifest",
    )?;
    validate_resolved_artifact(
        &resolution.entrypoint,
        &format!("{expected_base_path}/entrypoint"),
        MAX_ENTRYPOINT_BYTES,
        "entrypoint",
    )
}

fn validate_resolved_artifact(
    artifact: &ResolvedArtifact,
    expected_download_path: &str,
    maximum_size: u64,
    label: &str,
) -> Result<(), String> {
    validate_relative_path(Path::new(&artifact.path), &format!("{label} path"))?;
    validate_digest(&artifact.sha256, &format!("{label} sha256"))?;
    if artifact.download_path != expected_download_path {
        return Err(format!("{label} download path is not canonical"));
    }
    if artifact.size_bytes == 0 || artifact.size_bytes > maximum_size {
        return Err(format!("{label} size is outside the Installer limit"));
    }
    Ok(())
}

fn validate_downloaded_manifest(
    manifest: &OperatorPackageManifest,
    package_id: &str,
    package_version: &str,
) -> Result<(), String> {
    if manifest.schema_version != OPERATOR_PACKAGE_SCHEMA_VERSION
        || manifest.sdk_api_version != OPERATOR_SDK_API_VERSION
        || manifest.execution_abi != OPERATOR_JSON_ABI_SCHEMA_VERSION
        || manifest.package_id != package_id
        || manifest.package_version != package_version
    {
        return Err("downloaded operator package manifest identity mismatch".to_string());
    }
    let entrypoint = expand_platform_library_template(&manifest.entrypoint);
    validate_relative_path(Path::new(&entrypoint), "operator entrypoint")
}

fn download_artifact(
    agent: &ureq::Agent,
    central_url: &str,
    bearer_token: Option<&str>,
    artifact: &ResolvedArtifact,
    maximum_size: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    let bytes = get_bytes(
        agent,
        &format!("{central_url}{}", artifact.download_path),
        bearer_token,
        maximum_size,
        label,
    )?;
    if bytes.len() as u64 != artifact.size_bytes {
        return Err(format!("{label} size mismatch"));
    }
    if sha256(&bytes) != artifact.sha256 {
        return Err(format!("{label} digest mismatch"));
    }
    Ok(bytes)
}

fn get_bytes(
    agent: &ureq::Agent,
    url: &str,
    bearer_token: Option<&str>,
    maximum_size: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    let mut request = agent.get(url);
    if let Some(token) = bearer_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request
        .call()
        .map_err(|error| format!("failed to fetch {label}: {error}"))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(maximum_size.saturating_add(1))
        .read_to_vec()
        .map_err(|error| format!("failed to read {label}: {error}"))?;
    if bytes.len() as u64 > maximum_size {
        return Err(format!("{label} exceeds the Installer size limit"));
    }
    Ok(bytes)
}

fn validate_central_url(value: &str) -> Result<String, String> {
    let value = value.trim_end_matches('/');
    let authority = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| "central URL must use http or https".to_string())?;
    if authority.is_empty()
        || authority.contains(['?', '#', '\\', '/'])
        || authority
            .split('/')
            .next()
            .is_some_and(|host| host.contains('@'))
    {
        return Err("central URL is not a safe origin".to_string());
    }
    Ok(value.to_string())
}

fn validate_bearer_token(token: Option<&str>) -> Result<(), String> {
    if token.is_some_and(|value| value.is_empty() || value.contains(['\r', '\n'])) {
        return Err("Orchestra bearer token is malformed".to_string());
    }
    Ok(())
}

fn validate_portable_token(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!("{label} is not a safe portable token"));
    }
    Ok(())
}

fn validate_relative_path(path: &Path, label: &str) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.to_string_lossy().contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("{label} must be a strict portable relative path"));
    }
    Ok(())
}

fn validate_digest(digest: &str, label: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} must be lowercase SHA-256 hex"));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct DownloadedPackage {
    root: PathBuf,
}

impl DownloadedPackage {
    fn prepare(manifest: &OperatorPackageManifest, manifest_bytes: &[u8]) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_nanos();
        let root = std::env::temp_dir().join(download_staging_name(nonce));
        fs::create_dir(&root)
            .map_err(|error| format!("failed to create operator download staging: {error}"))?;
        let result = (|| {
            write_new(&root.join(OPERATOR_PACKAGE_MANIFEST_FILE), manifest_bytes)?;
            let entrypoint = root.join(expand_platform_library_template(&manifest.entrypoint));
            validate_relative_path(
                entrypoint
                    .strip_prefix(&root)
                    .map_err(|_| "operator entrypoint escaped download staging".to_string())?,
                "operator entrypoint",
            )?;
            fs::create_dir_all(
                entrypoint
                    .parent()
                    .ok_or_else(|| "operator entrypoint has no parent".to_string())?,
            )
            .map_err(|error| format!("failed to create operator entrypoint directory: {error}"))
        })();
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&root);
            return Err(error);
        }
        Ok(Self { root })
    }
}

fn download_staging_name(nonce: u128) -> String {
    // Wall-clock resolution is not a uniqueness guarantee for concurrent downloads.
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!(
        "kyuubiki-operator-fetch-{}-{nonce}-{sequence}",
        std::process::id()
    )
}

#[cfg(test)]
mod streaming_tests {
    use super::*;

    #[test]
    fn concurrent_downloads_with_identical_clock_values_have_distinct_staging() {
        let names = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        (0..64)
                            .map(|_| download_staging_name(42))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().unwrap())
                .collect::<std::collections::BTreeSet<_>>()
        });
        assert_eq!(names.len(), 512);
    }

    #[test]
    fn streamed_entrypoint_requires_exact_length_and_digest() {
        let bytes = vec![0x5a; 2 * 1024 * 1024 + 17];
        let mut output = Vec::new();
        copy_verified_entrypoint(
            &mut bytes.as_slice(),
            &mut output,
            bytes.len() as u64,
            &sha256(&bytes),
        )
        .unwrap();
        assert_eq!(output, bytes);
        for (size, digest, expected) in [
            (bytes.len() as u64 + 1, sha256(&bytes), "size mismatch"),
            (bytes.len() as u64 - 1, sha256(&bytes), "size mismatch"),
            (bytes.len() as u64, "0".repeat(64), "digest mismatch"),
        ] {
            let mut output = Vec::new();
            let error = copy_verified_entrypoint(&mut bytes.as_slice(), &mut output, size, &digest)
                .unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert!(output.len() as u64 <= size);
        }
    }

    #[test]
    fn streamed_entrypoint_rejects_disk_failure() {
        struct FullDisk;
        impl Write for FullDisk {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full fixture"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let error = copy_verified_entrypoint(
            &mut b"bytes".as_slice(),
            &mut FullDisk,
            5,
            &sha256(b"bytes"),
        )
        .unwrap_err();
        assert!(error.contains("failed to write"));
    }
}

impl Drop for DownloadedPackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", path.display()))
}
