use super::{
    ReplacementRule, VersionKind, extract_local_hrefs, required_snippets, semver_major,
    semver_minor, sync_replacements,
};

#[test]
fn extracts_local_hrefs_like_the_legacy_checker() {
    let hrefs = extract_local_hrefs(
        r##"<a href="./book.html">book</a><a href="#local">local</a><a href="https://example.invalid">remote</a>"##,
    );
    assert_eq!(hrefs, vec!["./book.html"]);
}

#[test]
fn required_snippets_cover_hub_mirrors() {
    assert!(required_snippets("apps/hub-gui/ui/docs/index.html").contains(&"Open central book"));
    assert!(required_snippets("docs/book-ch08-reading-paths.html").contains(&"docs/README.md"));
}

#[test]
fn replacement_rule_updates_semver_tokens() {
    let rule = ReplacementRule {
        prefix: "Current line: moxi ".to_string(),
        suffix: "<",
        replacement: "2.0.0".to_string(),
        version_kind: VersionKind::Semver,
    };
    assert_eq!(
        rule.apply("Current line: moxi 1.19.0<"),
        "Current line: moxi 2.0.0<"
    );
}

#[test]
fn replacement_rule_updates_product_line_tokens() {
    let rule = ReplacementRule {
        prefix: "Current line: moxi ".to_string(),
        suffix: "<",
        replacement: "3.x".to_string(),
        version_kind: VersionKind::Line,
    };
    assert_eq!(
        rule.apply("Current line: moxi 2.x<"),
        "Current line: moxi 3.x<"
    );
}

#[test]
fn sync_rules_update_every_version_surface() {
    let replacements = sync_replacements("2.2.8", "moxi 2.x", "2.2");
    let apply = |path: &str, text: &str| {
        replacements
            .iter()
            .find(|(candidate, _)| *candidate == path)
            .unwrap()
            .1
            .iter()
            .fold(text.to_string(), |next, rule| rule.apply(&next))
    };
    assert_eq!(
        apply(
            "docs/book.html",
            "Version line: moxi 2.x; Current development: 2.0.0"
        ),
        "Version line: moxi 2.x; Current development: 2.2.8"
    );
    assert_eq!(
        apply(
            "apps/hub-gui/ui/docs/installation-integrity.html",
            "Shipping version: 2.0.0"
        ),
        "Shipping version: 2.2.8"
    );
    assert_eq!(
        apply(
            "apps/hub-gui/ui/docs/current-line.html",
            "<h1>moxi 2.x</h1>The current development point is <code>moxi 2.x</code>. The line began at <code>moxi 2.0.0</code>."
        ),
        "<h1>moxi 2.x</h1>The current development point is <code>moxi 2.2.8</code>. The line began at <code>moxi 2.0.0</code>."
    );
}

#[test]
fn semver_minor_keeps_major_and_minor() {
    assert_eq!(semver_major("2.2.8").unwrap(), "2");
    assert_eq!(semver_minor("1.20.0").unwrap(), "1.20");
}

#[test]
fn daji_patch_sync_preserves_historical_versions() {
    let rules = sync_replacements("3.0.1", "daji 3.0.1", "3.0");
    let (_, rules) = rules
        .iter()
        .find(|(path, _)| *path == "docs/book.html")
        .unwrap();
    let text = rules.iter().fold(
        "One book for daji 3.0.0; historical moxi 2.20.1".to_string(),
        |text, rule| rule.apply(&text),
    );
    assert_eq!(text, "One book for daji 3.0.1; historical moxi 2.20.1");
}
