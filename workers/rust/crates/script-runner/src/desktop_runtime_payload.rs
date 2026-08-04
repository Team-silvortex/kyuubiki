use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use kyuubiki_installer::{Platform as InstallerPlatform, seal_runtime_payload};

use crate::desktop::{Platform, host_platform};
use crate::{RepoPaths, RunnerResult, run_installer};

pub(crate) fn run_desktop_runtime_payload(
    paths: &RepoPaths,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let platform = parse_platform(args.first())?;
    if platform != host_platform() {
        return Err(format!(
            "runtime payload for {} must be assembled on a {} host",
            platform.as_str(),
            platform.as_str()
        ));
    }
    let version = workspace_version(&paths.rust.join("Cargo.toml"))?;
    let node = configured_node_binary(platform)?;
    verify_node_binary(&node)?;
    let stage = paths.root.join("dist").join(platform.as_str());

    let status = run_installer(
        paths,
        "stage-release",
        vec![OsString::from(platform.as_str())],
    )?;
    if status != 0 {
        return Ok(status);
    }
    build_rust_runtime(paths)?;
    build_orchestrator(paths, &version)?;
    build_frontend(paths)?;
    populate_stage(paths, platform, &stage, &node)?;
    let manifest = seal_runtime_payload(&stage, &version, installer_platform(platform))?;
    println!(
        "sealed {} runtime payload {} at {}",
        platform.as_str(),
        version,
        manifest
    );
    Ok(0)
}

fn build_rust_runtime(paths: &RepoPaths) -> RunnerResult<()> {
    run_checked(
        &paths.rust,
        "cargo",
        [
            "build",
            "--release",
            "-p",
            "kyuubiki-cli",
            "--bin",
            "kyuubiki-cli",
            "--bin",
            "kyuubiki-headless",
        ],
        &[],
    )?;
    run_checked(
        &paths.rust,
        "cargo",
        [
            "build",
            "--release",
            "-p",
            "kyuubiki-desktop-runtime",
            "--bin",
            "kyuubiki-runtime",
        ],
        &[],
    )
}

fn build_orchestrator(paths: &RepoPaths, version: &str) -> RunnerResult<()> {
    run_checked(
        &paths.web,
        "mix",
        ["release", "kyuubiki_web", "--overwrite"],
        &[("MIX_ENV", "prod"), ("KYUUBIKI_RELEASE_VERSION", version)],
    )
}

fn build_frontend(paths: &RepoPaths) -> RunnerResult<()> {
    run_checked(&paths.frontend, "npm", ["run", "build"], &[])
}

fn populate_stage(
    paths: &RepoPaths,
    platform: Platform,
    stage: &Path,
    node: &Path,
) -> RunnerResult<()> {
    let executable = |name: &str| {
        if platform == Platform::Windows {
            format!("{name}.exe")
        } else {
            name.to_string()
        }
    };
    copy_file(
        &paths
            .rust
            .join("target/release")
            .join(executable("kyuubiki-cli")),
        &stage.join("bin").join(executable("kyuubiki-cli")),
    )?;
    copy_file(
        &paths
            .rust
            .join("target/release")
            .join(executable("kyuubiki-headless")),
        &stage.join("bin").join(executable("kyuubiki-headless")),
    )?;
    copy_file(
        &paths
            .rust
            .join("target/release")
            .join(executable("kyuubiki-runtime")),
        &stage.join("bin").join(executable("kyuubiki-runtime")),
    )?;

    let orchestrator = paths.web.join("_build/prod/rel/kyuubiki_web");
    replace_tree(&orchestrator, &stage.join("services/orchestrator"))?;

    let standalone = paths.frontend.join(".next/standalone");
    if !standalone.join("server.js").is_file() {
        return Err(format!(
            "Next standalone output is missing {}; outputFileTracingRoot must remain app-scoped",
            standalone.join("server.js").display()
        ));
    }
    let frontend = stage.join("services/frontend");
    replace_tree(&standalone, &frontend)?;
    replace_tree(
        &paths.frontend.join(".next/static"),
        &frontend.join(".next/static"),
    )?;
    replace_tree(&paths.frontend.join("public"), &frontend.join("public"))?;

    let node_target = stage
        .join("runtimes")
        .join(platform.as_str())
        .join("node/bin")
        .join(executable("node"));
    copy_file(node, &node_target)?;
    Ok(())
}

fn configured_node_binary(platform: Platform) -> RunnerResult<PathBuf> {
    let root = env::var_os("KYUUBIKI_NODE_RUNTIME_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "KYUUBIKI_NODE_RUNTIME_ROOT must point to an official, self-contained Node runtime; host package-manager Node is not accepted for distribution payloads"
                .to_string()
        })?;
    let candidates = if platform == Platform::Windows {
        vec![root.join("node.exe"), root.join("bin/node.exe")]
    } else {
        vec![root.join("bin/node")]
    };
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "configured Node runtime has no executable under {}",
                root.display()
            )
        })
}

fn verify_node_binary(node: &Path) -> RunnerResult<()> {
    let output = Command::new(node)
        .arg("--version")
        .output()
        .map_err(|error| format!("failed to run configured Node {}: {error}", node.display()))?;
    if !output.status.success() {
        return Err(format!(
            "configured Node runtime failed its version probe: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if version.trim() != "v20.19.2" {
        return Err(format!(
            "configured Node runtime is {}, expected v20.19.2 from config/toolchains.json",
            version.trim()
        ));
    }
    Ok(())
}

fn replace_tree(source: &Path, target: &Path) -> RunnerResult<()> {
    if !source.is_dir() {
        return Err(format!(
            "required payload directory is missing: {}",
            source.display()
        ));
    }
    if target.exists() {
        fs::remove_dir_all(target)
            .map_err(|error| format!("failed to clear {}: {error}", target.display()))?;
    }
    copy_tree(source, target)
}

fn copy_tree(source: &Path, target: &Path) -> RunnerResult<()> {
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::metadata(&source_path)
            .map_err(|error| format!("failed to inspect {}: {error}", source_path.display()))?;
        if metadata.is_dir() {
            copy_tree(&source_path, &target_path)?;
        } else if metadata.is_file() {
            copy_file(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, target: &Path) -> RunnerResult<()> {
    if !source.is_file() {
        return Err(format!(
            "required payload file is missing: {}",
            source.display()
        ));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::copy(source, target).map_err(|error| {
        format!(
            "failed to copy {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn run_checked<I, S>(
    cwd: &Path,
    program: &str,
    args: I,
    environment: &[(&str, &str)],
) -> RunnerResult<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(environment.iter().copied())
        .status()
        .map_err(|error| format!("failed to run {program} in {}: {error}", cwd.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} failed in {} with status {status}",
            cwd.display()
        ))
    }
}

fn workspace_version(path: &Path) -> RunnerResult<String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        .map(ToString::to_string)
        .ok_or_else(|| format!("workspace version is missing from {}", path.display()))
}

fn parse_platform(value: Option<&OsString>) -> RunnerResult<Platform> {
    match value.and_then(|value| value.to_str()) {
        None | Some("") => Ok(host_platform()),
        Some("macos") => Ok(Platform::Macos),
        Some("linux") => Ok(Platform::Linux),
        Some("windows") => Ok(Platform::Windows),
        Some(other) => Err(format!(
            "unsupported runtime payload platform `{other}`; expected macos, linux, or windows"
        )),
    }
}

fn installer_platform(platform: Platform) -> InstallerPlatform {
    match platform {
        Platform::Macos => InstallerPlatform::Macos,
        Platform::Linux => InstallerPlatform::Linux,
        Platform::Windows => InstallerPlatform::Windows,
    }
}

#[cfg(test)]
mod tests {
    use super::workspace_version;

    #[test]
    fn reads_workspace_package_version() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
        assert_eq!(workspace_version(&path).unwrap(), env!("CARGO_PKG_VERSION"));
    }
}
