use crate::operator_package_generation_session::{
    GenerationJanitorReport, OperatorPackageGenerationSession,
};
use kyuubiki_installer::{install_operator_package_into, managed_operator_package_status_in};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const GENERATION_MARKER_FILE: &str = "kyuubiki-agent-generation.json";
const GENERATION_MARKER_SCHEMA: &str = "kyuubiki.agent-operator-generation/v2";
const MAX_MARKER_BYTES: u64 = 16 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct GenerationMarker {
    schema_version: String,
    generation_id: String,
    session_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct OwnedOperatorPackageGeneration {
    generation_root: PathBuf,
    generations_root: PathBuf,
    marker: GenerationMarker,
    session: Arc<OperatorPackageGenerationSession>,
}

pub(crate) struct PreparedOperatorPackageGeneration {
    owned: OwnedOperatorPackageGeneration,
    cleanup_armed: bool,
}

pub(crate) fn prepare_operator_package_generation(
    session: Arc<OperatorPackageGenerationSession>,
    active_packages_root: &Path,
    replacing_package_id: &str,
) -> Result<PreparedOperatorPackageGeneration, String> {
    prepare_operator_package_generation_excluding(
        session,
        active_packages_root,
        &BTreeSet::from([replacing_package_id.to_string()]),
    )
}

pub(crate) fn prepare_operator_package_generation_excluding(
    session: Arc<OperatorPackageGenerationSession>,
    active_packages_root: &Path,
    excluded_package_ids: &BTreeSet<String>,
) -> Result<PreparedOperatorPackageGeneration, String> {
    let active_packages_root =
        canonical_directory(active_packages_root, "active operator packages root")?;
    if active_packages_root
        .file_name()
        .and_then(|value| value.to_str())
        != Some("packages")
    {
        return Err(
            "active operator packages root must use the managed packages layout".to_string(),
        );
    }
    let active_store_root = active_packages_root
        .parent()
        .ok_or_else(|| "active operator packages root has no store parent".to_string())?;
    let active = managed_operator_package_status_in(active_store_root)?;

    let generations_root = session.generations_root().to_path_buf();

    let generation_id = next_generation_id()?;
    let generation_root = generations_root.join(&generation_id);
    fs::create_dir(&generation_root).map_err(|error| {
        format!(
            "failed to create operator package generation {}: {error}",
            generation_root.display()
        )
    })?;
    let marker = GenerationMarker {
        schema_version: GENERATION_MARKER_SCHEMA.to_string(),
        generation_id,
        session_id: session.session_id().to_string(),
    };
    if let Err(error) = write_marker(&generation_root, &marker) {
        let _ = fs::remove_dir_all(&generation_root);
        return Err(error);
    }
    if let Err(error) = fs::create_dir(generation_root.join("packages")) {
        let _ = fs::remove_dir_all(&generation_root);
        return Err(format!(
            "failed to create operator package generation store layout: {error}"
        ));
    }
    let prepared = PreparedOperatorPackageGeneration {
        owned: OwnedOperatorPackageGeneration {
            generation_root,
            generations_root,
            marker,
            session,
        },
        cleanup_armed: true,
    };

    for receipt in active.installed_packages {
        if excluded_package_ids.contains(&receipt.package_id) {
            continue;
        }
        let source = active_store_root.join(&receipt.relative_root);
        install_operator_package_into(&source, prepared.store_root()).map_err(|error| {
            format!(
                "failed to carry package {} into the next Agent generation: {error}",
                receipt.package_id
            )
        })?;
    }
    Ok(prepared)
}

impl PreparedOperatorPackageGeneration {
    pub(crate) fn store_root(&self) -> &Path {
        &self.owned.generation_root
    }

    pub(crate) fn packages_root(&self) -> PathBuf {
        self.store_root().join("packages")
    }

    pub(crate) fn generation_id(&self) -> &str {
        &self.owned.marker.generation_id
    }

    pub(crate) fn session_id(&self) -> &str {
        self.owned.session.session_id()
    }

    pub(crate) fn janitor_report(&self) -> GenerationJanitorReport {
        self.owned.session.janitor_report()
    }

    pub(crate) fn commit(mut self) -> OwnedOperatorPackageGeneration {
        self.cleanup_armed = false;
        self.owned.clone()
    }
}

impl OwnedOperatorPackageGeneration {
    pub(crate) fn generation_id(&self) -> &str {
        &self.marker.generation_id
    }

    pub(crate) fn session_id(&self) -> &str {
        self.session.session_id()
    }

    pub(crate) fn janitor_report(&self) -> GenerationJanitorReport {
        self.session.janitor_report()
    }
}

impl Drop for PreparedOperatorPackageGeneration {
    fn drop(&mut self) {
        if self.cleanup_armed {
            let _ = remove_owned_operator_package_generation(&self.owned);
        }
    }
}

pub(crate) fn remove_owned_operator_package_generation(
    generation: &OwnedOperatorPackageGeneration,
) -> Result<(), String> {
    match generation.generation_root.symlink_metadata() {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "owned operator package generation must not be a symlink: {}",
                generation.generation_root.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "failed to inspect owned operator package generation {}: {error}",
                generation.generation_root.display()
            ));
        }
    }
    reject_symlink(
        &generation.generation_root,
        "owned operator package generation",
    )?;
    let parent = generation
        .generation_root
        .parent()
        .ok_or_else(|| "owned operator package generation has no parent".to_string())?;
    let canonical_parent = canonical_directory(parent, "operator package generations root")?;
    if canonical_parent != generation.generations_root
        || generation
            .generation_root
            .file_name()
            .and_then(|value| value.to_str())
            != Some(generation.marker.generation_id.as_str())
    {
        return Err("owned operator package generation path identity mismatch".to_string());
    }
    let marker = read_marker(&generation.generation_root)?;
    if marker != generation.marker {
        return Err("owned operator package generation marker mismatch".to_string());
    }
    fs::remove_dir_all(&generation.generation_root).map_err(|error| {
        format!(
            "failed to remove retired operator package generation {}: {error}",
            generation.generation_root.display()
        )
    })?;
    Ok(())
}

fn next_generation_id() -> Result<String, String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!("{}-{nonce}-{counter}", std::process::id()))
}

fn write_marker(root: &Path, marker: &GenerationMarker) -> Result<(), String> {
    let path = root.join(GENERATION_MARKER_FILE);
    let payload = serde_json::to_vec_pretty(marker).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    file.write_all(&payload)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("failed to persist {}: {error}", path.display()))
}

fn read_marker(root: &Path) -> Result<GenerationMarker, String> {
    let path = root.join(GENERATION_MARKER_FILE);
    reject_symlink(&path, "operator package generation marker")?;
    let metadata = path
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_MARKER_BYTES {
        return Err("operator package generation marker has an invalid size".to_string());
    }
    let payload =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let marker: GenerationMarker = serde_json::from_slice(&payload)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    if marker.schema_version != GENERATION_MARKER_SCHEMA {
        return Err("unsupported operator package generation marker schema".to_string());
    }
    Ok(marker)
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    reject_symlink(path, label)?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("{label} must be a directory"));
    }
    Ok(canonical)
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), String> {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(format!("{label} must not be a symlink: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abandoned_prepared_generation_is_removed() {
        let root = temporary_root("prepared-drop");
        let packages = root.join("packages");
        fs::create_dir_all(&packages).expect("create empty packages root");
        let session = OperatorPackageGenerationSession::open(&root).expect("open session");
        let generation = prepare_operator_package_generation(session, &packages, "operator.next")
            .expect("prepare generation");
        let generation_root = generation.store_root().to_path_buf();
        assert!(generation_root.exists());
        assert!(generation.packages_root().is_dir());
        drop(generation);
        assert!(!generation_root.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn committed_generation_requires_its_ownership_marker_for_removal() {
        let root = temporary_root("marker-guard");
        let packages = root.join("packages");
        fs::create_dir_all(&packages).expect("create empty packages root");
        let session = OperatorPackageGenerationSession::open(&root).expect("open session");
        let owned = prepare_operator_package_generation(session, &packages, "operator.next")
            .expect("prepare generation")
            .commit();
        fs::write(owned.generation_root.join(GENERATION_MARKER_FILE), b"{}")
            .expect("tamper marker");
        assert!(remove_owned_operator_package_generation(&owned).is_err());
        assert!(owned.generation_root.exists());
        drop(owned);
        let _ = fs::remove_dir_all(root);
    }

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("kyuubiki-generation-{label}-{nonce}"))
    }
}
