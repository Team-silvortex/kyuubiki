use crate::migration::{ProjectMigrationLimits, ProjectMigrationReceipt};
use crate::migration_input::{self, EntryDigest};
use crate::paths;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

fn options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600)
}

pub(crate) fn build_candidate(
    source: &Path,
    canonical: &Path,
    output: &Path,
    manifest: &Value,
    receipt: &mut ProjectMigrationReceipt,
    limits: &ProjectMigrationLimits,
) -> Result<(), String> {
    let mut original = if paths::has_extension(source, "kyuubiki") {
        Some(migration_input::open_archive(source, limits)?)
    } else {
        None
    };
    let original_index = original
        .as_mut()
        .map(|archive| migration_input::inventory(archive, limits))
        .transpose()?
        .unwrap_or_default();
    let mut canonical = migration_input::open_archive(canonical, limits)?;
    let canonical_index = migration_input::inventory(&mut canonical, limits)?;
    let mut expected = BTreeMap::new();
    let mut open = OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open.mode(0o600);
    }
    let mut writer = ZipWriter::new(open.open(output).map_err(|error| error.to_string())?);
    let mut budget = limits.max_expanded_bytes;
    for (name, digest) in &canonical_index {
        if original_index.contains_key(name) {
            if name == "project.json" {
                copy_entry(
                    &mut canonical,
                    &mut writer,
                    name,
                    digest,
                    &mut expected,
                    &mut budget,
                )?;
                receipt.rewritten_entries.push(name.clone());
            } else if name == "README.txt" {
                copy_entry(
                    original.as_mut().unwrap(),
                    &mut writer,
                    name,
                    &original_index[name],
                    &mut expected,
                    &mut budget,
                )?;
                receipt.preserved_entries += 1;
            } else {
                let source_json = migration_input::read_json_entry(
                    original.as_mut().unwrap(),
                    name,
                    limits.max_manifest_bytes,
                )?;
                let canonical_json = migration_input::read_json_entry(
                    &mut canonical,
                    name,
                    limits.max_manifest_bytes,
                )?;
                let merged = reconcile(name, &source_json, &canonical_json, manifest)?;
                if merged == source_json {
                    copy_entry(
                        original.as_mut().unwrap(),
                        &mut writer,
                        name,
                        &original_index[name],
                        &mut expected,
                        &mut budget,
                    )?;
                    receipt.preserved_entries += 1;
                } else {
                    write_json(
                        &mut writer,
                        name,
                        &merged,
                        &mut expected,
                        &mut budget,
                        limits,
                    )?;
                    receipt.rewritten_entries.push(name.clone());
                }
            }
        } else {
            copy_entry(
                &mut canonical,
                &mut writer,
                name,
                digest,
                &mut expected,
                &mut budget,
            )?;
            receipt.added_entries.push(name.clone());
        }
    }
    for (name, digest) in &original_index {
        if canonical_index.contains_key(name) {
            continue;
        }
        copy_entry(
            original.as_mut().unwrap(),
            &mut writer,
            name,
            digest,
            &mut expected,
            &mut budget,
        )?;
        receipt.preserved_entries += 1;
    }
    if expected.contains_key(&receipt.record_path) {
        return Err("migration receipt path already exists".into());
    }
    write_json(
        &mut writer,
        &receipt.record_path,
        &serde_json::to_value(&*receipt).map_err(|error| error.to_string())?,
        &mut expected,
        &mut budget,
        limits,
    )?;
    if expected.len() > limits.max_entries {
        return Err("migrated archive exceeds configured entry limit".into());
    }
    writer
        .finish()
        .map_err(|error| format!("cannot finish migrated archive: {error}"))?;
    let mut readback = migration_input::open_archive(output, limits)?;
    if migration_input::inventory(&mut readback, limits)? != expected {
        return Err("migrated archive entry digests failed readback".into());
    }
    Ok(())
}

fn reconcile(
    name: &str,
    source: &Value,
    canonical: &Value,
    manifest: &Value,
) -> Result<Value, String> {
    if source == canonical {
        return Ok(source.clone());
    }
    let layout = &manifest["project_file_manifest"];
    if ["asset_catalog_path", "asset_references_path"]
        .iter()
        .any(|key| layout[*key] == name)
        && source.as_array().is_some_and(Vec::is_empty)
    {
        return Ok(canonical.clone());
    }
    if layout["workspace_settings_path"] == name || layout["engine_manifest_path"] == name {
        let mut merged = source
            .as_object()
            .cloned()
            .ok_or_else(|| format!("invalid metadata mirror: {name}"))?;
        for (key, value) in canonical.as_object().ok_or("invalid canonical metadata")? {
            if let Some(old) = merged.get(key) {
                let version_upgrade = key == "project_schema_version"
                    && old == "kyuubiki.project/v1"
                    && value == "kyuubiki.project/v2";
                if old != value && !version_upgrade {
                    return Err(format!(
                        "conflicting project metadata mirror: {name} ({key})"
                    ));
                }
            }
            merged.insert(key.clone(), value.clone());
        }
        return Ok(Value::Object(merged));
    }
    Err(format!(
        "conflicting project record mirror: {name}; resolve explicitly before migration"
    ))
}

fn reserve(bytes: u64, budget: &mut u64) -> Result<(), String> {
    *budget = budget
        .checked_sub(bytes)
        .ok_or("migrated archive exceeds configured expanded byte limit")?;
    Ok(())
}

fn copy_entry(
    input: &mut ZipArchive<File>,
    writer: &mut ZipWriter<File>,
    name: &str,
    digest: &EntryDigest,
    expected: &mut BTreeMap<String, EntryDigest>,
    budget: &mut u64,
) -> Result<(), String> {
    reserve(digest.bytes, budget)?;
    if digest.directory {
        writer
            .add_directory(name, options())
            .map_err(|error| error.to_string())?;
    } else {
        writer
            .start_file(name, options())
            .map_err(|error| error.to_string())?;
        let mut entry = input.by_name(name).map_err(|error| error.to_string())?;
        let copied = std::io::copy(&mut entry, writer)
            .map_err(|error| format!("cannot preserve {name}: {error}"))?;
        if copied != digest.bytes {
            return Err(format!("entry changed while copying: {name}"));
        }
    }
    expected.insert(name.into(), digest.clone());
    Ok(())
}

fn write_json(
    writer: &mut ZipWriter<File>,
    name: &str,
    value: &Value,
    expected: &mut BTreeMap<String, EntryDigest>,
    budget: &mut u64,
    limits: &ProjectMigrationLimits,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limits.max_manifest_bytes {
        return Err("migrated JSON exceeds configured manifest byte limit".into());
    }
    reserve(bytes.len() as u64, budget)?;
    writer
        .start_file(name, options())
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&bytes)
        .map_err(|error| error.to_string())?;
    expected.insert(
        name.into(),
        EntryDigest {
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
            directory: false,
        },
    );
    Ok(())
}
