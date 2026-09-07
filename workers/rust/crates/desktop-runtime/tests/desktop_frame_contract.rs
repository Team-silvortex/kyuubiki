use std::path::Path;

#[test]
fn workbench_shell_allows_the_fixed_loopback_frame_without_widening_default_sources() {
    let config = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../apps/workbench-gui/src-tauri/tauri.conf.json");
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config).unwrap()).unwrap();
    let csp = config["app"]["security"]["csp"].as_str().unwrap();
    let directives: Vec<_> = csp.split(';').map(str::trim).collect();
    let frames: Vec<_> = directives
        .iter()
        .filter(|value| value.starts_with("frame-src "))
        .copied()
        .collect();
    assert_eq!(frames, ["frame-src http://127.0.0.1:3000"]);
    for required in [
        "default-src 'self'",
        "script-src 'self'",
        "object-src 'none'",
        "frame-ancestors 'none'",
    ] {
        assert!(directives.contains(&required), "missing {required}");
    }
}
