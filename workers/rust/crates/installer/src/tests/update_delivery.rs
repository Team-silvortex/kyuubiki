use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::update_source::{
    apply_downloaded_update_at, download_update_at, validate_update_source_config,
};
use crate::{Platform, UnifiedUpdatePlan, UpdateArtifactRef, UpdateSourceConfig};

struct Fixture {
    root: PathBuf,
}

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-update-delivery-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("source/payloads")).unwrap();
        fs::write(root.join("source/payloads/engine.bin"), b"verified-engine").unwrap();
        Self { root }
    }

    fn config(&self) -> UpdateSourceConfig {
        UpdateSourceConfig {
            schema_version: "kyuubiki.update-source/v1".to_string(),
            catalog_path: "releases/update-catalog.json".to_string(),
            artifact_root: "source".to_string(),
            download_dir: "downloads".to_string(),
        }
    }

    fn plan(&self) -> UnifiedUpdatePlan {
        UnifiedUpdatePlan {
            schema_version: "kyuubiki.update-catalog/v1".to_string(),
            workspace: ".".to_string(),
            current_version: "2.7.0".to_string(),
            target_channel: "stable".to_string(),
            target_tag: "moxi:stable".to_string(),
            target_version: "2.7.1".to_string(),
            update_state: "update_available".to_string(),
            summary: "test update".to_string(),
            contract_rules: Vec::new(),
            artifacts: vec![UpdateArtifactRef {
                product: "agent".to_string(),
                platform: "linux".to_string(),
                kind: "binary".to_string(),
                path: "payloads/engine.bin".to_string(),
                exists: true,
            }],
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn downloads_and_applies_digest_verified_artifacts() {
    let fixture = Fixture::new();
    let config = fixture.config();
    let downloaded =
        download_update_at(&fixture.root, &config, &fixture.plan(), Platform::Linux).unwrap();

    assert_eq!(downloaded.downloaded_paths.len(), 1);
    assert!(downloaded.download_dir.starts_with("downloads/"));
    assert!(
        !downloaded
            .manifest_path
            .contains(&fixture.root.to_string_lossy().to_string())
    );
    let download_manifest = read_json(&fixture.root.join(&downloaded.manifest_path));
    assert_eq!(
        download_manifest["artifacts"][0]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );

    let applied = apply_downloaded_update_at(&fixture.root, &config, &downloaded).unwrap();
    let applied_manifest = read_json(&fixture.root.join(&applied.manifest_path));
    let applied_path = applied_manifest["artifacts"][0]["applied_path"]
        .as_str()
        .unwrap();
    assert_eq!(
        fs::read(fixture.root.join(applied_path)).unwrap(),
        b"verified-engine"
    );
    assert_eq!(
        applied_manifest["artifacts"][0]["sha256"],
        download_manifest["artifacts"][0]["sha256"]
    );
}

#[test]
fn rejects_tampered_download_before_apply() {
    let fixture = Fixture::new();
    let config = fixture.config();
    let downloaded =
        download_update_at(&fixture.root, &config, &fixture.plan(), Platform::Linux).unwrap();
    fs::write(
        fixture.root.join(&downloaded.downloaded_paths[0]),
        b"tampered-engine",
    )
    .unwrap();

    let error = apply_downloaded_update_at(&fixture.root, &config, &downloaded).unwrap_err();
    assert!(error.contains("digest mismatch"));
    assert!(
        !fixture
            .root
            .join("downloads/latest-applied-update.json")
            .exists()
    );
}

#[test]
fn rejects_download_pointer_directory_rebinding() {
    let fixture = Fixture::new();
    let config = fixture.config();
    let mut downloaded =
        download_update_at(&fixture.root, &config, &fixture.plan(), Platform::Linux).unwrap();
    downloaded.download_dir = "downloads/applied/stable-2.7.1".to_string();

    let error = apply_downloaded_update_at(&fixture.root, &config, &downloaded).unwrap_err();
    assert!(error.contains("configured version directory"));
    assert!(fixture.root.join(&downloaded.downloaded_paths[0]).exists());
}

#[test]
fn rejects_catalog_artifact_path_escape_before_download() {
    let fixture = Fixture::new();
    let config = fixture.config();
    let mut plan = fixture.plan();
    plan.artifacts[0].path = "../outside.bin".to_string();

    let error = download_update_at(&fixture.root, &config, &plan, Platform::Linux).unwrap_err();
    assert!(error.contains("catalog artifact path is not controlled"));
    assert!(!fixture.root.join("downloads/stable-2.7.1").exists());
}

#[test]
fn rejects_unmanaged_download_directory_and_schema() {
    let fixture = Fixture::new();
    let mut config = fixture.config();
    config.download_dir = "../outside".to_string();
    assert!(validate_update_source_config(&config).is_err());

    config = fixture.config();
    config.schema_version = "kyuubiki.update-source/v0".to_string();
    assert!(validate_update_source_config(&config).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_managed_download_root() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    fs::create_dir_all(fixture.root.join("outside")).unwrap();
    symlink(fixture.root.join("outside"), fixture.root.join("downloads")).unwrap();

    let error = download_update_at(
        &fixture.root,
        &fixture.config(),
        &fixture.plan(),
        Platform::Linux,
    )
    .unwrap_err();
    assert!(error.contains("crosses symlink"));
}
