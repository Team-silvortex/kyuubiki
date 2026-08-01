use crate::model::{default_file_manifest, normalize};
use crate::paths::{PROJECT_EXTENSION, existing_input, has_extension};
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub(crate) const PROJECT_MANIFEST: &str = "project.json";

fn zip_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

pub(crate) fn write_json_entry(
    writer: &mut ZipWriter<File>,
    path: &str,
    value: &Value,
) -> Result<(), String> {
    writer
        .start_file(path, zip_options())
        .map_err(|error| format!("failed to add {path}: {error}"))?;
    writer
        .write_all(
            serde_json::to_string_pretty(value)
                .map_err(|error| format!("failed to serialize {path}: {error}"))?
                .as_bytes(),
        )
        .map_err(|error| format!("failed to write {path}: {error}"))
}

pub(crate) fn create_archive(path: &Path, manifest: &Value) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    let result = (|| {
        let mut writer = ZipWriter::new(file);
        let file_manifest = manifest
            .get("project_file_manifest")
            .cloned()
            .unwrap_or_else(default_file_manifest);
        let project = manifest.get("project").cloned().unwrap_or(Value::Null);
        let engine_manifest_path = manifest_path(
            &file_manifest,
            "engine_manifest_path",
            ".kyuubiki/project.json",
        );
        let project_record_path = manifest_path(
            &file_manifest,
            "project_record_path",
            "Assets/project/project.json",
        );
        let workspace_settings_path = manifest_path(
            &file_manifest,
            "workspace_settings_path",
            "ProjectSettings/workspace.json",
        );
        let workspace_snapshot_path = manifest_path(
            &file_manifest,
            "workspace_snapshot_path",
            "Workspace/current-model.json",
        );
        let automation_presets_path = manifest_path(
            &file_manifest,
            "automation_presets_path",
            "ProjectSettings/automation-presets.json",
        );
        let asset_catalog_path = manifest_path(
            &file_manifest,
            "asset_catalog_path",
            "ProjectSettings/asset-catalog.json",
        );
        let asset_references_path = manifest_path(
            &file_manifest,
            "asset_references_path",
            "ProjectSettings/asset-references.json",
        );
        let job_directory = manifest_path(&file_manifest, "job_directory", "Analysis/jobs");
        let result_directory =
            manifest_path(&file_manifest, "result_directory", "Analysis/results");
        let workspace_settings = json!({
            "active_model_id": manifest.get("active_model_id").unwrap_or(&Value::Null),
            "active_version_id": manifest.get("active_version_id").unwrap_or(&Value::Null),
            "exported_at": manifest.get("exported_at").unwrap_or(&Value::Null),
            "project_schema_version": manifest.get("project_schema_version").unwrap_or(&Value::Null),
            "layout_version": file_manifest.get("layout_version").unwrap_or(&Value::Null),
        });
        write_json_entry(&mut writer, PROJECT_MANIFEST, manifest)?;
        write_json_entry(&mut writer, &engine_manifest_path, &file_manifest)?;
        write_json_entry(&mut writer, "project/project.json", &project)?;
        write_json_entry(&mut writer, &project_record_path, &project)?;
        write_json_entry(&mut writer, &workspace_settings_path, &workspace_settings)?;
        write_json_entry(
            &mut writer,
            &automation_presets_path,
            manifest.get("automation_presets").unwrap_or(&json!([])),
        )?;
        write_json_entry(
            &mut writer,
            &asset_catalog_path,
            manifest.get("asset_catalog").unwrap_or(&json!([])),
        )?;
        write_json_entry(
            &mut writer,
            &asset_references_path,
            manifest.get("asset_references").unwrap_or(&json!([])),
        )?;
        write_json_entry(
            &mut writer,
            "jobs/jobs.json",
            manifest.get("jobs").unwrap_or(&json!([])),
        )?;
        write_json_entry(
            &mut writer,
            &format!("{job_directory}/index.json"),
            manifest.get("jobs").unwrap_or(&json!([])),
        )?;
        write_json_entry(
            &mut writer,
            "results/results.json",
            manifest.get("results").unwrap_or(&json!([])),
        )?;
        write_json_entry(
            &mut writer,
            &format!("{result_directory}/index.json"),
            manifest.get("results").unwrap_or(&json!([])),
        )?;
        if let Some(snapshot) = manifest
            .get("workspace_snapshot")
            .filter(|value| !value.is_null())
        {
            write_json_entry(&mut writer, &workspace_snapshot_path, snapshot)?;
        }
        write_catalog_records(&mut writer, manifest, "models", "model", "model_id")?;
        write_catalog_records(
            &mut writer,
            manifest,
            "model_versions",
            "model_version",
            "version_id",
        )?;
        write_catalog_records(&mut writer, manifest, "jobs", "job", "job_id")?;
        write_catalog_records(&mut writer, manifest, "results", "result", "job_id")?;
        writer
            .start_file("README.txt", zip_options())
            .map_err(|error| format!("failed to add README.txt: {error}"))?;
        writer
            .write_all(
                b"Kyuubiki project bundle\n\nSchema: kyuubiki.project/v2\nLayout: kyuubiki.project-layout/v1\nManifest: project.json\n",
            )
            .map_err(|error| format!("failed to write README.txt: {error}"))?;
        writer
            .finish()
            .map_err(|error| format!("failed to finalize project bundle: {error}"))?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

fn manifest_path(manifest: &Value, key: &str, fallback: &str) -> String {
    manifest
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn write_catalog_records(
    writer: &mut ZipWriter<File>,
    manifest: &Value,
    records_key: &str,
    kind: &str,
    id_key: &str,
) -> Result<(), String> {
    let records = manifest
        .get(records_key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for record in records {
        let Some(id) = record.get(id_key).and_then(Value::as_str) else {
            continue;
        };
        let Some(entry) = catalog_entry(manifest, kind, id) else {
            continue;
        };
        let Some(path) = entry.get("path").and_then(Value::as_str) else {
            continue;
        };
        write_json_entry(writer, path, record)?;
        write_json_entry(writer, &format!("{path}.meta"), entry)?;
    }
    Ok(())
}

fn catalog_entry<'a>(manifest: &'a Value, kind: &str, source_id: &str) -> Option<&'a Value> {
    manifest
        .get("asset_catalog")
        .and_then(Value::as_array)?
        .iter()
        .find(|entry| {
            entry.get("kind").and_then(Value::as_str) == Some(kind)
                && entry.get("source_id").and_then(Value::as_str) == Some(source_id)
        })
}

pub(crate) fn read_input(value: &str, label: &str) -> Result<Value, String> {
    let path = existing_input(value, label)?;
    let raw = if path.is_dir() {
        read_json_file(&path.join(PROJECT_MANIFEST))?
    } else if has_extension(&path, PROJECT_EXTENSION) {
        read_archive_manifest(&path)?
    } else {
        read_json_file(&path)?
    };
    normalize(raw)
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn read_archive_manifest(path: &Path) -> Result<Value, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("invalid project bundle: {error}"))?;
    let mut entry = archive
        .by_name(PROJECT_MANIFEST)
        .map_err(|_| "project bundle is missing project.json".to_string())?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|error| format!("failed to read project.json: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid project.json: {error}"))
}

pub(crate) fn write_normalized(input: &str, output: &Path) -> Result<(), String> {
    let bundle = read_input(input, "project input")?;
    if has_extension(output, PROJECT_EXTENSION) {
        let input_path = existing_input(input, "project input")?;
        if input_path.is_file() && has_extension(&input_path, PROJECT_EXTENSION) {
            rewrite_archive(&input_path, output, &bundle)
        } else if input_path.is_dir() {
            pack_directory(&input_path, output)
        } else {
            create_archive(output, &bundle)
        }
    } else {
        fs::write(
            output,
            serde_json::to_string_pretty(&bundle).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("failed to write {}: {error}", output.display()))
    }
}

pub(crate) fn pack_directory(input: &Path, output: &Path) -> Result<(), String> {
    let manifest = read_json_file(&input.join(PROJECT_MANIFEST))?;
    normalize(manifest)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let result = (|| {
        let mut writer = ZipWriter::new(file);
        archive_directory(&mut writer, input, input)?;
        writer
            .finish()
            .map_err(|error| format!("failed to finalize project bundle: {error}"))?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

fn archive_directory(
    writer: &mut ZipWriter<File>,
    root: &Path,
    directory: &Path,
) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to list {}: {error}", directory.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "refusing to archive symbolic link: {}",
                path.display()
            ));
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("failed to resolve bundle entry {}: {error}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        if file_type.is_dir() {
            writer
                .add_directory(format!("{relative}/"), zip_options())
                .map_err(|error| format!("failed to add directory {relative}: {error}"))?;
            archive_directory(writer, root, &path)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(format!("unsupported project entry: {}", path.display()));
        }
        writer
            .start_file(&relative, zip_options())
            .map_err(|error| format!("failed to add {relative}: {error}"))?;
        let mut input = File::open(&path)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
        std::io::copy(&mut input, writer)
            .map_err(|error| format!("failed to archive {relative}: {error}"))?;
    }
    Ok(())
}

fn rewrite_archive(input: &Path, output: &Path, manifest: &Value) -> Result<(), String> {
    let input_file = File::open(input)
        .map_err(|error| format!("failed to open {}: {error}", input.display()))?;
    let mut archive =
        ZipArchive::new(input_file).map_err(|error| format!("invalid project bundle: {error}"))?;
    let output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let result = (|| {
        let mut writer = ZipWriter::new(output_file);
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| format!("failed to read bundle entry: {error}"))?;
            let name = entry
                .enclosed_name()
                .ok_or_else(|| format!("unsafe bundle entry: {}", entry.name()))?
                .to_string_lossy()
                .replace('\\', "/");
            if entry.is_dir() {
                writer
                    .add_directory(name, zip_options())
                    .map_err(|error| format!("failed to write bundle directory: {error}"))?;
                continue;
            }
            writer
                .start_file(&name, zip_options())
                .map_err(|error| format!("failed to write bundle entry {name}: {error}"))?;
            if name == PROJECT_MANIFEST {
                writer
                    .write_all(
                        serde_json::to_string_pretty(manifest)
                            .map_err(|error| error.to_string())?
                            .as_bytes(),
                    )
                    .map_err(|error| format!("failed to write project.json: {error}"))?;
            } else {
                std::io::copy(&mut entry, &mut writer)
                    .map_err(|error| format!("failed to copy bundle entry {name}: {error}"))?;
            }
        }
        writer
            .finish()
            .map_err(|error| format!("failed to finalize project bundle: {error}"))?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

pub(crate) fn unpack_archive(input: &Path, output: &Path) -> Result<(), String> {
    if output.exists() {
        return Err(format!(
            "refusing to overwrite existing output directory: {}",
            output.display()
        ));
    }
    let file = File::open(input)
        .map_err(|error| format!("failed to open {}: {error}", input.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("invalid project bundle: {error}"))?;
    fs::create_dir_all(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let result = (|| {
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| format!("failed to read bundle entry: {error}"))?;
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| format!("unsafe bundle entry: {}", entry.name()))?;
            let destination = output.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&destination).map_err(|error| {
                    format!("failed to create {}: {error}", destination.display())
                })?;
                continue;
            }
            let parent = destination
                .parent()
                .ok_or_else(|| format!("bundle entry has no parent: {}", destination.display()))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            let mut output_file = File::create(&destination)
                .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
            std::io::copy(&mut entry, &mut output_file)
                .map_err(|error| format!("failed to extract {}: {error}", destination.display()))?;
        }
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(output);
    }
    result
}
