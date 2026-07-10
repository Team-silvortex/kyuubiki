use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type RunnerResult<T> = Result<T, String>;

pub(crate) fn ssh_status(root: &Path, host: &str, remote_command: String) -> RunnerResult<u8> {
    run_status(
        "ssh",
        [OsString::from(host), OsString::from(remote_command)],
        root,
    )
}

pub(crate) fn ssh_output(root: &Path, host: &str, remote_command: String) -> RunnerResult<String> {
    let output = Command::new("ssh")
        .args([OsString::from(host), OsString::from(remote_command)])
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to run ssh: {error}"))?;
    if !output.status.success() {
        return Err(format!("ssh command failed on {host}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn ssh_success_quiet(
    root: &Path,
    host: &str,
    remote_command: String,
) -> RunnerResult<bool> {
    let status = Command::new("ssh")
        .args([OsString::from(host), OsString::from(remote_command)])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("failed to run ssh: {error}"))?;
    Ok(status.success())
}

pub(crate) fn scp_from(
    root: &Path,
    host: &str,
    remote_path: &str,
    local_path: &Path,
) -> RunnerResult<u8> {
    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    run_status(
        "scp",
        [
            OsString::from(format!("{host}:{remote_path}")),
            local_path.to_path_buf().into_os_string(),
        ],
        root,
    )
}

pub(crate) fn rsync_to(
    root: &Path,
    excludes: &[&str],
    sources: &[PathBuf],
    destination: &str,
) -> RunnerResult<u8> {
    let mut args = vec![OsString::from("-az")];
    for exclude in excludes {
        args.push(OsString::from("--exclude"));
        args.push(OsString::from(exclude));
    }
    args.extend(sources.iter().map(|path| path.clone().into_os_string()));
    args.push(OsString::from(destination));
    run_status("rsync", args, root)
}

pub(crate) fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn remote_shell_path(value: &str) -> String {
    value
        .strip_prefix("~/")
        .map(|rest| format!("$HOME/{}", shell_escape(rest)))
        .unwrap_or_else(|| shell_escape(value))
}

fn run_status<I>(program: &str, args: I, cwd: &Path) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_shell_values_and_home_paths() {
        assert_eq!(shell_escape("abc/def:1"), "'abc/def:1'");
        assert_eq!(shell_escape("a'b"), "'a'\\''b'");
        assert_eq!(remote_shell_path("~/kyuubiki lab"), "$HOME/'kyuubiki lab'");
        assert_eq!(remote_shell_path("/tmp/kyuubiki"), "'/tmp/kyuubiki'");
    }
}
