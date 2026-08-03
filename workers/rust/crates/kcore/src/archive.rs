use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::export::render_issues;
use crate::model::{MEDIA_TYPE, Manifest};

pub const MIMETYPE_ENTRY: &str = "mimetype";
pub const MANIFEST_ENTRY: &str = "manifest.json";

#[derive(Clone, Copy, Debug)]
pub struct ReaderLimits {
    pub max_entries: usize,
    pub max_manifest_bytes: u64,
    pub max_artifact_bytes: u64,
    pub max_total_bytes: u64,
}

impl Default for ReaderLimits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_manifest_bytes: 16 * 1024 * 1024,
            max_artifact_bytes: 1024 * 1024 * 1024 * 1024,
            max_total_bytes: 4 * 1024 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InspectionReport {
    pub ok: bool,
    pub schema_version: String,
    pub core_id: String,
    pub title: String,
    pub kind: String,
    pub core_digest_sha256: String,
    pub artifact_count: usize,
    pub contract_count: usize,
    pub entrypoints: Vec<String>,
    pub payload_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct VerificationReport {
    #[serde(flatten)]
    pub inspection: InspectionReport,
    pub object_count: usize,
    pub verified_payload_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ExtractionReport {
    pub ok: bool,
    pub output: String,
    pub core_id: String,
    pub object_count: usize,
    pub extracted_bytes: u64,
}

pub(crate) fn write(
    output: &Path,
    manifest: &Manifest,
    objects: &BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let result = (|| {
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(MIMETYPE_ENTRY, stored_options())
            .map_err(|error| format!("failed to add kcore mimetype: {error}"))?;
        writer
            .write_all(MEDIA_TYPE.as_bytes())
            .map_err(|error| format!("failed to write kcore mimetype: {error}"))?;
        writer
            .start_file(MANIFEST_ENTRY, deflated_options())
            .map_err(|error| format!("failed to add kcore manifest: {error}"))?;
        writer
            .write_all(
                serde_json::to_string_pretty(manifest)
                    .map_err(|error| format!("failed to serialize kcore manifest: {error}"))?
                    .as_bytes(),
            )
            .map_err(|error| format!("failed to write kcore manifest: {error}"))?;

        for (expected_digest, source) in objects {
            let object_path = Manifest::object_path(expected_digest);
            writer
                .start_file(&object_path, deflated_options())
                .map_err(|error| format!("failed to add {object_path}: {error}"))?;
            let (length, digest) = copy_and_hash(source, &mut writer)?;
            if digest != *expected_digest {
                return Err(format!(
                    "artifact changed while exporting: {} (wrote {length} bytes)",
                    source.display()
                ));
            }
        }
        writer
            .finish()
            .map_err(|error| format!("failed to finalize {}: {error}", output.display()))?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

pub fn inspect(path: &Path) -> Result<InspectionReport, String> {
    let manifest = read_manifest(path, ReaderLimits::default())?;
    Ok(inspection(&manifest))
}

pub fn verify(path: &Path) -> Result<VerificationReport, String> {
    verify_with_limits(path, ReaderLimits::default())
}

pub fn verify_with_limits(path: &Path, limits: ReaderLimits) -> Result<VerificationReport, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("invalid kcore container {}: {error}", path.display()))?;
    validate_entry_table(&mut archive, limits)?;
    let manifest = read_manifest_from_archive(&mut archive, limits)?;
    let expected = expected_objects(&manifest);
    let mut total = 0_u64;
    for (object_path, (expected_length, expected_digest)) in &expected {
        let mut entry = archive
            .by_name(object_path)
            .map_err(|_| format!("kcore object is missing: {object_path}"))?;
        if entry.size() != *expected_length {
            return Err(format!("kcore object size mismatch: {object_path}"));
        }
        if entry.size() > limits.max_artifact_bytes {
            return Err(format!("kcore object exceeds reader limit: {object_path}"));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| "kcore total size overflow".to_string())?;
        if total > limits.max_total_bytes {
            return Err("kcore payload exceeds total reader limit".to_string());
        }
        let actual = hash_reader(&mut entry, *expected_length)?;
        if actual != *expected_digest {
            return Err(format!("kcore object digest mismatch: {object_path}"));
        }
    }
    Ok(VerificationReport {
        inspection: inspection(&manifest),
        object_count: expected.len(),
        verified_payload_bytes: total,
    })
}

pub fn extract(path: &Path, output: &Path) -> Result<ExtractionReport, String> {
    let verification = verify(path)?;
    if output.exists() {
        return Err(format!(
            "refusing to overwrite extraction output: {}",
            output.display()
        ));
    }
    fs::create_dir_all(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let result = extract_verified(path, output, &verification);
    if result.is_err() {
        let _ = fs::remove_dir_all(output);
    }
    result
}

fn extract_verified(
    path: &Path,
    output: &Path,
    verification: &VerificationReport,
) -> Result<ExtractionReport, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("invalid kcore container: {error}"))?;
    let limits = ReaderLimits::default();
    validate_entry_table(&mut archive, limits)?;
    let manifest = read_manifest_from_archive(&mut archive, limits)?;
    if manifest.integrity.core_digest_sha256 != verification.inspection.core_digest_sha256 {
        return Err("kcore changed between verification and extraction".to_string());
    }
    let expected = expected_objects(&manifest);
    let mut extracted_payload_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to read kcore entry {index}: {error}"))?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| format!("unsafe kcore entry path: {}", entry.name()))?
            .to_path_buf();
        if entry.is_dir() {
            return Err(format!("unexpected directory entry: {}", entry.name()));
        }
        let destination = output.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
        if let Some((expected_length, expected_digest)) = expected.get(entry.name()) {
            let (length, digest) = copy_reader_and_hash(&mut entry, &mut target, *expected_length)?;
            if length != *expected_length || digest != *expected_digest {
                return Err(format!(
                    "kcore object changed while extracting: {}",
                    entry.name()
                ));
            }
            extracted_payload_bytes = extracted_payload_bytes
                .checked_add(length)
                .ok_or_else(|| "kcore extraction size overflow".to_string())?;
        } else {
            std::io::copy(&mut entry, &mut target)
                .map_err(|error| format!("failed to extract {}: {error}", entry.name()))?;
        }
    }
    Ok(ExtractionReport {
        ok: true,
        output: output.to_string_lossy().into_owned(),
        core_id: verification.inspection.core_id.clone(),
        object_count: verification.object_count,
        extracted_bytes: extracted_payload_bytes,
    })
}

fn read_manifest(path: &Path, limits: ReaderLimits) -> Result<Manifest, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("invalid kcore container {}: {error}", path.display()))?;
    validate_entry_table(&mut archive, limits)?;
    read_manifest_from_archive(&mut archive, limits)
}

fn read_manifest_from_archive(
    archive: &mut ZipArchive<File>,
    limits: ReaderLimits,
) -> Result<Manifest, String> {
    let mimetype = read_limited(archive, MIMETYPE_ENTRY, 256)?;
    if mimetype != MEDIA_TYPE.as_bytes() {
        return Err("kcore mimetype marker is missing or invalid".to_string());
    }
    let bytes = read_limited(archive, MANIFEST_ENTRY, limits.max_manifest_bytes)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid kcore manifest: {error}"))?;
    manifest.validate().map_err(render_issues)?;
    Ok(manifest)
}

fn validate_entry_table(
    archive: &mut ZipArchive<File>,
    limits: ReaderLimits,
) -> Result<(), String> {
    if archive.len() > limits.max_entries {
        return Err(format!("kcore entry count exceeds {}", limits.max_entries));
    }
    let mut names = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to inspect kcore entry {index}: {error}"))?;
        if entry.enclosed_name().is_none() || entry.is_dir() {
            return Err(format!(
                "unsafe or unsupported kcore entry: {}",
                entry.name()
            ));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!(
                "symbolic links are forbidden in kcore: {}",
                entry.name()
            ));
        }
        if !names.insert(entry.name().to_string()) {
            return Err(format!("duplicate kcore entry: {}", entry.name()));
        }
    }
    let first = archive
        .by_index(0)
        .map_err(|error| format!("failed to inspect first kcore entry: {error}"))?;
    if first.name() != MIMETYPE_ENTRY {
        return Err("mimetype must be the first kcore entry".to_string());
    }
    if first.compression() != CompressionMethod::Stored {
        return Err("kcore mimetype entry must be stored without compression".to_string());
    }
    drop(first);
    if !names.contains(MIMETYPE_ENTRY) || !names.contains(MANIFEST_ENTRY) {
        return Err("kcore must contain mimetype and manifest.json".to_string());
    }
    let manifest = read_manifest_entry_only(archive, limits.max_manifest_bytes)?;
    let expected = expected_objects(&manifest);
    for name in &names {
        if name != MIMETYPE_ENTRY && name != MANIFEST_ENTRY && !expected.contains_key(name) {
            return Err(format!("unreferenced kcore entry: {name}"));
        }
    }
    for name in expected.keys() {
        if !names.contains(name) {
            return Err(format!("kcore object is missing: {name}"));
        }
    }
    Ok(())
}

fn read_manifest_entry_only(
    archive: &mut ZipArchive<File>,
    limit: u64,
) -> Result<Manifest, String> {
    let bytes = read_limited(archive, MANIFEST_ENTRY, limit)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid kcore manifest: {error}"))
}

fn expected_objects(manifest: &Manifest) -> HashMap<String, (u64, String)> {
    let mut expected = HashMap::new();
    for artifact in &manifest.artifacts {
        expected
            .entry(artifact.object_path.clone())
            .or_insert((artifact.byte_length, artifact.sha256.clone()));
    }
    expected
}

fn inspection(manifest: &Manifest) -> InspectionReport {
    InspectionReport {
        ok: true,
        schema_version: manifest.schema_version.clone(),
        core_id: manifest.core_id.clone(),
        title: manifest.title.clone(),
        kind: manifest.kind.clone(),
        core_digest_sha256: manifest.integrity.core_digest_sha256.clone(),
        artifact_count: manifest.artifacts.len(),
        contract_count: manifest.contracts.len(),
        entrypoints: manifest.entrypoints.clone(),
        payload_bytes: manifest
            .artifacts
            .iter()
            .fold(0_u64, |total, item| total.saturating_add(item.byte_length)),
    }
}

fn read_limited(archive: &mut ZipArchive<File>, name: &str, limit: u64) -> Result<Vec<u8>, String> {
    let entry = archive
        .by_name(name)
        .map_err(|_| format!("kcore entry is missing: {name}"))?;
    if entry.size() > limit {
        return Err(format!("kcore entry exceeds reader limit: {name}"));
    }
    let capacity = usize::try_from(entry.size())
        .map_err(|_| format!("kcore entry is too large for this platform: {name}"))?;
    let mut bytes = Vec::with_capacity(capacity);
    entry
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read kcore entry {name}: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("kcore entry exceeds reader limit: {name}"));
    }
    Ok(bytes)
}

fn copy_and_hash(path: &Path, target: &mut impl Write) -> Result<(u64, String), String> {
    let mut input =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        target
            .write_all(&buffer[..read])
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        digest.update(&buffer[..read]);
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| "artifact length overflow".to_string())?;
    }
    Ok((length, format!("{:x}", digest.finalize())))
}

fn copy_reader_and_hash(
    reader: &mut impl Read,
    target: &mut impl Write,
    max_length: u64,
) -> Result<(u64, String), String> {
    let mut digest = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("failed to read kcore object: {error}"))?;
        if read == 0 {
            break;
        }
        let next_length = length
            .checked_add(read as u64)
            .ok_or_else(|| "kcore object size overflow".to_string())?;
        if next_length > max_length {
            return Err("kcore object exceeds its declared byte length".to_string());
        }
        target
            .write_all(&buffer[..read])
            .map_err(|error| format!("failed to extract kcore object: {error}"))?;
        digest.update(&buffer[..read]);
        length = next_length;
    }
    Ok((length, format!("{:x}", digest.finalize())))
}

fn hash_reader(reader: &mut impl Read, max_length: u64) -> Result<String, String> {
    let mut digest = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("failed to read kcore object: {error}"))?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| "kcore object size overflow".to_string())?;
        if length > max_length {
            return Err("kcore object exceeds its declared byte length".to_string());
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn stored_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644)
}

fn deflated_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644)
}
