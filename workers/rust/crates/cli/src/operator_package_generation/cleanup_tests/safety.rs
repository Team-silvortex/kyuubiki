use super::*;

#[test]
fn ambiguous_generation_cleanup_records_never_authorize_payload_removal() {
    for scenario in [
        "invalid",
        "foreign",
        "oversized",
        "directory",
        "marker-conflict",
        "missing-marker",
    ] {
        let fixture = Fixture::new(scenario);
        let generation = fixture.prepared().commit();
        let root = &generation.generation_root;
        let failure = Failure::at("record-published", root);
        assert!(remove_owned_operator_package_generation(&generation).is_err());
        drop(failure);
        let journal = record(&generation);
        match scenario {
            "invalid" => fs::write(&journal, b"invalid").unwrap(),
            "foreign" => {
                let mut changed = generation.marker.clone();
                changed.session_id.push_str("-foreign");
                fs::write(&journal, serde_json::to_vec(&changed).unwrap()).unwrap();
            }
            "oversized" => fs::write(&journal, vec![b'x'; MAX_MARKER_BYTES as usize + 1]).unwrap(),
            "directory" => {
                fs::remove_file(&journal).unwrap();
                fs::create_dir(&journal).unwrap();
            }
            "marker-conflict" => {
                fs::remove_file(root.join(GENERATION_MARKER_FILE)).unwrap();
                fs::write(root.join(GENERATION_MARKER_FILE), b"{}").unwrap();
            }
            _ => fs::remove_file(root.join(GENERATION_MARKER_FILE)).unwrap(),
        }
        assert!(
            remove_owned_operator_package_generation(&generation).is_err(),
            "{scenario}"
        );
        assert_eq!(
            fs::read(root.join("packages/locked/remaining")).unwrap(),
            b"keep"
        );
        assert!(journal.exists());
        assert!(root.join("first-removable").is_file());
    }
}

#[test]
fn generation_cleanup_does_not_adopt_legacy_missing_identity() {
    let fixture = Fixture::new("legacy");
    let generation = fixture.prepared().commit();
    fs::remove_file(generation.generation_root.join(GENERATION_MARKER_FILE)).unwrap();
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    assert!(!record(&generation).exists());
    assert!(
        generation
            .generation_root
            .join("packages/locked/remaining")
            .exists()
    );
}

#[test]
fn missing_generation_root_does_not_authorize_removing_a_foreign_record() {
    let fixture = Fixture::new("missing-root");
    let generation = fixture.prepared().commit();
    let failure = Failure::at("directory-removed", &generation.generation_root);
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    drop(failure);
    assert!(!generation.generation_root.exists());
    fs::write(record(&generation), b"foreign record").unwrap();
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    assert_eq!(fs::read(record(&generation)).unwrap(), b"foreign record");
}

#[test]
fn retired_generation_blocks_late_downloader_but_not_a_healthy_sibling() {
    use crate::operator_package_generation_session::acquire_download_session_lease;
    let fixture = Fixture::new("downloader");
    let generation = fixture.prepared().commit();
    let failure = Failure::at("record-published", &generation.generation_root);
    assert!(remove_owned_operator_package_generation(&generation).is_err());
    drop(failure);
    let error = acquire_download_session_lease(&generation.generation_root).unwrap_err();
    assert!(error.contains("generation is retiring"), "{error}");
    let sibling = fixture.prepared();
    let lease = acquire_download_session_lease(sibling.store_root()).unwrap();
    remove_owned_operator_package_generation(&generation).unwrap();
    assert!(sibling.packages_root().join("locked/remaining").is_file());
    drop(lease);
    drop(sibling);
}

#[cfg(unix)]
#[test]
fn generation_cleanup_rejects_symlink_identity_and_unlinks_only_nested_links() {
    for target in ["root", "marker", "record", "nested"] {
        let fixture = Fixture::new(target);
        let generation = fixture.prepared().commit();
        let root = &generation.generation_root;
        let outside = fixture.root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), b"outside").unwrap();
        let path = match target {
            "root" => {
                fs::remove_dir_all(root).unwrap();
                root.clone()
            }
            "marker" => {
                let marker = root.join(GENERATION_MARKER_FILE);
                fs::remove_file(&marker).unwrap();
                marker
            }
            "record" => record(&generation),
            _ => root.join("packages/outside"),
        };
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        let result = remove_owned_operator_package_generation(&generation);
        assert_eq!(result.is_ok(), target == "nested", "{target}: {result:?}");
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"outside");
        if target != "nested" {
            assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
        }
    }
}
