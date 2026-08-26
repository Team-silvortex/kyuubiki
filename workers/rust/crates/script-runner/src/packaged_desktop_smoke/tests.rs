use super::*;

#[test]
fn parses_smoke_options() {
    let root = Path::new("/tmp/repo");
    let options = parse_options(
        root,
        [
            OsString::from("macos"),
            OsString::from("--timeout-secs"),
            OsString::from("40"),
            OsString::from("--out"),
            OsString::from("tmp/report.json"),
        ]
        .to_vec(),
    )
    .expect("options should parse");
    assert_eq!(options.platform, Platform::Macos);
    assert_eq!(options.timeout_secs, 40);
    assert_eq!(options.output_path, root.join("tmp/report.json"));
    assert_eq!(
        options.bundle_root,
        root.join("target/desktop-cache/macos/release/bundle/macos")
    );
    assert!(options.verify_report.is_none());
    let linux = parse_options(root, vec![OsString::from("linux")]).expect("linux options");
    assert_eq!(linux.platform, Platform::Linux);
    assert_eq!(linux.bundle_root, Path::new("/usr/bin"));
    let windows = parse_options(
        root,
        [
            OsString::from("windows"),
            OsString::from("--install-nsis"),
            OsString::from("--bundle-root"),
            OsString::from("/tmp/windows-apps"),
        ]
        .to_vec(),
    )
    .expect("Windows options");
    assert_eq!(windows.platform, Platform::Windows);
    assert_eq!(windows.bundle_root, Path::new("/tmp/windows-apps"));
    assert!(windows.install_nsis);
}

#[test]
fn rejects_invalid_timeout() {
    let error = parse_options(
        Path::new("/tmp/repo"),
        [OsString::from("--timeout-secs"), OsString::from("0")].to_vec(),
    )
    .err()
    .expect("zero timeout should fail");
    assert!(error.contains("between 1 and 300"));
}

#[test]
fn report_paths_are_portable() {
    let root = Path::new("/tmp/repo");
    assert_eq!(
        portable_path(root, &root.join("tmp/report.json")),
        "tmp/report.json"
    );
    assert_eq!(
        portable_path(root, Path::new("/Applications")),
        "@external/Applications"
    );
    assert_eq!(
        portable_bundle_path(
            root,
            Path::new("/Applications"),
            Path::new("/Applications/Kyuubiki Hub.app/Contents/MacOS/kyuubiki-hub-gui")
        ),
        "@bundle-root/Kyuubiki Hub.app/Contents/MacOS/kyuubiki-hub-gui"
    );
    assert_eq!(
        portable_detail(
            root,
            Path::new("/Applications"),
            "failed at /tmp/repo/target/app"
        ),
        "failed at ./target/app"
    );
}

#[test]
fn validates_portable_retained_report() {
    let mut report = retained_report(Platform::Macos, "@external/Applications");
    validate_retained_report(&report).expect("portable report should pass");
    report["surfaces"][0]["detail"] = Value::String("startup assumed".to_string());
    let error = validate_retained_report(&report).expect_err("invalid receipt should fail");
    assert!(error.contains("/detail"));
    report["surfaces"][0] = retained_surface("hub", 1, Platform::Macos);
    report["surfaces"][0]["app_path"] = Value::String("/Applications/Hub.app".to_string());
    let error = validate_retained_report(&report).expect_err("absolute path should fail");
    assert!(error.contains("must be portable"));

    let linux = retained_report(Platform::Linux, "@external/bin");
    validate_retained_report(&linux).expect("portable Linux report should pass");
    let windows = retained_report(Platform::Windows, "@external/LocalAppData");
    validate_retained_report(&windows).expect("portable Windows report should pass");
}

fn retained_report(platform: Platform, bundle_root: &str) -> Value {
    serde_json::json!({
        "schema_version": REPORT_SCHEMA,
        "platform": platform.as_str(),
        "bundle_root": bundle_root,
        "expected_version": VERSION,
        "status": "pass",
        "passed": 3,
        "failed": 0,
        "surfaces": [
            retained_surface("hub", 1, platform),
            retained_surface("installer", 2, platform),
            retained_surface("workbench", 3, platform)
        ]
    })
}

fn retained_surface(surface: &str, pid: u64, platform: Platform) -> Value {
    let definition = surface_definitions()
        .into_iter()
        .find(|definition| definition.surface == surface)
        .expect("fixture surface should exist");
    let (app_path, executable_path) = retained_surface_paths(definition, platform);
    serde_json::json!({
        "surface": surface,
        "app_path": app_path,
        "executable_path": executable_path,
        "log_path": format!("tmp/packaged-desktop-smoke/{surface}.log"),
        "status": "pass",
        "pid": pid,
        "detail": format!("interactive startup receipt accepted for {surface} {VERSION}")
    })
}
