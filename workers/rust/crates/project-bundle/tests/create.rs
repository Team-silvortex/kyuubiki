use kyuubiki_project_bundle::{create_project_bundle_in, read_project_bundle};
use serde_json::Value;
use std::fs;

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("kyuubiki-create-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&path).unwrap();
    path
}

struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn creates_an_empty_bundle_in_the_chosen_existing_directory() {
    let root = Cleanup(directory());
    for name in ["Research", "mixed materials", "材料研究", "sample.KYUUBIKI"] {
        let receipt: Value = serde_json::from_str(
            &create_project_bundle_in(root.0.to_str().unwrap(), name).unwrap(),
        )
        .unwrap();
        let path = receipt["path"].as_str().unwrap();
        assert_eq!(std::path::Path::new(path).parent(), Some(root.0.as_path()));
        let bundle = read_project_bundle(path).unwrap();
        assert!(bundle["models"].as_array().unwrap().is_empty());
        assert!(bundle["model_versions"].as_array().unwrap().is_empty());
        assert_eq!(
            bundle["project"]["name"],
            name.strip_suffix(".KYUUBIKI").unwrap_or(name)
        );
    }
}

#[test]
fn rejects_portability_and_traversal_errors_without_creating_anything() {
    let root = Cleanup(directory());
    for name in [
        "",
        " ",
        ".",
        "..",
        "../escape",
        "sub/name",
        "sub\\name",
        "bad:name",
        "bad?name",
        "bad*name",
        "bad\"name",
        "bad<name",
        "bad>name",
        "bad|name",
        "bad\0name",
        "bad\nname",
        "CON",
        "con.txt",
        "NUL",
        "aux",
        "PRN",
        "COM1",
        "LPT9",
        "trailing.",
        "space .kyuubiki",
        ".kyuubiki",
    ] {
        assert!(
            create_project_bundle_in(root.0.to_str().unwrap(), name).is_err(),
            "accepted {name:?}"
        );
    }
    assert!(create_project_bundle_in(root.0.to_str().unwrap(), &"x".repeat(201)).is_err());
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
}

#[test]
fn rejects_missing_relative_or_file_parent_without_making_directories() {
    let root = Cleanup(directory());
    assert!(create_project_bundle_in(".", "Research").is_err());
    assert!(create_project_bundle_in("", "Research").is_err());
    let missing = root.0.join("not-created");
    assert!(create_project_bundle_in(missing.to_str().unwrap(), "Research").is_err());
    assert!(!missing.exists());
    let file = root.0.join("file");
    fs::write(&file, b"unchanged").unwrap();
    assert!(create_project_bundle_in(file.to_str().unwrap(), "Research").is_err());
    assert_eq!(fs::read(file).unwrap(), b"unchanged");
}

#[test]
fn existing_bundle_is_never_overwritten() {
    let root = Cleanup(directory());
    let path = root.0.join("Research.kyuubiki");
    fs::write(&path, b"existing unrelated content").unwrap();
    let error = create_project_bundle_in(root.0.to_str().unwrap(), "Research").unwrap_err();
    assert!(error.contains("refusing to overwrite"));
    assert_eq!(fs::read(path).unwrap(), b"existing unrelated content");
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
}

#[test]
#[cfg(unix)]
fn chooser_directory_is_not_silently_trimmed_into_a_different_directory() {
    let root = Cleanup(directory());
    let selected = root.0.join("Research ");
    let other = root.0.join("Research");
    fs::create_dir(&selected).unwrap();
    fs::create_dir(&other).unwrap();
    create_project_bundle_in(selected.to_str().unwrap(), "Study").unwrap();
    assert!(selected.join("Study.kyuubiki").is_file());
    assert_eq!(fs::read_dir(&other).unwrap().count(), 0);
}
