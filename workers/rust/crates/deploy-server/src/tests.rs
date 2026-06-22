use std::collections::HashMap;
use std::env;
use std::fs;
use std::iter;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::*;

#[test]
fn safe_join_blocks_parent_escape() {
    let root = PathBuf::from("/tmp/kyuubiki");
    let error = safe_join(&root, "../secrets.txt").unwrap_err();
    assert!(error.contains("escapes configured root"));
}

#[test]
fn channel_route_uses_catalog_payload() {
    let root = unique_test_root("channel");
    fs::create_dir_all(root.join("deploy")).unwrap();
    fs::write(
        root.join("deploy").join("update-channels.json"),
        serde_json::to_vec_pretty(&json!({ "channels": [{ "id": "stable", "version": "1.8.0" }] }))
            .unwrap(),
    )
    .unwrap();

    let config = test_config(&root);
    let response = serve_channel_details(&config, "/api/v1/update/channels/stable");
    assert_eq!(response.status_code, 200);
    assert!(
        String::from_utf8(response.body)
            .unwrap()
            .contains("\"stable\"")
    );
}

#[test]
fn artifact_route_serves_file_body() {
    let root = unique_test_root("artifact");
    fs::create_dir_all(root.join("artifacts")).unwrap();
    fs::write(root.join("artifacts").join("demo.txt"), "hello artifact").unwrap();

    let mut config = test_config(&root);
    config.artifact_root = root.join("artifacts");
    let response = serve_artifact(&config, "/artifacts/demo.txt");
    assert_eq!(response.status_code, 200);
    assert_eq!(String::from_utf8(response.body).unwrap(), "hello artifact");
}

#[test]
fn config_descriptor_does_not_expose_host_paths() {
    let root = unique_test_root("descriptor");
    let rendered = test_config(&root).render();

    assert!(!rendered.contains("workspace_root"));
    assert!(!rendered.contains("deploy_root"));
    assert!(!rendered.contains("artifact_root"));
    assert!(!rendered.contains("update_catalog_path"));
}

#[test]
fn local_agent_route_serves_checked_in_example_shape() {
    let root = unique_test_root("local-agent-example");
    fs::create_dir_all(root.join("deploy")).unwrap();
    fs::write(
        root.join("deploy").join("agents.local.example.json"),
        br#"{"source":"example"}"#,
    )
    .unwrap();
    fs::write(
        root.join("deploy").join("agents.local.json"),
        br#"{"source":"local"}"#,
    )
    .unwrap();

    let response = route_request(
        &test_config(&root),
        &Request {
            method: "GET".to_string(),
            path: "/api/v1/deploy/agents/local".to_string(),
            headers: HashMap::new(),
        },
    );
    assert_eq!(response.status_code, 200);
    assert!(
        String::from_utf8(response.body)
            .unwrap()
            .contains("example")
    );
}

#[test]
fn non_health_routes_require_token_when_configured() {
    let root = unique_test_root("auth");
    let mut config = test_config(&root);
    config.auth_token = Some("secret".to_string());

    let unauthorized = route_request(
        &config,
        &Request {
            method: "GET".to_string(),
            path: "/api/v1/server/config".to_string(),
            headers: HashMap::new(),
        },
    );
    assert_eq!(unauthorized.status_code, 401);

    let authorized = route_request(
        &config,
        &Request {
            method: "GET".to_string(),
            path: "/api/v1/server/config".to_string(),
            headers: iter::once(("authorization".to_string(), "Bearer secret".to_string()))
                .collect(),
        },
    );
    assert_eq!(authorized.status_code, 200);
}

fn test_config(root: &Path) -> DeployServerConfig {
    DeployServerConfig {
        host: "127.0.0.1".to_string(),
        port: 4070,
        auth_token: None,
        workspace_root: root.to_path_buf(),
        deploy_root: root.join("deploy"),
        artifact_root: root.to_path_buf(),
        update_catalog_path: root.join("deploy").join("update-channels.json"),
    }
}

fn unique_test_root(label: &str) -> PathBuf {
    let root = env::temp_dir().join(format!(
        "kyuubiki-deploy-server-{label}-{}",
        unix_timestamp()
    ));
    let _ = fs::remove_dir_all(&root);
    root
}
