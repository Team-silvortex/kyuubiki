use crate::{archive, migration_archive, migration_input, migration_validation, model, paths};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const TARGET_SCHEMA: &str = "kyuubiki.project/v2";

/// Explicit resource policy for offline project migration, not a solver limit.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMigrationLimits {
    pub max_source_bytes: u64,
    pub max_manifest_bytes: u64,
    pub max_expanded_bytes: u64,
    pub max_entries: usize,
}

impl Default for ProjectMigrationLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 4 * 1024 * 1024 * 1024,
            max_manifest_bytes: 64 * 1024 * 1024,
            max_expanded_bytes: 8 * 1024 * 1024 * 1024,
            max_entries: 100_000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMigrationPlan {
    pub schema_version: String,
    pub source_sha256: String,
    pub source_bytes: u64,
    pub source_format: String,
    pub source_schema: Option<String>,
    pub target_schema: String,
    /// current, upgrade_required, normalization_required, or blocked.
    pub status: String,
    pub changed_fields: Vec<String>,
    pub issues: Vec<String>,
    pub limits: ProjectMigrationLimits,
    pub source_mutation: bool,
    pub overwrite_allowed: bool,
    pub validation_scope: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMigrationReceipt {
    pub schema_version: String,
    pub migration_id: String,
    pub created_at: String,
    pub source_sha256: String,
    pub source_schema: String,
    pub target_schema: String,
    pub changed_fields: Vec<String>,
    pub preserved_entries: usize,
    pub rewritten_entries: Vec<String>,
    pub added_entries: Vec<String>,
    pub rollback: String,
    pub record_path: String,
    // Omitted inside the archive to avoid a self-referential file digest.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub directory_synced: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cleanup_pending: Option<String>,
}

/// Read-only preflight. Unknown versions and invalid structures are never normalized away.
pub fn plan_project_migration(
    input: &str,
    limits: &ProjectMigrationLimits,
) -> Result<ProjectMigrationPlan, String> {
    if limits.max_source_bytes == 0
        || limits.max_manifest_bytes == 0
        || limits.max_expanded_bytes == 0
        || limits.max_entries == 0
    {
        return Err("migration limits must be positive".into());
    }
    let path = migration_input::source_path(input)?;
    let (source_sha256, source_bytes) = migration_input::hash_file(&path, limits.max_source_bytes)?;
    let format = if paths::has_extension(&path, "kyuubiki") {
        "archive"
    } else {
        "json"
    };
    let mut plan = ProjectMigrationPlan {
        schema_version: "kyuubiki.project-migration-plan/v1".into(),
        source_sha256,
        source_bytes,
        source_format: format.into(),
        source_schema: None,
        target_schema: TARGET_SCHEMA.into(),
        status: "blocked".into(),
        changed_fields: Vec::new(),
        issues: Vec::new(),
        limits: limits.clone(),
        source_mutation: false,
        overwrite_allowed: false,
        validation_scope: "source_integrity_and_root_structure; apply_also_checks_record_mirrors"
            .into(),
    };
    match migration_input::read_source(&path, limits) {
        Ok(raw) => {
            plan.source_schema = raw
                .get("project_schema_version")
                .and_then(Value::as_str)
                .map(str::to_owned);
            match migration_validation::normalize_checked(raw.clone()) {
                Ok(target) => {
                    plan.changed_fields = target
                        .as_object()
                        .unwrap()
                        .iter()
                        .filter(|(key, value)| raw.get(*key) != Some(*value))
                        .map(|(key, _)| key.clone())
                        .collect();
                    plan.status = if plan.source_schema.as_deref() != Some(TARGET_SCHEMA) {
                        "upgrade_required"
                    } else if plan.changed_fields.is_empty() {
                        "current"
                    } else {
                        "normalization_required"
                    }
                    .into();
                }
                Err(error) => plan.issues.push(error),
            }
        }
        Err(error) => plan.issues.push(error),
    }
    let (after, _) = migration_input::hash_file(&path, limits.max_source_bytes)?;
    if after != plan.source_sha256 {
        return Err("project changed during inspection; stop writers and retry".into());
    }
    Ok(plan)
}

/// Copy-on-write migration. The approved digest binds *all* source bytes, including assets.
/// The original is the rollback copy; no source deletion or in-place downgrade is offered.
pub fn migrate_project_bundle(
    input: &str,
    output: &str,
    expected_source_sha256: &str,
    limits: &ProjectMigrationLimits,
) -> Result<ProjectMigrationReceipt, String> {
    if expected_source_sha256.len() != 64
        || !expected_source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("expected source SHA-256 must be 64 lowercase hexadecimal characters".into());
    }
    let source = migration_input::source_path(input)?;
    let target = paths::output(output, "migration output")?;
    if !paths::has_extension(&target, "kyuubiki") {
        return Err("migration output must end with .kyuubiki".into());
    }
    if fs::symlink_metadata(&target).is_ok() {
        return Err("refusing to overwrite migration output (including symbolic links)".into());
    }
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|error| error.to_string())?;
    let target = parent.join(target.file_name().ok_or("output requires a file name")?);
    let stage = Stage::new(&parent)?;
    let result = (|| {
        let snapshot = stage.0.join(if paths::has_extension(&source, "kyuubiki") {
            "source.kyuubiki"
        } else {
            "source.json"
        });
        migration_input::copy_source(&source, &snapshot, limits.max_source_bytes)?;
        let plan =
            plan_project_migration(snapshot.to_str().ok_or("non-UTF-8 staging path")?, limits)?;
        if plan.source_sha256 != expected_source_sha256 {
            return Err(
                "source digest differs from approved plan; inspect again before migration".into(),
            );
        }
        if plan.status == "blocked" {
            return Err(format!(
                "project migration blocked: {}",
                plan.issues.join("; ")
            ));
        }
        let manifest = migration_validation::normalize_checked(migration_input::read_source(
            &snapshot, limits,
        )?)?;
        let canonical = stage.0.join("canonical.kyuubiki");
        archive::create_archive(&canonical, &manifest)?;
        let candidate = stage.0.join("candidate.kyuubiki");
        let id = Uuid::new_v4().to_string();
        let mut receipt = ProjectMigrationReceipt {
            schema_version: "kyuubiki.project-migration-receipt/v1".into(),
            migration_id: id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            source_sha256: plan.source_sha256,
            source_schema: plan.source_schema.ok_or("source schema missing")?,
            target_schema: TARGET_SCHEMA.into(),
            changed_fields: plan.changed_fields,
            preserved_entries: 0,
            rewritten_entries: Vec::new(),
            added_entries: Vec::new(),
            rollback: "reopen_retained_source_no_automatic_downgrade".into(),
            record_path: format!(".kyuubiki/migrations/{id}.json"),
            output_sha256: None,
            directory_synced: None,
            cleanup_pending: None,
        };
        migration_archive::build_candidate(
            &snapshot,
            &canonical,
            &candidate,
            &manifest,
            &mut receipt,
            limits,
        )?;
        let reread = migration_input::read_source(&candidate, limits)?;
        if reread != manifest || model::validation(&reread)["ok"] != true {
            return Err("migrated project failed full manifest readback".into());
        }
        let (source_now, _) = migration_input::hash_file(&source, limits.max_source_bytes)?;
        if source_now != receipt.source_sha256 {
            return Err("source changed during migration; original was not modified, retry after stopping writers".into());
        }
        OpenOptions::new()
            .write(true)
            .open(&candidate)
            .and_then(|file| file.sync_all())
            .map_err(|error| format!("cannot sync candidate: {error}"))?;
        receipt.output_sha256 =
            Some(migration_input::hash_file(&candidate, limits.max_source_bytes)?.0);
        publish_candidate(&candidate, &target)?;
        receipt.directory_synced = Some(false);
        #[cfg(unix)]
        {
            receipt.directory_synced =
                Some(File::open(&parent).and_then(|file| file.sync_all()).is_ok());
        }
        Ok(receipt)
    })();
    match fs::remove_dir_all(&stage.0) {
        Ok(()) => result,
        Err(cleanup) => {
            let pending = stage.0.file_name().unwrap().to_string_lossy().into_owned();
            match result {
                Ok(mut receipt) => {
                    receipt.cleanup_pending = Some(pending);
                    Ok(receipt)
                }
                Err(error) => Err(format!(
                    "{error}; cleanup pending beside output: {pending} ({cleanup})"
                )),
            }
        }
    }
}

/// Verify a moved/copied archive against a separately retained receipt. Digests are
/// integrity evidence, not a publisher signature or material-result certification.
pub fn verify_project_migration(
    input: &str,
    receipt: &ProjectMigrationReceipt,
    limits: &ProjectMigrationLimits,
) -> Result<(), String> {
    if receipt.schema_version != "kyuubiki.project-migration-receipt/v1"
        || receipt.target_schema != TARGET_SCHEMA
        || receipt.record_path != format!(".kyuubiki/migrations/{}.json", receipt.migration_id)
        || Uuid::parse_str(&receipt.migration_id).is_err()
    {
        return Err("unsupported or invalid project migration receipt".into());
    }
    let path = migration_input::source_path(input)?;
    if !paths::has_extension(&path, "kyuubiki") {
        return Err("verification requires a .kyuubiki archive".into());
    }
    let (before, _) = migration_input::hash_file(&path, limits.max_source_bytes)?;
    if receipt.output_sha256.as_deref() != Some(before.as_str()) {
        return Err("migrated archive digest differs from the retained receipt".into());
    }
    let mut archive = migration_input::open_archive(&path, limits)?;
    migration_input::inventory(&mut archive, limits)?;
    let record = migration_input::read_json_entry(
        &mut archive,
        &receipt.record_path,
        limits.max_manifest_bytes,
    )?;
    let mut expected = receipt.clone();
    expected.output_sha256 = None;
    expected.directory_synced = None;
    expected.cleanup_pending = None;
    if record != serde_json::to_value(&expected).map_err(|error| error.to_string())? {
        return Err("embedded migration record differs from the retained receipt".into());
    }
    let manifest =
        migration_input::read_json_entry(&mut archive, "project.json", limits.max_manifest_bytes)?;
    if migration_validation::normalize_checked(manifest.clone())? != manifest
        || manifest["project_schema_version"] != TARGET_SCHEMA
    {
        return Err("migrated manifest is not in the verified target format".into());
    }
    if migration_input::hash_file(&path, limits.max_source_bytes)?.0 != before {
        return Err("archive changed during verification".into());
    }
    Ok(())
}

fn publish_candidate(candidate: &Path, target: &Path) -> Result<(), String> {
    // No rename/copy fallback: neither has the same no-clobber visibility contract.
    fs::hard_link(candidate, target).map_err(|error| format!("cannot publish migration without overwrite; same-filesystem hard links are required: {error}"))
}

struct Stage(PathBuf);

impl Stage {
    fn new(parent: &Path) -> Result<Self, String> {
        let path = parent.join(format!(".kyuubiki-migrate-{}", Uuid::new_v4()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&path)
            .map_err(|error| format!("cannot create private migration staging: {error}"))?;
        Ok(Self(path))
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_never_clobbers_a_concurrently_created_target() {
        let root = Stage::new(&std::env::temp_dir()).unwrap();
        let candidate = root.0.join("candidate");
        fs::write(&candidate, b"verified candidate").unwrap();
        let target = root.0.join("target");
        // Simulate a target created after the caller's initial existence check.
        fs::write(&target, b"other writer").unwrap();
        assert!(publish_candidate(&candidate, &target).is_err());
        assert_eq!(fs::read(target).unwrap(), b"other writer");
        assert_eq!(fs::read(candidate).unwrap(), b"verified candidate");
    }
}
