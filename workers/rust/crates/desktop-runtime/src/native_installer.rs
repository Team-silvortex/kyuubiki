//! Bounded process transport to the companion Installer, never to a PATH-selected program.
use serde_json::Value;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, &'static str>;

pub(crate) fn binary() -> Result<PathBuf> {
    let paths =
        crate::runtime_layout::runtime_paths().map_err(|_| "native_installer_unavailable")?;
    let name = if cfg!(windows) {
        "kyuubiki-installer.exe"
    } else {
        "kyuubiki-installer"
    };
    let path = if paths.is_development() {
        paths.root.join("workers/rust/target/debug").join(name)
    } else {
        paths.root.join("bin").join(name)
    };
    let meta = std::fs::symlink_metadata(&path).map_err(|_| "native_installer_unavailable")?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("native_installer_unavailable");
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| "native_installer_unavailable")?;
    if !canonical.starts_with(
        paths
            .root
            .canonicalize()
            .map_err(|_| "native_installer_unavailable")?,
    ) {
        return Err("native_installer_unavailable");
    }
    Ok(canonical)
}
pub(crate) fn invoke(
    binary: &Path,
    action: &str,
    source: Option<&Path>,
    request: &Value,
    root: Option<&Path>,
) -> Result<Value> {
    if !matches!(action, "install-store-operator" | "project-operator") {
        return Err("invalid_install_request");
    }
    let input = serde_json::to_vec(request).map_err(|_| "invalid_install_request")?;
    if input.len() > 131072 {
        return Err("invalid_install_request");
    }
    let mut command = Command::new(binary);
    command
        .arg(action)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(source) = source {
        command.arg(source);
    }
    if let Some(root) = root {
        command.arg("--store-root").arg(root);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| "native_installer_unavailable")?;
    let mut stdin = child.stdin.take().ok_or("native_installer_unavailable")?;
    // A bounded writer runs alongside the deadline, so a stalled helper cannot block stdin forever.
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let stdout = child.stdout.take().ok_or("native_installer_unavailable")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(1048577).read_to_end(&mut bytes).map(|_| bytes)
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if start.elapsed() < Duration::from_secs(120) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let wrote = writer.join().ok().is_some_and(|r| r.is_ok());
    let bytes = reader
        .join()
        .map_err(|_| "native_install_check_required")?
        .map_err(|_| "native_install_check_required")?;
    if !wrote || !status.is_some_and(|s| s.success()) {
        return Err("native_install_check_required");
    }
    if bytes.len() > 1048576 {
        return Err("invalid_install_receipt");
    }
    serde_json::from_slice(&bytes).map_err(|_| "invalid_install_receipt")
}
