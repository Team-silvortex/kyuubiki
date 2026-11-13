use super::*;

const ENTRYPOINT: &str = "target/debug/{lib_prefix}fetch_fixture.{lib_extension}";

#[test]
fn nested_entrypoint_stages_installs_and_cleans_up() {
    exercise_staging(false);
}

#[test]
fn canonical_temporary_root_preserves_portable_manifest_paths() {
    exercise_staging(true);
}

fn exercise_staging(canonical: bool) {
    let temporary = TestRoot::new();
    let base = if canonical {
        temporary.0.canonicalize().unwrap()
    } else {
        temporary.0.clone()
    };
    #[cfg(windows)]
    if canonical {
        assert!(base.to_string_lossy().starts_with(r"\\?\"));
        let native = base.join(expand_platform_library_template(ENTRYPOINT));
        let relative = native.strip_prefix(&base).unwrap();
        assert!(relative.to_string_lossy().contains('\\'));
        assert!(validate_relative_path(relative, "old post-join check").is_err());
    }
    let manifest = manifest(ENTRYPOINT);
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let download = DownloadedPackage::prepare_under(&base, &manifest, &bytes).unwrap();
    let staging = download.root.clone();
    assert_eq!(
        download.entrypoint,
        staging.join(expand_platform_library_template(ENTRYPOINT))
    );
    assert!(download.entrypoint.parent().unwrap().is_dir());
    assert!(!download.entrypoint.exists());
    assert_eq!(
        fs::read(staging.join(OPERATOR_PACKAGE_MANIFEST_FILE)).unwrap(),
        bytes
    );
    write_new(&download.entrypoint, b"verified fixture bytes").unwrap();
    let store = base.join("managed store");
    let receipt = install_operator_package_into(&staging, &store).unwrap();
    crate::verify_managed_operator_package(&store.join(&receipt.relative_root)).unwrap();
    drop(download);
    assert!(!staging.exists());
    assert!(store.join(receipt.relative_root).is_dir());
}

#[test]
fn invalid_wire_paths_are_rejected_before_staging_is_created() {
    let temporary = TestRoot::new();
    let base = temporary.0.canonicalize().unwrap();
    for entrypoint in [
        "",
        "../outside.dll",
        "target/../outside.dll",
        "./operator.dll",
        "/outside.dll",
        r"target\operator.dll",
        r"target\..\outside.dll",
        r"\\server\share\operator.dll",
    ] {
        let manifest = manifest(entrypoint);
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(validate_downloaded_manifest(&manifest, "fetch.fixture", "0.1.0").is_err());
        let error = DownloadedPackage::prepare_under(&base, &manifest, &bytes)
            .err()
            .expect("invalid entrypoint must fail");
        assert!(error.contains("strict portable relative path"), "{error}");
        assert_eq!(fs::read_dir(&base).unwrap().count(), 0, "{entrypoint}");
    }
}

fn manifest(entrypoint: &str) -> OperatorPackageManifest {
    serde_json::from_value(serde_json::json!({
        "schema_version": OPERATOR_PACKAGE_SCHEMA_VERSION,
        "sdk_api_version": OPERATOR_SDK_API_VERSION,
        "execution_abi": OPERATOR_JSON_ABI_SCHEMA_VERSION,
        "package_id": "fetch.fixture",
        "package_version": "0.1.0",
        "minimum_host_version": "1.15.0",
        "validation_status": "partial",
        "validation_notes": "Download staging path regression fixture.",
        "runtime": "rust_crate",
        "entrypoint": entrypoint,
        "operators": [{
            "operator_id": "extract.fetch_fixture",
            "kind": "extract",
            "entry_symbol": "run_fetch_fixture_json"
        }]
    }))
    .unwrap()
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{} path test", download_staging_name(nonce)));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
