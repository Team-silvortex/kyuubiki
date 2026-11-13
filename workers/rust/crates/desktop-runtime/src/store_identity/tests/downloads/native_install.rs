use super::*;
use crate::store_download::{CatalogRequest, catalog};
use std::fs;

#[test]
#[ignore = "requires built native Installer and the trusted template cdylib; loopback TLS only, no activation"]
fn catalog_grant_real_native_install_recovery_and_cache_removal() {
    let rust = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = rust.join("target/debug/kyuubiki-installer");
    assert!(binary.is_file(), "build -p kyuubiki-installer first");
    let template = rust.join("templates/operator-crate-template");
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let (filename, library) = if cfg!(target_os = "macos") {
        ("operator.dylib", "libkyuubiki_operator_template.dylib")
    } else {
        ("operator.so", "libkyuubiki_operator_template.so")
    };
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(template.join("kyuubiki-operator.json")).unwrap())
            .unwrap();
    manifest["entrypoint"] = json!(filename);
    let library = fs::read(template.join("target/debug").join(library))
        .expect("build trusted template cdylib first");
    let fixture = Fixture::with_native_files(Some(vec![
        (
            format!("{platform}/kyuubiki-operator.json"),
            serde_json::to_vec(&manifest).unwrap(),
        ),
        (format!("{platform}/{filename}"), library.clone()),
    ]));
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let installed = Sandbox::new();
    let mut manager = disk.manager();
    manager.test_installer = Some((binary, installed.0.clone()));
    let page = catalog(
        &identity,
        CatalogRequest {
            query: "template".into(),
            cursor: None,
        },
    )
    .unwrap();
    let mut selection = fixture.store.lock().unwrap().selection.clone();
    selection.asset_id = page.items[0].asset_id.clone();
    selection.version = page.items[0].latest_version.clone();
    selection.platform = page.host_platform;
    let prepared = manager.prepare(&identity, selection).unwrap();
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
    assert_eq!(
        manager
            .handle(
                &identity,
                DownloadRequest::Install {
                    id: prepared.id.clone()
                }
            )
            .err(),
        Some("operator_download_required")
    );
    start(&manager, &identity, &prepared.id);
    let downloaded = finished(&manager, &identity, "downloaded");
    let result = manager
        .handle(
            &identity,
            DownloadRequest::Install {
                id: prepared.id.clone(),
            },
        )
        .unwrap();
    assert_eq!(result[0].installation.as_deref(), Some("installed"));
    let target = installed.0.join("packages/operator.template.summary");
    assert_eq!(fs::read(target.join(filename)).unwrap(), library);
    let receipt = fs::read(target.join("kyuubiki-managed-install.json")).unwrap();
    // Simulate a lost desktop completion receipt after Installer has already committed.
    let mut record = disk.record(&prepared.id);
    record["installation"] = json!("installing");
    fs::write(
        disk.0.join(&prepared.id).join("record.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    let mut reopened = disk.manager();
    reopened.test_installer = manager.test_installer.clone();
    assert_eq!(
        reopened
            .handle(
                &identity,
                DownloadRequest::Install {
                    id: prepared.id.clone()
                }
            )
            .unwrap()[0]
            .installation
            .as_deref(),
        Some("installed")
    );
    assert_eq!(
        fs::read(target.join("kyuubiki-managed-install.json")).unwrap(),
        receipt
    );
    let cached = PathBuf::from(downloaded.directory.unwrap())
        .join(&platform)
        .join(filename);
    fs::write(&cached, b"tampered after download").unwrap();
    assert_eq!(
        reopened
            .handle(
                &identity,
                DownloadRequest::Install {
                    id: prepared.id.clone()
                }
            )
            .err(),
        Some("native_install_check_required")
    );
    assert_eq!(fs::read(target.join(filename)).unwrap(), library);
    fs::write(cached, &library).unwrap();
    fixture.store.lock().unwrap().recalled = true;
    assert_eq!(
        reopened
            .handle(
                &identity,
                DownloadRequest::Install {
                    id: prepared.id.clone()
                }
            )
            .err(),
        Some("release_recalled")
    );
    assert_eq!(fixture.store.lock().unwrap().charges, 1);
    assert_eq!(fixture.store.lock().unwrap().renewals, 0);
    reopened
        .handle(&identity, DownloadRequest::Remove { id: prepared.id })
        .unwrap();
    assert!(reopened.list(&identity).unwrap().is_empty());
    assert_eq!(
        fs::read(target.join("kyuubiki-managed-install.json")).unwrap(),
        receipt
    );
}
