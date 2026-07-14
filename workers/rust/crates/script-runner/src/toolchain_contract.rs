use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/toolchains.json";
const PACKAGE_FILES: &[&str] = &[
    "apps/frontend/package.json",
    "apps/hub-gui/package.json",
    "apps/workbench-gui/package.json",
    "apps/installer-gui/package.json",
];

pub(crate) fn run_check_toolchain_contract(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-toolchain-contract");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-toolchain-contract does not accept positional arguments".to_string());
    }

    let contract = read_json(root, CONTRACT_PATH)?;
    let issues = validate_toolchain_contract(root, &contract)?;
    if !issues.is_empty() {
        eprintln!("toolchain contract drift detected:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }

    println!("toolchain contract ok");
    Ok(0)
}

fn validate_toolchain_contract(root: &Path, contract: &Value) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    let rust = section(contract, "rust")?;
    let elixir = section(contract, "elixir")?;
    let node = section(contract, "node")?;

    let rust_toolchain = read_text(root, "rust-toolchain.toml")?;
    require_contains(
        &mut issues,
        "rust-toolchain.toml",
        &rust_toolchain,
        &format!("channel = \"{}\"", required_str(rust, "channel")?),
        "Rust channel",
    );
    require_contains(
        &mut issues,
        "rust-toolchain.toml",
        &rust_toolchain,
        &format!("profile = \"{}\"", required_str(rust, "profile")?),
        "Rust profile",
    );
    for component in string_array(rust, "components") {
        require_contains(
            &mut issues,
            "rust-toolchain.toml",
            &rust_toolchain,
            &format!("\"{component}\""),
            &format!("Rust component {component}"),
        );
    }

    let direct_mesh_dockerfile = read_text(root, "deploy/docker/direct-mesh-benchmark.Dockerfile")?;
    require_contains(
        &mut issues,
        "deploy/docker/direct-mesh-benchmark.Dockerfile",
        &direct_mesh_dockerfile,
        &format!("ARG BASE_IMAGE={}", required_str(elixir, "container_base")?),
        "Elixir container base",
    );
    require_contains(
        &mut issues,
        "deploy/docker/direct-mesh-benchmark.Dockerfile",
        &direct_mesh_dockerfile,
        &format!("ARG NODE_VERSION={}", required_str(node, "preferred")?),
        "Node version",
    );
    require_contains(
        &mut issues,
        "deploy/docker/direct-mesh-benchmark.Dockerfile",
        &direct_mesh_dockerfile,
        &format!("ARG RUST_TOOLCHAIN={}", required_str(rust, "preferred")?),
        "Rust toolchain",
    );

    let headless_dockerfile = read_text(root, "deploy/docker/headless-live-test.Dockerfile")?;
    require_contains(
        &mut issues,
        "deploy/docker/headless-live-test.Dockerfile",
        &headless_dockerfile,
        &format!("ARG BASE_IMAGE={}", required_str(elixir, "container_base")?),
        "Elixir container base",
    );
    require_contains(
        &mut issues,
        "deploy/docker/headless-live-test.Dockerfile",
        &headless_dockerfile,
        &format!("ARG RUST_IMAGE=rust:{}", required_str(rust, "preferred")?),
        "Rust base image",
    );

    for file in ["apps/web/mix.exs", "sdks/elixir/mix.exs"] {
        let text = read_text(root, file)?;
        require_contains(
            &mut issues,
            file,
            &text,
            &format!("elixir: \"{}\"", required_str(elixir, "constraint")?),
            "Elixir constraint",
        );
    }

    let web_config = read_text(root, "apps/web/config/config.exs")?;
    for env_key in string_array(elixir, "self_host_required_env") {
        require_contains(
            &mut issues,
            "apps/web/config/config.exs",
            &web_config,
            &env_key,
            &format!("self-host env {env_key}"),
        );
    }

    require_file_tokens(
        root,
        &mut issues,
        "scripts/check-elixir-self-host.mjs",
        &[
            ("config/toolchains.json", "shared toolchain contract"),
            ("apps/web/mix.exs", "web Mix contract"),
        ],
    )?;
    require_file_tokens(
        root,
        &mut issues,
        "workers/rust/crates/installer/src/embedded_runtime.rs",
        &[
            ("kyuubiki.embedded-runtimes/v1", "embedded runtime schema"),
            (
                "config/toolchains.json#/elixir",
                "embedded Elixir source contract",
            ),
            (
                "config/toolchains.json#/node",
                "embedded Node source contract",
            ),
        ],
    )?;
    require_file_tokens(
        root,
        &mut issues,
        "scripts/kyuubiki-runtime-resolver.mjs",
        &[
            ("embedded-runtimes.json", "embedded runtime manifest lookup"),
            ("KYUUBIKI_RUNTIME_STRICT", "strict runtime mode"),
            ("host-fallback", "host fallback visibility"),
        ],
    )?;
    require_file_tokens(
        root,
        &mut issues,
        "workers/rust/crates/script-runner/src/workflow_mesh_remote.rs",
        &[
            ("scripts/toolchain-env.mjs", "toolchain env loader"),
            ("KYUUBIKI_REMOTE_OTP_VERSION", "remote OTP default key"),
            (
                "KYUUBIKI_REMOTE_ELIXIR_VERSION",
                "remote Elixir default key",
            ),
        ],
    )?;

    let expected_node_engine = required_str(node, "package_engine")?;
    for package_file in PACKAGE_FILES {
        let manifest = read_json(root, package_file)?;
        require_package_engine(&mut issues, package_file, &manifest, expected_node_engine);
        let lockfile_path = package_file.replace("package.json", "package-lock.json");
        let lockfile = read_json(root, &lockfile_path)?;
        require_package_lock_engine(&mut issues, &lockfile_path, &lockfile, expected_node_engine);
    }

    Ok(issues)
}

fn require_file_tokens(
    root: &Path,
    issues: &mut Vec<String>,
    file: &str,
    tokens: &[(&str, &str)],
) -> RunnerResult<()> {
    let text = read_text(root, file)?;
    for (expected, label) in tokens {
        require_contains(issues, file, &text, expected, label);
    }
    Ok(())
}

fn require_contains(issues: &mut Vec<String>, file: &str, text: &str, expected: &str, label: &str) {
    if !text.contains(expected) {
        issues.push(format!("{file}: expected {label} to include {expected:?}"));
    }
}

fn require_package_engine(issues: &mut Vec<String>, file: &str, manifest: &Value, expected: &str) {
    let actual = manifest
        .pointer("/engines/node")
        .and_then(Value::as_str)
        .unwrap_or("");
    if actual != expected {
        issues.push(format!(
            "{file}: engines.node is {actual:?}, expected {expected:?}"
        ));
    }
}

fn require_package_lock_engine(
    issues: &mut Vec<String>,
    file: &str,
    lockfile: &Value,
    expected: &str,
) {
    let actual = lockfile
        .pointer("/packages//engines/node")
        .and_then(Value::as_str)
        .unwrap_or("");
    if actual != expected {
        issues.push(format!(
            "{file}: root engines.node is {actual:?}, expected {expected:?}"
        ));
    }
}

fn section<'a>(value: &'a Value, key: &str) -> RunnerResult<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| format!("{CONTRACT_PATH}: missing {key} section"))
}

fn required_str<'a>(value: &'a Value, key: &str) -> RunnerResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{CONTRACT_PATH}: missing string field {key}"))
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = root.join(relative_path);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::{require_contains, require_package_engine};
    use serde_json::json;

    #[test]
    fn reports_missing_contract_tokens() {
        let mut issues = Vec::new();
        require_contains(&mut issues, "file", "abc", "missing", "label");
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn package_engine_accepts_exact_contract() {
        let manifest = json!({ "engines": { "node": ">=20 <25" } });
        let mut issues = Vec::new();
        require_package_engine(&mut issues, "package.json", &manifest, ">=20 <25");
        assert!(issues.is_empty());
    }
}
