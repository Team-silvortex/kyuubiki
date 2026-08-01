mod archive;
mod model;
mod paths;

use chrono::{SecondsFormat, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

pub fn create_project_bundle(path: &str) -> Result<String, String> {
    let output = paths::new_bundle(path)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let project_name = output
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Untitled");
    let manifest = model::normalize(json!({
        "project_schema_version": "kyuubiki.project/v2",
        "exported_at": now,
        "project_file_manifest": model::default_file_manifest(),
        "project": {
            "project_id": format!("project-{}", Uuid::new_v4()),
            "name": project_name,
            "description": null,
            "inserted_at": now,
            "updated_at": now,
        },
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
    }))?;
    archive::create_archive(&output, &manifest)?;
    render(&json!({
        "created": true,
        "path": output,
        "summary": model::summary(&manifest),
    }))
}

pub fn inspect_project_bundle(path: &str) -> Result<String, String> {
    render(&model::summary(&archive::read_input(
        path,
        "project input",
    )?))
}

pub fn read_project_bundle(path: &str) -> Result<Value, String> {
    archive::read_input(path, "project input")
}

pub fn validate_project_bundle(path: &str) -> Result<String, String> {
    render(&model::validation(&archive::read_input(
        path,
        "project input",
    )?))
}

pub fn normalize_project_bundle(input: &str, output: &str) -> Result<String, String> {
    let output = paths::output(output, "output path")?;
    if output.exists() {
        return Err(format!(
            "refusing to overwrite output: {}",
            output.display()
        ));
    }
    archive::write_normalized(input, &output)?;
    Ok(format!(
        "native project normalize completed -> {}",
        output.display()
    ))
}

pub fn unpack_project_bundle(input: &str, output: &str) -> Result<String, String> {
    let input = paths::existing_bundle(input, "project bundle path")?;
    let output = paths::output(output, "output path")?;
    archive::unpack_archive(&input, &output)?;
    Ok(format!(
        "native project unpack completed -> {}",
        output.display()
    ))
}

pub fn pack_project_bundle(input: &str, output: &str) -> Result<String, String> {
    let input = paths::existing_directory(input, "project directory path")?;
    let output = paths::output(output, "output path")?;
    if output.exists() {
        return Err(format!(
            "refusing to overwrite output: {}",
            output.display()
        ));
    }
    archive::pack_directory(&input, &output)?;
    Ok(format!(
        "native project pack completed -> {}",
        output.display()
    ))
}

pub fn diff_project_bundles(left: &str, right: &str) -> Result<String, String> {
    let left = archive::read_input(left, "left project input")?;
    let right = archive::read_input(right, "right project input")?;
    render(&model::diff(&left, &right))
}

pub fn validation_passed(rendered: &str) -> Result<bool, String> {
    let report: Value = serde_json::from_str(rendered)
        .map_err(|error| format!("invalid project validation report: {error}"))?;
    Ok(report.get("ok").and_then(Value::as_bool) == Some(true))
}

fn render(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(label: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "kyuubiki-project-bundle-{label}-{}",
                Uuid::new_v4()
            ))
            .join("Research Study.kyuubiki")
    }

    #[test]
    fn creates_inspects_validates_and_refuses_overwrite() {
        let path = fixture("roundtrip");
        let rendered = create_project_bundle(path.to_str().expect("UTF-8 path"))
            .expect("create project bundle");
        let created: Value = serde_json::from_str(&rendered).expect("created JSON");
        assert_eq!(created["created"], true);
        assert_eq!(created["summary"]["project_name"], "Research Study");
        assert_eq!(created["summary"]["asset_count"], 2);

        let inspected = inspect_project_bundle(path.to_str().expect("UTF-8 path"))
            .expect("inspect project bundle");
        let inspected: Value = serde_json::from_str(&inspected).expect("inspect JSON");
        assert_eq!(inspected["schema"], "kyuubiki.project/v2");

        let validation = validate_project_bundle(path.to_str().expect("UTF-8 path"))
            .expect("validate project bundle");
        assert!(validation_passed(&validation).expect("validation report"));
        assert!(
            create_project_bundle(path.to_str().expect("UTF-8 path"))
                .expect_err("must refuse overwrite")
                .contains("refusing to overwrite")
        );

        fs::remove_dir_all(path.parent().expect("fixture parent")).expect("clean fixture");
    }

    #[test]
    fn rejects_asset_references_outside_the_catalog() {
        let path = fixture("invalid-reference");
        create_project_bundle(path.to_str().expect("UTF-8 path")).expect("create bundle");
        let mut bundle =
            archive::read_input(path.to_str().expect("UTF-8 path"), "project test input")
                .expect("read bundle");
        bundle["asset_references"] = json!([{
            "from_guid": "missing-from",
            "relation": "contains",
            "to_guid": "missing-to"
        }]);
        let report = model::validation(&bundle);
        assert_eq!(report["ok"], false);
        assert!(
            report["issues"]
                .as_array()
                .expect("issues")
                .iter()
                .any(|issue| issue
                    .as_str()
                    .is_some_and(|text| text.contains("unknown from_guid")))
        );
        fs::remove_dir_all(path.parent().expect("fixture parent")).expect("clean fixture");
    }

    #[test]
    fn rejects_relative_and_wrong_extension_create_paths() {
        assert!(
            create_project_bundle("relative.kyuubiki")
                .expect_err("relative path must fail")
                .contains("must be absolute")
        );
        let path = fixture("extension").with_extension("zip");
        assert!(
            create_project_bundle(path.to_str().expect("UTF-8 path"))
                .expect_err("wrong extension must fail")
                .contains("must end with .kyuubiki")
        );
    }

    #[test]
    fn preserves_extension_assets_across_unpack_pack_and_normalize() {
        let original = fixture("preserve");
        let root = original.parent().expect("fixture root");
        create_project_bundle(original.to_str().expect("UTF-8 path")).expect("create bundle");

        let unpacked = root.join("unpacked");
        unpack_project_bundle(
            original.to_str().expect("UTF-8 path"),
            unpacked.to_str().expect("UTF-8 path"),
        )
        .expect("unpack bundle");
        let extension_dir = unpacked.join("Extensions");
        fs::create_dir_all(&extension_dir).expect("create extension directory");
        fs::write(extension_dir.join("vendor.dat"), b"retained-extension")
            .expect("write extension asset");

        let repacked = root.join("Repacked.kyuubiki");
        pack_project_bundle(
            unpacked.to_str().expect("UTF-8 path"),
            repacked.to_str().expect("UTF-8 path"),
        )
        .expect("repack bundle");
        let normalized = root.join("Normalized.kyuubiki");
        normalize_project_bundle(
            repacked.to_str().expect("UTF-8 path"),
            normalized.to_str().expect("UTF-8 path"),
        )
        .expect("normalize bundle");

        let restored = root.join("restored");
        unpack_project_bundle(
            normalized.to_str().expect("UTF-8 path"),
            restored.to_str().expect("UTF-8 path"),
        )
        .expect("unpack normalized bundle");
        assert_eq!(
            fs::read(restored.join("Extensions/vendor.dat")).expect("read retained asset"),
            b"retained-extension"
        );
        let diff = diff_project_bundles(
            original.to_str().expect("UTF-8 path"),
            normalized.to_str().expect("UTF-8 path"),
        )
        .expect("diff bundles");
        let diff: Value = serde_json::from_str(&diff).expect("diff JSON");
        assert_eq!(diff["changed_project_identity"], false);

        fs::remove_dir_all(root).expect("clean fixture");
    }

    #[test]
    fn materializes_json_records_into_the_standard_archive_layout() {
        let bundle = fixture("json-layout");
        let root = bundle.parent().expect("fixture root");
        fs::create_dir_all(root).expect("create fixture root");
        let json_input = root.join("project-input.json");
        fs::write(
            &json_input,
            serde_json::to_vec_pretty(&json!({
                "project_schema_version": "kyuubiki.project/v2",
                "project": {"project_id": "p-1", "name": "Imported"},
                "models": [{"model_id": "model-1", "name": "Frame"}],
                "model_versions": [{
                    "version_id": "version-1",
                    "model_id": "model-1",
                    "kind": "frame",
                    "version_number": 1
                }]
            }))
            .expect("serialize JSON fixture"),
        )
        .expect("write JSON fixture");
        normalize_project_bundle(
            json_input.to_str().expect("UTF-8 path"),
            bundle.to_str().expect("UTF-8 path"),
        )
        .expect("normalize JSON project");

        let unpacked = root.join("json-layout-unpacked");
        unpack_project_bundle(
            bundle.to_str().expect("UTF-8 path"),
            unpacked.to_str().expect("UTF-8 path"),
        )
        .expect("unpack normalized project");
        assert!(unpacked.join("Assets/models/model-1.json").is_file());
        assert!(unpacked.join("Assets/models/model-1.json.meta").is_file());
        assert!(unpacked.join("Assets/versions/version-1.json").is_file());
        assert!(
            unpacked
                .join("Assets/versions/version-1.json.meta")
                .is_file()
        );
        assert!(
            validation_passed(
                &validate_project_bundle(bundle.to_str().expect("UTF-8 path"))
                    .expect("validate normalized project")
            )
            .expect("validation report")
        );

        fs::remove_dir_all(root).expect("clean fixture");
    }
}
