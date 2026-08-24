mod support;

use kyuubiki_installer::{
    install_operator_package_into, managed_operator_package_status_in,
    uninstall_operator_package_from,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use support::operator_package::{exercise_live_agent_package_journey, template_package_root};

#[test]
#[ignore = "requires prebuilt operator template cdylib"]
fn installer_manages_agent_package_execution_and_residue_free_removal() {
    let work_root = temp_dir("operator-package-installer-live");
    let cleanup = Cleanup(work_root.clone());
    let store_root = work_root.join("managed-store");
    let receipt = install_operator_package_into(&template_package_root(), &store_root)
        .expect("Installer should install template package");
    let status = managed_operator_package_status_in(&store_root)
        .expect("Installer should report managed package");
    assert_eq!(status.installed_package_count, 1);
    assert_eq!(status.installed_packages, vec![receipt.clone()]);

    exercise_live_agent_package_journey(
        &PathBuf::from(&status.packages_root),
        &receipt.entrypoint_sha256,
    );

    let removal = uninstall_operator_package_from(&store_root, &receipt.package_id)
        .expect("Installer should remove managed package");
    assert!(removal.removed);
    assert!(removal.store_pruned);
    assert!(!store_root.exists());
    drop(cleanup);
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("kyuubiki-{label}-{nonce}"))
}
