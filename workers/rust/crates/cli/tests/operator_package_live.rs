mod support;

use support::operator_package::{
    exercise_live_agent_package_journey, sha256_file, template_library_path, template_packages_root,
};

#[test]
#[ignore = "requires prebuilt operator template cdylib"]
fn live_agent_loads_executes_rejects_tamper_and_recovers() {
    let packages_root = template_packages_root();
    let library = template_library_path(&packages_root);
    assert!(
        library.is_file(),
        "prebuilt cdylib missing: {}",
        library.display()
    );
    let entrypoint_sha256 = sha256_file(&library).expect("hash template cdylib");

    exercise_live_agent_package_journey(&packages_root, &entrypoint_sha256);
}
