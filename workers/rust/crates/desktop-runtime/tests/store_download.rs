use kyuubiki_desktop_runtime::store_download::{DownloadSelection, StoreDownloads};
use kyuubiki_desktop_runtime::store_identity::StoreIdentity;

#[test]
fn download_selection_is_explicit_and_rejects_paths_or_unknown_credentials() {
    let value = serde_json::json!({"kind":"operator","asset_id":"operator.demo","version":"1.0.0",
        "platform":"macos-aarch64","key_id":"download-test","key_sha256":"a".repeat(64)});
    let selection: DownloadSelection = serde_json::from_value(value.clone()).unwrap();
    selection.validate().unwrap();
    for (field, invalid) in [
        ("asset_id", "../secret"),
        ("version", "latest"),
        ("platform", "unknown"),
        ("key_sha256", "auto-trust"),
        ("kind", "runtime"),
    ] {
        let mut bad = value.clone();
        bad[field] = serde_json::json!(invalid);
        assert!(
            serde_json::from_value::<DownloadSelection>(bad)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    let mut bad = value;
    bad["access_token"] = serde_json::json!("must-not-cross-ui");
    assert!(serde_json::from_value::<DownloadSelection>(bad).is_err());
}

#[test]
fn an_unsigned_download_never_creates_a_grant() {
    let downloads = StoreDownloads::default();
    let identity = StoreIdentity::default();
    let selection = serde_json::from_value(
        serde_json::json!({"kind":"operator","asset_id":"operator.demo",
        "version":"1.0.0","platform":"macos-aarch64","key_id":"test","key_sha256":"a".repeat(64)}),
    )
    .unwrap();
    assert_eq!(
        downloads.prepare(&identity, selection).err(),
        Some("login_required")
    );
}
