use super::{contains_todo, last_quoted_value, version_allowed, version_minor_line};
use serde_json::json;

#[test]
fn minor_line_keeps_major_minor() {
    assert_eq!(version_minor_line("1.20.0"), "1.20.x");
    assert_eq!(version_minor_line("2"), "2.x");
}

#[test]
fn todo_scan_recurses() {
    assert!(contains_todo(&json!({"nested": ["TODO: fill"]})));
    assert!(!contains_todo(&json!({"nested": ["done"]})));
}

#[test]
fn mix_version_uses_the_environment_default() {
    assert_eq!(
        last_quoted_value(r#"version: System.get_env("KYUUBIKI_RELEASE_VERSION", "2.17.0"),"#),
        Some("2.17.0")
    );
}

#[test]
fn active_major_version_space_is_bounded() {
    assert!(version_allowed("3.0.0", 3, 20, 9));
    assert!(version_allowed("3.20.9", 3, 20, 9));
    assert!(!version_allowed("3.21.0", 3, 20, 9));
    assert!(!version_allowed("3.0.10", 3, 20, 9));
    assert!(!version_allowed("2.20.9", 3, 20, 9));
    assert!(version_allowed("2.0.0", 2, 20, 9));
    assert!(version_allowed("2.11.7", 2, 20, 9));
    assert!(version_allowed("2.20.9", 2, 20, 9));
    assert!(!version_allowed("2.20.10", 2, 20, 9));
    assert!(!version_allowed("2.21.0", 2, 20, 9));
    assert!(!version_allowed("3.0.0", 2, 20, 9));
}

#[test]
fn current_policy_tracks_brand_and_book_instead_of_a_fixed_major() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let book = super::read_json(&root, "docs/book-manifest.json").unwrap();
    let brand = super::read_json(&root, "assets/brand/brand.json").unwrap();
    let version = super::field(&book, "current_development_version");
    let codename = super::field(&brand, "releaseCodename");
    let checks = super::version_policy_checks(&root, version, codename).unwrap();
    assert!(checks.iter().all(|check| check["ok"] == true), "{checks:?}");
    let mismatched = super::version_policy_checks(&root, version, "wrong-line").unwrap();
    assert!(
        mismatched
            .iter()
            .any(|check| { check["kind"] == "current_development_line" && check["ok"] == false })
    );
}
