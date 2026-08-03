use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::archive;
use crate::model::{
    Artifact, ExportSpec, FORMAT_SCHEMA_VERSION, FORMAT_VERSION, Integrity, Manifest,
    validate_export_spec,
};

const MAX_EXPORT_SPEC_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct ExportReport {
    pub ok: bool,
    pub path: String,
    pub core_id: String,
    pub core_digest_sha256: String,
    pub artifact_count: usize,
    pub object_count: usize,
    pub payload_bytes: u64,
}

pub fn export(spec_path: &Path, output: &Path) -> Result<ExportReport, String> {
    let spec_path = existing_regular_file(spec_path, "export spec")?;
    let spec = read_spec(&spec_path)?;
    export_spec(
        spec,
        spec_path.parent().unwrap_or_else(|| Path::new(".")),
        output,
    )
}

pub fn export_spec(
    spec: ExportSpec,
    source_base: &Path,
    output: &Path,
) -> Result<ExportReport, String> {
    require_extension(output)?;
    if output.exists() {
        return Err(format!(
            "refusing to overwrite output: {}",
            output.display()
        ));
    }
    validate_export_spec(&spec).map_err(render_issues)?;
    let mut object_sources = BTreeMap::<String, PathBuf>::new();
    let mut artifacts = Vec::with_capacity(spec.artifacts.len());
    let mut payload_bytes = 0_u64;

    for source in spec.artifacts {
        let unresolved = if Path::new(&source.source).is_absolute() {
            PathBuf::from(&source.source)
        } else {
            source_base.join(&source.source)
        };
        let path = existing_regular_file(&unresolved, &format!("artifact {}", source.id))?;
        let (byte_length, sha256) = hash_file(&path)?;
        payload_bytes = payload_bytes
            .checked_add(byte_length)
            .ok_or_else(|| "kcore payload byte count overflow".to_string())?;
        object_sources.entry(sha256.clone()).or_insert(path);
        artifacts.push(Artifact {
            id: source.id,
            role: source.role,
            media_type: source.media_type,
            object_path: Manifest::object_path(&sha256),
            byte_length,
            sha256,
            name: source.name,
            schema_ref: source.schema_ref,
            encoding: source.encoding,
            shape: source.shape,
            unit: source.unit,
            metadata: source.metadata,
        });
    }

    artifacts.sort_by(|left, right| left.id.cmp(&right.id));
    let mut manifest = Manifest {
        schema_version: FORMAT_SCHEMA_VERSION.to_string(),
        format: "kcore".to_string(),
        format_version: FORMAT_VERSION,
        core_id: spec.core_id,
        title: spec.title,
        kind: spec.kind,
        producer: spec.producer,
        artifacts,
        contracts: spec.contracts,
        entrypoints: spec.entrypoints,
        integrity: Integrity {
            algorithm: "sha256".to_string(),
            core_digest_sha256: String::new(),
        },
        created_at: spec.created_at,
        provenance: spec.provenance,
        metadata: spec.metadata,
    };
    manifest
        .contracts
        .sort_by(|left, right| left.name.cmp(&right.name));
    manifest.entrypoints.sort();
    manifest.seal()?;
    manifest.validate().map_err(render_issues)?;
    archive::write(output, &manifest, &object_sources)?;

    Ok(ExportReport {
        ok: true,
        path: output.to_string_lossy().into_owned(),
        core_id: manifest.core_id,
        core_digest_sha256: manifest.integrity.core_digest_sha256,
        artifact_count: manifest.artifacts.len(),
        object_count: object_sources.len(),
        payload_bytes,
    })
}

fn read_spec(path: &Path) -> Result<ExportSpec, String> {
    let size = path
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
        .len();
    if size > MAX_EXPORT_SPEC_BYTES {
        return Err(format!(
            "export spec exceeds {} bytes: {}",
            MAX_EXPORT_SPEC_BYTES,
            path.display()
        ));
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("invalid kcore export spec {}: {error}", path.display()))
}

fn existing_regular_file(path: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must not be a symbolic link: {}",
            path.display()
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "{label} must be a regular file: {}",
            path.display()
        ));
    }
    path.canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
}

pub(crate) fn hash_file(path: &Path) -> Result<(u64, String), String> {
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
        digest.update(&buffer[..read]);
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| format!("file size overflow: {}", path.display()))?;
    }
    Ok((length, format!("{:x}", digest.finalize())))
}

fn require_extension(path: &Path) -> Result<(), String> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("kcore"))
    {
        Ok(())
    } else {
        Err("kcore output path must end with .kcore".to_string())
    }
}

pub(crate) fn render_issues(issues: Vec<String>) -> String {
    format!("invalid kcore contract: {}", issues.join("; "))
}
