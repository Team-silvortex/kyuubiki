use super::*;
use std::os::unix::fs::PermissionsExt;

thread_local! { static DENIAL: RefCell<Option<(&'static str, PathBuf)>> = const { RefCell::new(None) }; }

pub(super) fn checkpoint(stage: &str, root: &Path) {
    DENIAL.with_borrow(|denial| {
        if denial
            .as_ref()
            .is_some_and(|(selected, target)| *selected == stage && target == root)
        {
            fs::set_permissions(root.parent().unwrap(), fs::Permissions::from_mode(0o500)).unwrap();
        }
    });
}

struct Restore {
    path: PathBuf,
    permissions: fs::Permissions,
}
impl Drop for Restore {
    fn drop(&mut self) {
        DENIAL.replace(None);
        let _ = fs::set_permissions(&self.path, self.permissions.clone());
    }
}

#[test]
#[ignore = "requires non-root Unix directory permission enforcement"]
fn generation_cleanup_metadata_permission_failures_resume_without_replay() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for stage in ["publication", "marker-removed", "directory-removed"] {
        let fixture = Fixture::new(stage);
        let generation = fixture.prepared().commit();
        let root = &generation.generation_root;
        let parent = &generation.generations_root;
        let restore = Restore {
            path: parent.clone(),
            permissions: fs::metadata(parent).unwrap().permissions(),
        };
        if stage == "publication" {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
        } else {
            DENIAL.replace(Some((stage, root.clone())));
        }
        assert!(
            remove_owned_operator_package_generation(&generation).is_err(),
            "{stage}"
        );
        if stage == "publication" {
            assert!(!record(&generation).exists());
            assert!(root.join("first-removable").exists());
            assert!(root.join(GENERATION_MARKER_FILE).exists());
        } else {
            assert!(record(&generation).is_file());
            assert!(!root.join(GENERATION_MARKER_FILE).exists());
            assert_eq!(root.exists(), stage == "marker-removed");
        }
        drop(restore);
        remove_owned_operator_package_generation(&generation).unwrap();
        assert!(!root.exists());
        assert!(!record(&generation).exists());
        remove_owned_operator_package_generation(&generation).unwrap();
    }
}
