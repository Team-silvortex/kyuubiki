mod support;
use kyuubiki_installer::{
    OperatorReleaseArtifact, install_operator_package_into, prepare_operator_distribution,
    uninstall_operator_package_from,
};
use kyuubiki_operator_sdk::{current_platform_library_file_name, current_platform_target_id};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use support::operator_package::{exercise_live_agent_package_journey, template_package_root};

struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires release template cdylib for explicit native Rust target and local Agent ports"]
fn prepared_distribution_installs_executes_and_uninstalls_on_native_agent() {
    let platform = current_platform_target_id();
    let triple = match platform.as_str() {
        "macos-aarch64" => "aarch64-apple-darwin",
        "macos-x86_64" => "x86_64-apple-darwin",
        "linux-aarch64" => "aarch64-unknown-linux-gnu",
        "linux-x86_64" => "x86_64-unknown-linux-gnu",
        "windows-x86_64" => "x86_64-pc-windows-msvc",
        _ => panic!("unsupported live target {platform}"),
    };
    let template = template_package_root();
    let library = template.join("target").join(triple).join("release").join(
        current_platform_library_file_name("kyuubiki_operator_template"),
    );
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-release-live-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _cleanup = Cleanup(root.clone());
    let candidate = root.join("candidate");
    let index = prepare_operator_distribution(
        &template.join("kyuubiki-operator.json"),
        &[OperatorReleaseArtifact {
            rust_target: triple.into(),
            library_path: library,
        }],
        &candidate,
    )
    .unwrap();
    let receipt =
        install_operator_package_into(&candidate.join(platform), &root.join("store")).unwrap();
    assert_eq!(
        receipt.entrypoint_sha256,
        index.artifacts[0].entrypoint_sha256
    );
    assert_eq!(receipt.manifest_sha256, index.artifacts[0].manifest_sha256);
    exercise_live_agent_package_journey(&root.join("store/packages"), &receipt.entrypoint_sha256);
    let removal =
        uninstall_operator_package_from(&root.join("store"), &receipt.package_id).unwrap();
    assert!(removal.removed && removal.store_pruned);
}

#[test]
#[ignore = "requires an explicitly trusted native package returned from an isolated store test"]
fn returned_store_candidate_installs_executes_and_uninstalls_on_native_agent() {
    use kyuubiki_operator_sdk::read_operator_package_distribution;
    use support::operator_package::sha256_file;

    // This opt-in test loads code. Never point it at an unreviewed third-party upload.
    let candidate = PathBuf::from(
        std::env::var_os("KYUUBIKI_TEST_TRUSTED_STORE_CANDIDATE")
            .expect("set KYUUBIKI_TEST_TRUSTED_STORE_CANDIDATE to the trusted round-trip fixture"),
    );
    let index =
        read_operator_package_distribution(candidate.join("kyuubiki-operator-distribution.json"))
            .unwrap();
    assert_eq!(index.package_id, "operator.template.summary");
    assert_eq!(index.package_version, "0.1.0");
    let platform = current_platform_target_id();
    let artifact = index
        .artifacts
        .iter()
        .find(|a| a.target == platform)
        .unwrap();
    for (relative, digest, size) in [
        (
            &artifact.manifest_path,
            &artifact.manifest_sha256,
            artifact.manifest_size_bytes,
        ),
        (
            &artifact.entrypoint_path,
            &artifact.entrypoint_sha256,
            artifact.entrypoint_size_bytes,
        ),
    ] {
        let path = candidate.join(relative);
        assert_eq!(fs::metadata(&path).unwrap().len(), size);
        assert_eq!(&sha256_file(&path).unwrap(), digest);
    }
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-store-return-live-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _cleanup = Cleanup(root.clone());
    let receipt =
        install_operator_package_into(&candidate.join(platform), &root.join("store")).unwrap();
    assert_eq!(receipt.entrypoint_sha256, artifact.entrypoint_sha256);
    assert_eq!(receipt.manifest_sha256, artifact.manifest_sha256);
    exercise_live_agent_package_journey(&root.join("store/packages"), &receipt.entrypoint_sha256);
    let removal =
        uninstall_operator_package_from(&root.join("store"), &receipt.package_id).unwrap();
    assert!(removal.removed && removal.store_pruned);
}
