mod support;

use kyuubiki_project_bundle::{
    ProjectMigrationLimits, migrate_project_bundle, plan_project_migration, read_project_bundle,
};
use serde_json::json;
use std::fs;
use support::{Fixture, manifest};

#[test]
fn malformed_optional_arrays_are_never_replaced_with_empty_data() {
    for key in [
        "jobs",
        "results",
        "automation_presets",
        "asset_catalog",
        "asset_references",
    ] {
        for value in [json!(null), json!({"retained": true}), json!("invalid")] {
            let fixture = Fixture::new();
            let mut raw = manifest();
            raw[key] = value;
            let path = fixture.json(&raw);
            assert!(
                read_project_bundle(path.to_str().unwrap())
                    .unwrap_err()
                    .contains("refusing to discard data")
            );
            assert_eq!(
                plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
                    .unwrap()
                    .status,
                "blocked"
            );
        }
    }
}

#[test]
fn unknown_versions_and_layouts_are_blocked_without_downgrade() {
    for schema in [
        "kyuubiki.project/v3",
        "kyuubiki.project/v999",
        "other/v1",
        "",
    ] {
        let fixture = Fixture::new();
        let mut raw = manifest();
        raw["project_schema_version"] = schema.into();
        let path = fixture.json(&raw);
        let plan =
            plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
                .unwrap();
        assert_eq!(plan.source_schema.as_deref(), Some(schema));
        assert_eq!(plan.status, "blocked");
        let output = fixture.0.join("blocked.kyuubiki");
        assert!(
            migrate_project_bundle(
                path.to_str().unwrap(),
                output.to_str().unwrap(),
                &plan.source_sha256,
                &ProjectMigrationLimits::default()
            )
            .is_err()
        );
        assert!(!output.exists());
        fixture.no_staging();
    }
    let fixture = Fixture::new();
    let mut raw = manifest();
    raw["project_file_manifest"] = json!({"layout_version": "kyuubiki.project-layout/v2"});
    let path = fixture.json(&raw);
    assert!(
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
            .unwrap()
            .issues[0]
            .contains("layout")
    );
}

#[test]
fn duplicate_nested_json_keys_and_trailing_documents_are_rejected() {
    for bytes in [
        br#"{"project_schema_version":"kyuubiki.project/v1","project_schema_version":"kyuubiki.project/v2"}"#.as_slice(),
        br#"{"project":{"name":"one","name":"two"}}"#.as_slice(),
        b"{} {}".as_slice(),
    ] {
        let fixture = Fixture::new();
        let path = fixture.0.join("source.json");
        fs::write(&path, bytes).unwrap();
        assert_eq!(plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default()).unwrap().status, "blocked");
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn broken_references_duplicate_ids_and_colliding_slugs_are_blocked() {
    for variant in 0..5 {
        let fixture = Fixture::new();
        let mut raw = manifest();
        match variant {
            0 => raw["results"][0]["job_id"] = "missing".into(),
            1 => raw["models"]
                .as_array_mut()
                .unwrap()
                .push(json!({"model_id": "model-1"})),
            2 => raw["models"]
                .as_array_mut()
                .unwrap()
                .push(json!({"model_id": "Model 1"})),
            3 => raw["active_model_id"] = json!(17),
            _ => raw["results"] = json!([{}]),
        }
        let path = fixture.json(&raw);
        let plan =
            plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
                .unwrap();
        assert_eq!(plan.status, "blocked", "variant {variant}");
    }
}

#[test]
fn unsafe_and_case_colliding_archive_paths_are_blocked() {
    for name in [
        "../escape",
        "/absolute",
        "a/../../escape",
        "C:/test",
        "a\\b",
        "CON",
        "a/COM1.txt",
        "a/trailing.",
        "a//b",
        "PROJECT.JSON",
    ] {
        let fixture = Fixture::new();
        let path = fixture.zip(vec![
            ("project.json", serde_json::to_vec(&manifest()).unwrap()),
            (name, vec![0]),
        ]);
        assert_eq!(
            plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
                .unwrap()
                .status,
            "blocked",
            "{name}"
        );
    }
}

#[test]
fn archive_prefix_collisions_and_truncation_are_blocked() {
    let fixture = Fixture::new();
    let path = fixture.zip(vec![
        ("project.json", serde_json::to_vec(&manifest()).unwrap()),
        ("data", vec![0]),
        ("data/child.bin", vec![1]),
    ]);
    assert_eq!(
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
            .unwrap()
            .status,
        "blocked"
    );
    fs::write(&path, b"PK").unwrap();
    assert_eq!(
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
            .unwrap()
            .status,
        "blocked"
    );
}

#[test]
fn input_and_expanded_limits_fail_closed() {
    let fixture = Fixture::new();
    let json = fixture.json(&manifest());
    let limits = ProjectMigrationLimits {
        max_source_bytes: 4,
        ..Default::default()
    };
    assert!(plan_project_migration(json.to_str().unwrap(), &limits).is_err());
    let limits = ProjectMigrationLimits {
        max_manifest_bytes: 4,
        ..Default::default()
    };
    assert_eq!(
        plan_project_migration(json.to_str().unwrap(), &limits)
            .unwrap()
            .status,
        "blocked"
    );
    let path = fixture.zip(vec![
        ("project.json", serde_json::to_vec(&manifest()).unwrap()),
        ("large.bin", vec![0; 8192]),
    ]);
    for limits in [
        ProjectMigrationLimits {
            max_expanded_bytes: 1024,
            ..Default::default()
        },
        ProjectMigrationLimits {
            max_entries: 1,
            ..Default::default()
        },
    ] {
        assert_eq!(
            plan_project_migration(path.to_str().unwrap(), &limits)
                .unwrap()
                .status,
            "blocked"
        );
    }
}

#[test]
fn corrupted_attachment_crc_and_archived_symlinks_are_blocked() {
    use std::io::Write;
    use zip::{ZipWriter, write::SimpleFileOptions};
    let fixture = Fixture::new();
    let marker = b"ATTACHMENT_CRC_SENTINEL";
    let source = fixture.zip(vec![
        ("project.json", serde_json::to_vec(&manifest()).unwrap()),
        ("attachment.bin", marker.to_vec()),
    ]);
    let mut bytes = fs::read(&source).unwrap();
    let offset = bytes
        .windows(marker.len())
        .position(|part| part == marker)
        .unwrap();
    bytes[offset] ^= 1;
    fs::write(&source, bytes).unwrap();
    assert_eq!(
        plan_project_migration(source.to_str().unwrap(), &ProjectMigrationLimits::default())
            .unwrap()
            .status,
        "blocked"
    );
    let mut writer = ZipWriter::new(fs::File::create(&source).unwrap());
    writer
        .start_file("project.json", SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(&serde_json::to_vec(&manifest()).unwrap())
        .unwrap();
    writer
        .add_symlink("linked-file", "../outside", SimpleFileOptions::default())
        .unwrap();
    writer.finish().unwrap();
    let plan = plan_project_migration(source.to_str().unwrap(), &ProjectMigrationLimits::default())
        .unwrap();
    assert_eq!(plan.status, "blocked");
    assert!(plan.issues[0].contains("link or special file"));
}

#[test]
fn unsafe_invocations_fail_before_creating_staging_or_output() {
    let fixture = Fixture::new();
    let source = fixture.json(&manifest());
    let limits = ProjectMigrationLimits::default();
    let output = fixture.0.join("new.kyuubiki");
    for digest in ["", "abc", &"A".repeat(64), &"g".repeat(64)] {
        assert!(
            migrate_project_bundle(
                source.to_str().unwrap(),
                output.to_str().unwrap(),
                digest,
                &limits
            )
            .is_err()
        );
    }
    assert!(plan_project_migration(fixture.0.to_str().unwrap(), &limits).is_err());
    let invalid_limits = ProjectMigrationLimits {
        max_expanded_bytes: 0,
        ..Default::default()
    };
    assert!(plan_project_migration(source.to_str().unwrap(), &invalid_limits).is_err());
    assert!(!output.exists());
    fixture.no_staging();
}

#[cfg(unix)]
#[test]
fn symbolic_link_sources_and_dangling_targets_are_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new();
    let path = fixture.json(&manifest());
    let alias = fixture.0.join("link.json");
    symlink(&path, &alias).unwrap();
    let limits = ProjectMigrationLimits::default();
    assert!(plan_project_migration(alias.to_str().unwrap(), &limits).is_err());
    let plan = plan_project_migration(path.to_str().unwrap(), &limits).unwrap();
    let target = fixture.0.join("out.kyuubiki");
    symlink(fixture.0.join("missing"), &target).unwrap();
    assert!(
        migrate_project_bundle(
            path.to_str().unwrap(),
            target.to_str().unwrap(),
            &plan.source_sha256,
            &limits
        )
        .is_err()
    );
    assert!(
        fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let safe = fixture.0.join("safe.kyuubiki");
    migrate_project_bundle(
        path.to_str().unwrap(),
        safe.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    assert_eq!(
        fs::metadata(safe).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fixture.no_staging();
}
