use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

use chrono::{SecondsFormat, Utc};
use uuid::Uuid;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

const PROJECT_MANIFEST: &str = "project.json";
const PROJECT_EXTENSION: &str = "kyuubiki";

fn default_project_file_manifest() -> serde_json::Value {
    json!({
        "layout_version": "kyuubiki.project-layout/v1",
        "engine_manifest_path": ".kyuubiki/project.json",
        "root_manifest_path": PROJECT_MANIFEST,
        "project_record_path": "Assets/project/project.json",
        "workspace_settings_path": "ProjectSettings/workspace.json",
        "workspace_snapshot_path": "Workspace/current-model.json",
        "automation_presets_path": "ProjectSettings/automation-presets.json",
        "asset_catalog_path": "ProjectSettings/asset-catalog.json",
        "asset_references_path": "ProjectSettings/asset-references.json",
        "model_directory": "Assets/models",
        "version_directory": "Assets/versions",
        "job_directory": "Analysis/jobs",
        "result_directory": "Analysis/results",
    })
}

fn default_bundle_root() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Documents")
        .join("Kyuubiki Projects")
}

fn unique_default_bundle_path(root: &Path) -> PathBuf {
    let first = root.join("Untitled.kyuubiki");
    if !first.exists() {
        return first;
    }
    for index in 2..10_000 {
        let candidate = root.join(format!("Untitled {index}.kyuubiki"));
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("Untitled-{}.kyuubiki", Uuid::new_v4()))
}

fn normalize_new_bundle_path(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    let mut candidate = if trimmed.is_empty() {
        unique_default_bundle_path(&default_bundle_root())
    } else {
        let supplied = PathBuf::from(trimmed);
        if !supplied.is_absolute() {
            return Err("new project bundle path must be absolute".to_string());
        }
        supplied
    };
    if candidate.extension().is_none() {
        candidate.set_extension(PROJECT_EXTENSION);
    }
    if !path_has_extension(&candidate, PROJECT_EXTENSION) {
        return Err("new project bundle path must end with .kyuubiki".to_string());
    }
    if candidate.exists() {
        return Err(format!("refusing to overwrite existing project bundle: {}", candidate.display()));
    }
    let parent = candidate
        .parent()
        .ok_or_else(|| format!("new project bundle has no parent: {}", candidate.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    Ok(candidate)
}

fn write_json_entry(
    writer: &mut ZipWriter<File>,
    path: &str,
    value: &serde_json::Value,
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

fn create_project_bundle(path: &str) -> Result<String, String> {
    let output = normalize_new_bundle_path(path)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let project_name = output
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Untitled");
    let project = json!({
        "project_id": format!("project-{}", Uuid::new_v4()),
        "name": project_name,
        "description": null,
        "inserted_at": now,
        "updated_at": now,
    });
    let file_manifest = default_project_file_manifest();
    let manifest = json!({
        "project_schema_version": "kyuubiki.project/v2",
        "exported_at": now,
        "project_file_manifest": file_manifest,
        "project": project,
        "models": [],
        "model_versions": [],
        "jobs": [],
        "results": [],
        "active_model_id": null,
        "active_version_id": null,
        "workspace_snapshot": null,
        "automation_presets": [],
        "asset_catalog": [],
        "asset_references": [],
    });
    let workspace_settings = json!({
        "active_model_id": null,
        "active_version_id": null,
        "exported_at": now,
        "project_schema_version": "kyuubiki.project/v2",
        "layout_version": "kyuubiki.project-layout/v1",
    });

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    let write_result = (|| {
        let mut writer = ZipWriter::new(file);
        write_json_entry(&mut writer, PROJECT_MANIFEST, &manifest)?;
        write_json_entry(&mut writer, ".kyuubiki/project.json", &file_manifest)?;
        write_json_entry(&mut writer, "project/project.json", &project)?;
        write_json_entry(&mut writer, "Assets/project/project.json", &project)?;
        write_json_entry(&mut writer, "ProjectSettings/workspace.json", &workspace_settings)?;
        write_json_entry(&mut writer, "ProjectSettings/asset-catalog.json", &json!([]))?;
        write_json_entry(&mut writer, "ProjectSettings/asset-references.json", &json!([]))?;
        write_json_entry(&mut writer, "jobs/jobs.json", &json!([]))?;
        write_json_entry(&mut writer, "Analysis/jobs/index.json", &json!([]))?;
        write_json_entry(&mut writer, "results/results.json", &json!([]))?;
        write_json_entry(&mut writer, "Analysis/results/index.json", &json!([]))?;
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
    if let Err(error) = write_result {
        let _ = fs::remove_file(&output);
        return Err(error);
    }

    serde_json::to_string_pretty(&json!({
        "created": true,
        "path": output,
        "summary": project_summary(&manifest),
    }))
    .map_err(|error| error.to_string())
}

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

#[cfg(test)]
mod project_bundle_tests {
    use super::*;

    fn test_bundle_path(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("kyuubiki-hub-{label}-{}", Uuid::new_v4()))
            .join("Research Study.kyuubiki")
    }

    #[test]
    fn creates_a_valid_empty_project_bundle_without_overwriting() {
        let path = test_bundle_path("create");
        let rendered = create_project_bundle(path.to_str().expect("UTF-8 test path"))
            .expect("create project bundle");
        let created: serde_json::Value =
            serde_json::from_str(&rendered).expect("created response JSON");

        assert_eq!(created["created"], true);
        assert_eq!(created["path"], path.to_string_lossy().as_ref());
        let bundle = read_project_bundle(&path).expect("read created project bundle");
        assert_eq!(validate_project_bundle(&bundle)["ok"], true);
        assert_eq!(bundle["project"]["name"], "Research Study");

        let duplicate = create_project_bundle(path.to_str().expect("UTF-8 test path"));
        assert!(duplicate.expect_err("must refuse overwrite").contains("refusing to overwrite"));

        fs::remove_dir_all(path.parent().expect("test bundle parent"))
            .expect("clean project bundle fixture");
    }

    #[test]
    fn rejects_relative_and_non_bundle_create_paths() {
        assert!(create_project_bundle("relative.kyuubiki")
            .expect_err("relative path must fail")
            .contains("must be absolute"));
        let path = test_bundle_path("extension").with_extension("zip");
        assert!(create_project_bundle(path.to_str().expect("UTF-8 test path"))
            .expect_err("wrong extension must fail")
            .contains("must end with .kyuubiki"));
    }
}
