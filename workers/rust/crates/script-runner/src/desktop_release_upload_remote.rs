use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::desktop::{Platform, desktop_target_cache_dir, host_platform};
use crate::remote_host;

type RunnerResult<T> = Result<T, String>;

#[derive(Clone, Copy)]
enum ReleaseTarget {
    All,
    Platform(Platform),
}

pub(crate) fn run_desktop_release_upload_remote(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(0);
    }
    let target = args
        .first()
        .and_then(|value| value.to_str())
        .map(parse_release_target)
        .transpose()?
        .unwrap_or_else(|| ReleaseTarget::Platform(host_platform()));
    let config = UploadConfig::from_env(root)?;
    let paths = collect_existing_paths(root, config.version.as_str(), target);
    if paths.is_empty() {
        eprintln!("no local release outputs were found to upload");
        return Ok(1);
    }

    let remote_version_dir = resolve_remote_version_dir(root, &config)?;
    let remote_metadata_dir = format!("{remote_version_dir}/metadata");
    ssh_output(
        root,
        &config,
        format!(
            "mkdir -p {} {}",
            remote_host::shell_escape(&remote_metadata_dir),
            remote_host::shell_escape(&remote_version_dir)
        ),
    )?;

    for relative_path in &paths {
        let source = PathBuf::from(format!("./{}", relative_path.display()));
        let status = rsync_relative_to(
            root,
            &source,
            &format!("{}:{remote_version_dir}/", config.remote_host),
            &config.ssh_bin,
            &config.rsync_bin,
            config.remote_password.as_deref(),
            &config.ssh_options,
        )?;
        if status != 0 {
            return Ok(status);
        }
    }

    if config.purge_local {
        purge_local_outputs(root, target)?;
    }

    println!("uploaded release artifacts for version {}", config.version);
    println!("remote host: {}", config.remote_host);
    println!("remote dir: {remote_version_dir}");
    if config.purge_local {
        println!("local generated bundle outputs were removed after upload");
    }
    Ok(0)
}

struct UploadConfig {
    remote_host: String,
    remote_base_dir: String,
    version: String,
    ssh_bin: String,
    rsync_bin: String,
    remote_password: Option<String>,
    ssh_options: Vec<OsString>,
    purge_local: bool,
}

impl UploadConfig {
    fn from_env(root: &Path) -> RunnerResult<Self> {
        let version = std::env::var("KYUUBIKI_RELEASE_VERSION")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(read_shipping_version(root)?);
        if version.trim().is_empty() {
            return Err(
                "unable to determine release version from deploy/update-channels.json".into(),
            );
        }
        Ok(Self {
            remote_host: std::env::var("KYUUBIKI_RELEASE_REMOTE_HOST")
                .unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            remote_base_dir: std::env::var("KYUUBIKI_RELEASE_REMOTE_DIR")
                .unwrap_or_else(|_| "~/kyuubiki-downloads".to_string()),
            version,
            ssh_bin: std::env::var("KYUUBIKI_RELEASE_REMOTE_SSH_BIN")
                .unwrap_or_else(|_| "ssh".to_string()),
            rsync_bin: std::env::var("KYUUBIKI_RELEASE_REMOTE_RSYNC_BIN")
                .unwrap_or_else(|_| "rsync".to_string()),
            remote_password: release_remote_password()?,
            ssh_options: split_words(
                &std::env::var("KYUUBIKI_RELEASE_REMOTE_SSH_OPTS")
                    .unwrap_or_else(|_| "-o StrictHostKeyChecking=yes".to_string()),
            ),
            purge_local: std::env::var("PURGE_LOCAL").is_ok_and(|value| value == "1"),
        })
    }
}

fn parse_release_target(value: &str) -> RunnerResult<ReleaseTarget> {
    match value {
        "macos" => Ok(ReleaseTarget::Platform(Platform::Macos)),
        "linux" => Ok(ReleaseTarget::Platform(Platform::Linux)),
        "windows" => Ok(ReleaseTarget::Platform(Platform::Windows)),
        "all" => Ok(ReleaseTarget::All),
        other => Err(format!("unsupported platform: {other}")),
    }
}

fn collect_existing_paths(root: &Path, version: &str, target: ReleaseTarget) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for path in [
        "releases/index.json".to_string(),
        "releases/update-catalog.json".to_string(),
        format!("releases/snapshots/{version}.json"),
        "deploy/update-channels.json".to_string(),
        "deploy/installation-integrity-contract.json".to_string(),
        "docs/update-catalog.html".to_string(),
        "docs/installation-integrity-contract.html".to_string(),
        "apps/hub-gui/ui/docs/update-catalog.html".to_string(),
        "apps/hub-gui/ui/docs/installation-integrity.html".to_string(),
    ] {
        append_if_exists(root, &mut paths, path);
    }
    for platform in expanded_platforms(target) {
        append_if_exists(root, &mut paths, format!("dist/{}", platform.as_str()));
        collect_bundle_paths(root, &mut paths, platform);
    }
    paths
}

fn collect_bundle_paths(root: &Path, paths: &mut Vec<PathBuf>, platform: Platform) {
    for subdir in platform.bundle_subdirs() {
        let bundle_path = desktop_target_cache_dir(root, platform)
            .join("release")
            .join("bundle")
            .join(subdir);
        append_if_exists(root, paths, workspace_relative_path(root, &bundle_path));
    }
}

fn workspace_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn append_if_exists(root: &Path, paths: &mut Vec<PathBuf>, relative: String) {
    if root.join(&relative).exists() {
        paths.push(PathBuf::from(relative));
    }
}

fn expanded_platforms(target: ReleaseTarget) -> Vec<Platform> {
    match target {
        ReleaseTarget::All => Platform::all().to_vec(),
        ReleaseTarget::Platform(platform) => vec![platform],
    }
}

fn purge_local_outputs(root: &Path, target: ReleaseTarget) -> RunnerResult<()> {
    for platform in expanded_platforms(target) {
        remove_dir_if_exists(root.join(format!("dist/{}", platform.as_str())))?;
        for subdir in platform.bundle_subdirs() {
            remove_dir_if_exists(
                desktop_target_cache_dir(root, platform)
                    .join("release")
                    .join("bundle")
                    .join(subdir),
            )?;
        }
    }
    Ok(())
}

fn remove_dir_if_exists(path: PathBuf) -> RunnerResult<()> {
    if path.exists() {
        std::fs::remove_dir_all(&path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

fn resolve_remote_version_dir(root: &Path, config: &UploadConfig) -> RunnerResult<String> {
    let command = format!(
        "base_dir={}; version={}; case \"$base_dir\" in '~') base_dir=\"$HOME\" ;; '~/'*) \
         base_dir=\"$HOME/${{base_dir#~/}}\" ;; esac; printf '%s/releases/%s\\n' \"$base_dir\" \"$version\"",
        remote_host::shell_escape(&config.remote_base_dir),
        remote_host::shell_escape(&config.version)
    );
    ssh_output(root, config, command)
}

fn ssh_output(root: &Path, config: &UploadConfig, remote_command: String) -> RunnerResult<String> {
    let mut command = if config.remote_password.is_some() {
        ensure_sshpass(root)?;
        let mut command = Command::new("sshpass");
        command.args(["-e", &config.ssh_bin]);
        command.env(
            "SSHPASS",
            config.remote_password.as_deref().unwrap_or_default(),
        );
        command
    } else {
        Command::new(&config.ssh_bin)
    };
    command
        .args(&config.ssh_options)
        .arg(&config.remote_host)
        .arg(remote_command)
        .current_dir(root);
    let output = command
        .output()
        .map_err(|error| format!("failed to run {}: {error}", config.ssh_bin))?;
    if !output.status.success() {
        return Err(format!("ssh command failed on {}", config.remote_host));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn rsync_relative_to(
    root: &Path,
    source: &Path,
    destination: &str,
    ssh_bin: &str,
    rsync_bin: &str,
    password: Option<&str>,
    ssh_options: &[OsString],
) -> RunnerResult<u8> {
    let mut command = if password.is_some() {
        ensure_sshpass(root)?;
        let mut command = Command::new("sshpass");
        command.args(["-e", rsync_bin]);
        command.env("SSHPASS", password.unwrap_or_default());
        command
    } else {
        Command::new(rsync_bin)
    };
    command
        .args(["-az", "--relative", "-e"])
        .arg(build_rsync_ssh_command(ssh_bin, ssh_options))
        .arg(source)
        .arg(destination)
        .current_dir(root);
    let status = command
        .status()
        .map_err(|error| format!("failed to run {rsync_bin}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

fn build_rsync_ssh_command(ssh_bin: &str, ssh_options: &[OsString]) -> OsString {
    let mut value = OsString::from(ssh_bin);
    for option in ssh_options {
        value.push(" ");
        value.push(option);
    }
    value
}

fn ensure_sshpass(root: &Path) -> RunnerResult<()> {
    let status = Command::new("sshpass")
        .arg("-V")
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "KYUUBIKI_RELEASE_REMOTE_PASSWORD was set but sshpass is not installed")?;
    if status.success() {
        Ok(())
    } else {
        Err("KYUUBIKI_RELEASE_REMOTE_PASSWORD was set but sshpass is not installed".to_string())
    }
}

fn read_shipping_version(root: &Path) -> RunnerResult<String> {
    let text = std::fs::read_to_string(root.join("deploy/update-channels.json"))
        .map_err(|error| format!("failed to read deploy/update-channels.json: {error}"))?;
    let marker = "\"shipping_version\"";
    let Some(index) = text.find(marker) else {
        return Ok(String::new());
    };
    let tail = &text[index + marker.len()..];
    let Some(colon) = tail.find(':') else {
        return Ok(String::new());
    };
    let quoted = tail[colon + 1..].trim_start();
    if !quoted.starts_with('"') {
        return Ok(String::new());
    }
    let rest = &quoted[1..];
    Ok(rest
        .split('"')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string())
}

fn split_words(value: &str) -> Vec<OsString> {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(OsString::from)
        .collect()
}

fn release_remote_password() -> RunnerResult<Option<String>> {
    let password = std::env::var("KYUUBIKI_RELEASE_REMOTE_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty());
    if password.is_some()
        && !std::env::var("KYUUBIKI_RELEASE_REMOTE_ALLOW_PASSWORD").is_ok_and(|value| value == "1")
    {
        return Err(
            "KYUUBIKI_RELEASE_REMOTE_PASSWORD requires KYUUBIKI_RELEASE_REMOTE_ALLOW_PASSWORD=1; prefer SSH keys or an agent"
                .to_string(),
        );
    }
    Ok(password)
}

fn print_help() {
    println!(
        "Usage:\n  ./scripts/kyuubiki desktop-release-upload-remote [macos|linux|windows|all]\n  \
./scripts/kyuubiki desktop-upload-remote [macos|linux|windows|all]\n\n\
Upload generated desktop release outputs to a remote download server.\n\n\
Environment:\n  \
KYUUBIKI_RELEASE_REMOTE_HOST   SSH host or alias. Default: kyuubiki-lab\n  \
	KYUUBIKI_RELEASE_REMOTE_DIR    Remote root directory. Default: ~/kyuubiki-downloads\n  \
	KYUUBIKI_RELEASE_REMOTE_PASSWORD Optional dev-only sshpass-backed compatibility path\n  \
	KYUUBIKI_RELEASE_REMOTE_ALLOW_PASSWORD Set to 1 to allow the password compatibility path\n  \
KYUUBIKI_RELEASE_VERSION       Override version folder\n  \
KYUUBIKI_RELEASE_REMOTE_SSH_BIN Override SSH binary. Default: ssh\n  \
KYUUBIKI_RELEASE_REMOTE_RSYNC_BIN Override rsync binary. Default: rsync\n  \
KYUUBIKI_RELEASE_REMOTE_SSH_OPTS Extra SSH options\n  \
PURGE_LOCAL                    Set to 1 to delete uploaded bundle outputs"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_release_target_expands_to_the_shared_platform_set() {
        assert_eq!(
            expanded_platforms(ReleaseTarget::All),
            vec![Platform::Macos, Platform::Linux, Platform::Windows]
        );
    }

    #[test]
    fn bundle_collection_keeps_shared_cache_paths_relative_to_the_workspace() {
        let root = unique_temp_dir();
        let bundle_dir = desktop_target_cache_dir(&root, Platform::Linux)
            .join("release")
            .join("bundle")
            .join("deb");
        std::fs::create_dir_all(&bundle_dir).unwrap();

        let mut paths = Vec::new();
        collect_bundle_paths(&root, &mut paths, Platform::Linux);

        assert_eq!(
            paths,
            vec![PathBuf::from(
                "target/desktop-cache/linux/release/bundle/deb"
            )]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "kyuubiki-desktop-release-upload-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
