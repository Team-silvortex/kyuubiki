mod support;

use kyuubiki_project_bundle::{
    ProjectMigrationLimits, migrate_project_bundle, plan_project_migration, read_project_bundle,
    verify_project_migration,
};
use serde_json::{Value, json};
use std::fs;
use support::{Fixture, manifest, read_entry};

#[test]
fn plan_is_read_only_and_reports_real_source_version() {
    let fixture = Fixture::new();
    let input = fixture.json(&manifest());
    let before = fs::read(&input).unwrap();
    let plan = plan_project_migration(input.to_str().unwrap(), &ProjectMigrationLimits::default())
        .unwrap();
    assert_eq!(plan.status, "upgrade_required");
    assert_eq!(plan.source_schema.as_deref(), Some("kyuubiki.project/v1"));
    assert_eq!(plan.target_schema, "kyuubiki.project/v2");
    assert!(
        plan.changed_fields
            .contains(&"project_schema_version".into())
    );
    assert!(!plan.source_mutation && !plan.overwrite_allowed);
    assert_eq!(fs::read(&input).unwrap(), before);
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn upgrade_preserves_research_fields_and_original_and_embeds_receipt() {
    let fixture = Fixture::new();
    let raw = manifest();
    let input = fixture.json(&raw);
    let before = fs::read(&input).unwrap();
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(input.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("upgraded.kyuubiki");
    let receipt = migrate_project_bundle(
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    let result = read_project_bundle(output.to_str().unwrap()).unwrap();
    for field in [
        "models",
        "model_versions",
        "jobs",
        "results",
        "project",
        "vendor_metadata",
        "active_version_id",
    ] {
        assert_eq!(result[field], raw[field], "{field} was changed");
    }
    assert!(
        result["results"][0]["result"]["u"][0]
            .as_f64()
            .unwrap()
            .is_sign_negative()
    );
    assert_eq!(fs::read(&input).unwrap(), before);
    let record: Value = serde_json::from_slice(&read_entry(&output, &receipt.record_path)).unwrap();
    assert_eq!(record["source_sha256"], plan.source_sha256);
    assert!(record.get("output_sha256").is_none());
    assert_eq!(receipt.output_sha256.as_ref().unwrap().len(), 64);
    assert_eq!(
        plan_project_migration(output.to_str().unwrap(), &limits)
            .unwrap()
            .status,
        "current"
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&read_entry(&output, "Analysis/results/job-1.json"))
            .unwrap(),
        raw["results"][0]
    );
    fixture.no_staging();
}

#[test]
fn preserves_opaque_attachments_and_updates_workspace_mirror_without_dropping_extensions() {
    let fixture = Fixture::new();
    let raw = manifest();
    let source = fixture.zip(vec![
        ("project.json", serde_json::to_vec(&raw).unwrap()),
        ("Extensions/vendor.bin", vec![0, 255, 1, 0, 7]),
        ("Attachments/材料.txt", b"retained notes".to_vec()),
        ("ProjectSettings/workspace.json", serde_json::to_vec(&json!({"project_schema_version": "kyuubiki.project/v1", "active_model_id": "model-1", "vendor_preference": "retain"})).unwrap()),
    ]);
    let before = fs::read(&source).unwrap();
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("new.kyuubiki");
    let receipt = migrate_project_bundle(
        source.to_str().unwrap(),
        output.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    assert_eq!(
        read_entry(&output, "Extensions/vendor.bin"),
        vec![0, 255, 1, 0, 7]
    );
    assert_eq!(
        read_entry(&output, "Attachments/材料.txt"),
        b"retained notes"
    );
    let workspace: Value =
        serde_json::from_slice(&read_entry(&output, "ProjectSettings/workspace.json")).unwrap();
    assert_eq!(workspace["project_schema_version"], "kyuubiki.project/v2");
    assert_eq!(workspace["vendor_preference"], "retain");
    assert!(
        receipt
            .rewritten_entries
            .contains(&"ProjectSettings/workspace.json".into())
    );
    assert_eq!(fs::read(source).unwrap(), before);
    fixture.no_staging();
}

#[test]
fn conflicting_record_mirrors_fail_before_publication() {
    let fixture = Fixture::new();
    let source = fixture.zip(vec![
        ("project.json", serde_json::to_vec(&manifest()).unwrap()),
        (
            "Analysis/results/job-1.json",
            br#"{"job_id":"job-1","result":{"temperatures":[1,2]}}"#.to_vec(),
        ),
    ]);
    let before = fs::read(&source).unwrap();
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("new.kyuubiki");
    assert!(
        migrate_project_bundle(
            source.to_str().unwrap(),
            output.to_str().unwrap(),
            &plan.source_sha256,
            &limits
        )
        .unwrap_err()
        .contains("conflicting project record mirror")
    );
    assert!(!output.exists());
    assert_eq!(fs::read(source).unwrap(), before);
    fixture.no_staging();
}

#[test]
fn stale_approval_detects_changes_outside_the_primary_manifest() {
    let fixture = Fixture::new();
    let entries = || vec![("project.json", serde_json::to_vec(&manifest()).unwrap())];
    let source = fixture.zip(entries());
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let mut changed = entries();
    changed.push(("new-attachment.bin", vec![1, 2, 3]));
    fixture.zip(changed);
    let output = fixture.0.join("new.kyuubiki");
    assert!(
        migrate_project_bundle(
            source.to_str().unwrap(),
            output.to_str().unwrap(),
            &plan.source_sha256,
            &limits
        )
        .unwrap_err()
        .contains("differs from approved plan")
    );
    assert!(!output.exists());
    fixture.no_staging();
}

#[test]
fn existing_targets_and_in_place_migration_are_never_overwritten() {
    let fixture = Fixture::new();
    let input = fixture.json(&manifest());
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(input.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("existing.kyuubiki");
    fs::write(&output, b"keep").unwrap();
    assert!(
        migrate_project_bundle(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            &plan.source_sha256,
            &limits
        )
        .is_err()
    );
    assert_eq!(fs::read(&output).unwrap(), b"keep");
    let source = fixture.zip(vec![(
        "project.json",
        serde_json::to_vec(&manifest()).unwrap(),
    )]);
    let bytes = fs::read(&source).unwrap();
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    assert!(
        migrate_project_bundle(
            source.to_str().unwrap(),
            source.to_str().unwrap(),
            &plan.source_sha256,
            &limits
        )
        .is_err()
    );
    assert_eq!(fs::read(source).unwrap(), bytes);
    fixture.no_staging();
}

#[test]
fn migration_chains_retain_prior_receipts_and_current_version_is_not_bumped() {
    let fixture = Fixture::new();
    let source = fixture.json(&manifest());
    let limits = ProjectMigrationLimits::default();
    let first = fixture.0.join("first.kyuubiki");
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let receipt = migrate_project_bundle(
        source.to_str().unwrap(),
        first.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    let second = fixture.0.join("second.kyuubiki");
    let plan = plan_project_migration(first.to_str().unwrap(), &limits).unwrap();
    assert!(plan.changed_fields.is_empty());
    let next = migrate_project_bundle(
        first.to_str().unwrap(),
        second.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    assert_eq!(
        read_entry(&first, &receipt.record_path),
        read_entry(&second, &receipt.record_path)
    );
    assert_ne!(receipt.migration_id, next.migration_id);
    assert_eq!(next.source_schema, next.target_schema);
    fixture.no_staging();
}

#[test]
fn failed_output_budget_leaves_no_partial_final_file_or_staging() {
    let fixture = Fixture::new();
    let source = fixture.json(&manifest());
    let limits = ProjectMigrationLimits {
        max_expanded_bytes: 100,
        ..Default::default()
    };
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("new.kyuubiki");
    assert!(
        migrate_project_bundle(
            source.to_str().unwrap(),
            output.to_str().unwrap(),
            &plan.source_sha256,
            &limits
        )
        .is_err()
    );
    assert!(!output.exists());
    fixture.no_staging();
}

#[test]
fn moved_archive_verifies_without_source_or_original_absolute_paths() {
    let fixture = Fixture::new();
    let source = fixture.json(&manifest());
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("migrated.kyuubiki");
    let receipt = migrate_project_bundle(
        source.to_str().unwrap(),
        output.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    let moved = fixture.0.join("moved.kyuubiki");
    fs::rename(&output, &moved).unwrap();
    fs::rename(source, fixture.0.join("retained-original.json")).unwrap();
    verify_project_migration(moved.to_str().unwrap(), &receipt, &limits).unwrap();
    let serialized = serde_json::to_string(&receipt).unwrap();
    assert!(!serialized.contains(fixture.0.to_str().unwrap()));
    let mut altered_receipt = receipt.clone();
    altered_receipt.source_sha256 = "0".repeat(64);
    assert!(
        verify_project_migration(moved.to_str().unwrap(), &altered_receipt, &limits)
            .unwrap_err()
            .contains("embedded migration record")
    );
    let mut bytes = fs::read(&moved).unwrap();
    bytes.push(0);
    fs::write(&moved, bytes).unwrap();
    assert!(
        verify_project_migration(moved.to_str().unwrap(), &receipt, &limits)
            .unwrap_err()
            .contains("digest differs")
    );
}

#[test]
fn incomplete_and_future_receipts_do_not_verify() {
    let fixture = Fixture::new();
    let source = fixture.json(&manifest());
    let limits = ProjectMigrationLimits::default();
    let plan = plan_project_migration(source.to_str().unwrap(), &limits).unwrap();
    let output = fixture.0.join("migrated.kyuubiki");
    let receipt = migrate_project_bundle(
        source.to_str().unwrap(),
        output.to_str().unwrap(),
        &plan.source_sha256,
        &limits,
    )
    .unwrap();
    for variant in 0..4 {
        let mut invalid = receipt.clone();
        match variant {
            0 => invalid.output_sha256 = None,
            1 => invalid.schema_version = "kyuubiki.project-migration-receipt/v2".into(),
            2 => invalid.record_path = "../other.json".into(),
            _ => invalid.migration_id = "../../other".into(),
        }
        assert!(verify_project_migration(output.to_str().unwrap(), &invalid, &limits).is_err());
    }
}
