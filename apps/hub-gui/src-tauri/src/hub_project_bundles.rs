use std::fs::File;
use std::io::{Read, Write};

use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

const PROJECT_MANIFEST: &str = "project.json";

fn read_project_bundle(path: &Path) -> Result<serde_json::Value, String> {
    let file = File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| format!("invalid project bundle: {error}"))?;
    let mut manifest = archive
        .by_name(PROJECT_MANIFEST)
        .map_err(|_| "project bundle is missing project.json".to_string())?;
    let mut text = String::new();
    manifest.read_to_string(&mut text).map_err(|error| format!("failed to read project.json: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid project.json: {error}"))
}

fn project_array_len(bundle: &serde_json::Value, key: &str) -> usize {
    bundle.get(key).and_then(serde_json::Value::as_array).map_or(0, Vec::len)
}

fn project_string(bundle: &serde_json::Value, path: &[&str]) -> Option<String> {
    path.iter().try_fold(bundle, |value, key| value.get(*key))?.as_str().map(ToOwned::to_owned)
}

fn project_summary(bundle: &serde_json::Value) -> serde_json::Value {
    json!({
        "schema": bundle.get("project_schema_version").and_then(serde_json::Value::as_str),
        "layout": project_string(bundle, &["project_file_manifest", "layout_version"]),
        "project_id": project_string(bundle, &["project", "project_id"]),
        "project_name": project_string(bundle, &["project", "name"]),
        "model_count": project_array_len(bundle, "models"),
        "version_count": project_array_len(bundle, "model_versions"),
        "job_count": project_array_len(bundle, "jobs"),
        "result_count": project_array_len(bundle, "results"),
        "automation_preset_count": project_array_len(bundle, "automation_presets"),
        "asset_count": project_array_len(bundle, "asset_catalog"),
        "asset_reference_count": project_array_len(bundle, "asset_references"),
        "active_model_id": bundle.get("active_model_id"),
        "active_version_id": bundle.get("active_version_id"),
        "has_workspace_snapshot": !bundle.get("workspace_snapshot").unwrap_or(&serde_json::Value::Null).is_null(),
    })
}

fn validate_project_bundle(bundle: &serde_json::Value) -> serde_json::Value {
    let mut issues = Vec::new();
    if !matches!(bundle.get("project_schema_version").and_then(serde_json::Value::as_str), Some("kyuubiki.project/v1" | "kyuubiki.project/v2")) {
        issues.push("unsupported project_schema_version".to_string());
    }
    if project_string(bundle, &["project", "project_id"]).is_none() { issues.push("project.project_id is required".to_string()); }
    if !bundle.get("models").is_some_and(serde_json::Value::is_array) { issues.push("models must be an array".to_string()); }
    if !bundle.get("model_versions").is_some_and(serde_json::Value::is_array) { issues.push("model_versions must be an array".to_string()); }
    json!({ "ok": issues.is_empty(), "issue_count": issues.len(), "issues": issues, "summary": project_summary(bundle) })
}

fn zip_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
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
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("failed to resolve bundle entry {}: {error}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            writer
                .add_directory(format!("{relative}/"), zip_options())
                .map_err(|error| format!("failed to add directory {relative}: {error}"))?;
            archive_directory(writer, root, &path)?;
            continue;
        }
        if !path.is_file() {
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

fn pack_project_directory(input: &Path, output: &Path) -> Result<(), String> {
    let manifest_path = input.join(PROJECT_MANIFEST);
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    serde_json::from_str::<serde_json::Value>(&manifest_text)
        .map_err(|error| format!("invalid project.json: {error}"))?;

    let file = File::create(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let mut writer = ZipWriter::new(file);
    archive_directory(&mut writer, input, input)?;
    writer
        .finish()
        .map_err(|error| format!("failed to finalize project bundle: {error}"))?;
    Ok(())
}

fn rewrite_normalized_bundle(input: &Path, output: &Path) -> Result<(), String> {
    if input == output {
        return Ok(());
    }

    let file = File::open(input).map_err(|error| format!("failed to open {}: {error}", input.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| format!("invalid project bundle: {error}"))?;
    let output_file = File::create(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
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
            let mut text = String::new();
            entry
                .read_to_string(&mut text)
                .map_err(|error| format!("failed to read project.json: {error}"))?;
            let manifest: serde_json::Value = serde_json::from_str(&text)
                .map_err(|error| format!("invalid project.json: {error}"))?;
            writer
                .write_all(
                    serde_json::to_string_pretty(&manifest)
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
    Ok(())
}

fn unpack_project_bundle(input: &Path, output: &Path) -> Result<(), String> {
    let file = File::open(input).map_err(|error| format!("failed to open {}: {error}", input.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| format!("invalid project bundle: {error}"))?;
    fs::create_dir_all(output).map_err(|error| format!("failed to create {}: {error}", output.display()))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to read bundle entry: {error}"))?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| format!("unsafe bundle entry: {}", entry.name()))?;
        let destination = output.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
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
    Ok(())
}

fn run_project_cli(command: &str, input_path: &str) -> Result<String, String> {
    let input = normalize_existing_bundle_path(input_path, "project bundle path")?;
    let bundle = read_project_bundle(&input)?;
    match command {
        "inspect" => serde_json::to_string_pretty(&project_summary(&bundle)).map_err(|error| error.to_string()),
        "validate" => serde_json::to_string_pretty(&validate_project_bundle(&bundle)).map_err(|error| error.to_string()),
        _ => Err(format!("unsupported native project action: {command}")),
    }
}

fn run_project_cli_with_output(command: &str, input_path: &str, output_path: &str) -> Result<String, String> {
    let output = normalize_output_path(output_path, "output path")?;
    match command {
        "normalize" => { let input = normalize_existing_bundle_path(input_path, "project bundle path")?; read_project_bundle(&input)?; rewrite_normalized_bundle(&input, &output)?; }
        "pack" => { let input = normalize_existing_directory_path(input_path, "project directory path")?; pack_project_directory(&input, &output)?; }
        "unpack" => { let input = normalize_existing_bundle_path(input_path, "project bundle path")?; unpack_project_bundle(&input, &output)?; }
        _ => return Err(format!("unsupported native project action: {command}")),
    }
    Ok(format!("native project {command} completed -> {}", output.display()))
}

fn run_project_cli_compare(_command: &str, left_path: &str, right_path: &str) -> Result<String, String> {
    let left = read_project_bundle(&normalize_existing_bundle_path(left_path, "left project bundle path")?)?;
    let right = read_project_bundle(&normalize_existing_bundle_path(right_path, "right project bundle path")?)?;
    serde_json::to_string_pretty(&json!({ "left": project_summary(&left), "right": project_summary(&right), "changed_project_identity": project_string(&left, &["project", "project_id"]) != project_string(&right, &["project", "project_id"]), "active_model_changed": left.get("active_model_id") != right.get("active_model_id"), "active_version_changed": left.get("active_version_id") != right.get("active_version_id") })).map_err(|error| error.to_string())
}
