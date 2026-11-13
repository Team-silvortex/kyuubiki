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

#[test]
#[ignore = "requires built native Installer; loopback TLS and inert ELF header fixtures, never load or execute"]
fn signed_linux_download_stages_via_native_installer_and_survives_logout() {
    for platform in ["linux-x86_64", "linux-aarch64"] {
        let rust = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let binary = rust.join("target/debug/kyuubiki-installer");
        assert!(binary.is_file());
        let mut manifest: Value = serde_json::from_slice(
            &fs::read(rust.join("templates/operator-crate-template/kyuubiki-operator.json"))
                .unwrap(),
        )
        .unwrap();
        manifest["entrypoint"] = json!("operator.so");
        let mut elf = vec![0; 64];
        elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        elf[16..18].copy_from_slice(&3_u16.to_le_bytes());
        elf[18..20].copy_from_slice(
            &(if platform == "linux-aarch64" {
                183_u16
            } else {
                62
            })
            .to_le_bytes(),
        );
        elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
        elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
        let fixture = Fixture::with_native_files(Some(vec![
            (
                format!("{platform}/kyuubiki-operator.json"),
                serde_json::to_vec(&manifest).unwrap(),
            ),
            (format!("{platform}/operator.so"), elf.clone()),
        ]));
        fixture.store.lock().unwrap().selection.platform = platform.into();
        let identity = login(&fixture);
        let disk = Sandbox::new();
        let staged = Sandbox::new();
        let mut manager = disk.manager();
        manager.test_installer = Some((binary.clone(), staged.0.clone()));
        let selection = fixture.store.lock().unwrap().selection.clone();
        let preview = manager.prepare(&identity, selection).unwrap();
        start(&manager, &identity, &preview.id);
        finished(&manager, &identity, "downloaded");
        let stage = || DownloadRequest::StageVariant {
            id: preview.id.clone(),
        };
        assert_eq!(
            manager.handle(&identity, stage()).unwrap()[0]
                .installation
                .as_deref(),
            Some("variant_staged")
        );
        let root = staged.0.join(format!(
            "variants/{platform}/packages/operator.template.summary"
        ));
        assert_eq!(fs::read(root.join("operator.so")).unwrap(), elf);
        assert!(!staged.0.join("packages").exists());
        let mut saved = disk.record(&preview.id);
        saved["installation"] = json!("variant_staging");
        fs::write(
            disk.0.join(&preview.id).join("record.json"),
            serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
        let mut reopened = disk.manager();
        reopened.test_installer = manager.test_installer.clone();
        assert_eq!(
            reopened.handle(&identity, stage()).unwrap()[0]
                .installation
                .as_deref(),
            Some("variant_staged")
        );
        fixture.store.lock().unwrap().recalled = true;
        assert_eq!(
            reopened.handle(&identity, stage()).err(),
            Some("release_recalled")
        );
        assert_eq!(fixture.store.lock().unwrap().charges, 1);
        assert_eq!(fixture.store.lock().unwrap().renewals, 0);
        identity.logout();
        // Project control is local intent: neither an account lease nor Store requests are used.
        let request = |operation| json!({"project_id":"research","scope":"a".repeat(64),"platform":platform,"operation":operation});
        let list = crate::native_installer::invoke(
            &binary,
            "project-operator",
            None,
            &request(json!({"action":"list"})),
            Some(&staged.0),
        )
        .unwrap();
        assert_eq!(list["platform"], platform);
        let pin = list["installed"][0].clone();
        assert!(!pin.is_null());
        crate::native_installer::invoke(
            &binary,
            "project-operator",
            None,
            &request(json!({"action":"enable","package":pin})),
            Some(&staged.0),
        )
        .unwrap();
        let chunk = crate::native_installer::invoke(
            &binary,
            "project-operator",
            None,
            &request(json!({"action":"read_deployment","package":pin,"offset":0})),
            Some(&staged.0),
        )
        .unwrap();
        assert_eq!(chunk["deployment"]["spec"]["platform"], platform);
        assert_eq!(chunk["deployment"]["data"], json!(elf));
    }
}
