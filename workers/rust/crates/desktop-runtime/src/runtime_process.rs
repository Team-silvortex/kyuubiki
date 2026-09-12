use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::runtime_process_record::{self as records, Ownership, ProcessRecord};

pub(crate) struct ManagedProcess {
    pub label: String,
    pub command: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub pid: PathBuf,
    pub log: PathBuf,
    pub port: Option<u16>,
    pub env: HashMap<String, String>,
}

pub(crate) fn already_running(pid: &Path, label: &str, port: u16) -> Result<bool, String> {
    match records::observe(pid, label, Some(port))? {
        Ownership::Owned(record) => {
            if is_port_listening(port) {
                Ok(true)
            } else {
                Err(format!(
                    "{label}: managed process {} is alive but port {port} is not ready; use explicit restart",
                    record.pid
                ))
            }
        }
        Ownership::Absent | Ownership::Stale => {
            if is_port_listening(port) {
                return Err(format!(
                    "{label}: port {port} is occupied by an unmanaged process; refusing to adopt it"
                ));
            }
            records::clear(pid)?;
            Ok(false)
        }
    }
}

pub(crate) fn spawn_managed(process: ManagedProcess, timeout: Duration) -> Result<u32, String> {
    if let Some(parent) = process.pid.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&process.log)
        .map_err(|error| format!("failed to open {}: {error}", process.log.display()))?;
    let stderr = stdout.try_clone().map_err(|error| error.to_string())?;
    let mut command = Command::new(&process.command);
    command
        .args(&process.args)
        .current_dir(&process.cwd)
        .envs(&process.env)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    configure_detached(&mut command);
    let mut child = command.spawn().map_err(|error| {
        format!(
            "failed to start {} with {}: {error}",
            process.label,
            process.command.display()
        )
    })?;
    let pid = child.id();
    let mut registered = false;
    let ready = (|| {
        records::persist(&process.pid, pid, &process.label, process.port)?;
        registered = true;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                return Err(format!(
                    "{} exited before readiness with {status}",
                    process.label
                ));
            }
            if process.port.is_none_or(is_port_listening) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "{} timed out waiting for port {:?} to become ready",
                    process.label, process.port
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
    })();
    if let Err(error) = ready {
        cleanup_child(&mut child);
        let mut cleanup = String::new();
        if registered {
            if let Err(cleanup_error) = records::clear(&process.pid) {
                cleanup = format!("; record cleanup failed: {cleanup_error}");
            }
        }
        return Err(format!(
            "{error}{cleanup}; {} (pid {pid}) log: {}",
            process.label,
            log_tail(&process.log)
        ));
    }
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pid)
}

#[cfg(test)]
#[path = "runtime_process_tests.rs"]
mod tests;

fn cleanup_child(child: &mut Child) {
    // The still-owned Child handle is authoritative even if writing the ownership record failed.
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}

#[cfg(windows)]
fn configure_detached(command: &mut Command) {
    command.creation_flags(0x0000_0200 | 0x0800_0000);
}

pub(crate) fn stop_managed(pid: &Path, label: &str, port: Option<u16>) -> Result<String, String> {
    match records::observe(pid, label, port)? {
        Ownership::Owned(record) => {
            terminate_process(&record)?;
            if let Some(port) = port {
                wait_for_port_closed(port, Duration::from_secs(5))?;
            }
            records::clear(pid)?;
            Ok(format!("stopped {label} (pid {})", record.pid))
        }
        Ownership::Absent | Ownership::Stale => {
            records::clear(pid)?;
            if port.is_some_and(is_port_listening) {
                Err(format!(
                    "{label}: port {} is still busy (unmanaged process); no process was stopped",
                    port.unwrap()
                ))
            } else {
                Ok(format!("{label}: stopped"))
            }
        }
    }
}

#[cfg(unix)]
fn signal_owned(record: &ProcessRecord, signal: i32) -> Result<(), String> {
    if records::identity(record.pid)?.as_deref() != Some(record.instance.as_str()) {
        return Ok(());
    }
    if unsafe { libc::getpgid(record.pid as i32) } != record.pid as i32 {
        return Err(format!(
            "process {} is not the managed process-group leader",
            record.pid
        ));
    }
    if unsafe { libc::kill(-(record.pid as i32), signal) } != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(format!("failed to signal process {}: {error}", record.pid));
        }
    }
    Ok(())
}

fn wait_for_exit(record: &ProcessRecord, timeout: Duration) -> Result<bool, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if records::identity(record.pid)?.as_deref() != Some(record.instance.as_str()) {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn terminate_process(record: &ProcessRecord) -> Result<(), String> {
    signal_owned(record, libc::SIGTERM)?;
    if !wait_for_exit(record, Duration::from_secs(5))? {
        signal_owned(record, libc::SIGKILL)?;
        if !wait_for_exit(record, Duration::from_secs(5))? {
            return Err(format!("managed process {} did not exit", record.pid));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn terminate_process(record: &ProcessRecord) -> Result<(), String> {
    if records::identity(record.pid)?.as_deref() != Some(record.instance.as_str()) {
        return Ok(());
    }
    let status = Command::new("taskkill")
        .args(["/PID", &record.pid.to_string(), "/T", "/F"])
        .creation_flags(0x0800_0000)
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() || !wait_for_exit(record, Duration::from_secs(5))? {
        return Err(format!("failed to stop managed process {}", record.pid));
    }
    Ok(())
}

fn wait_for_port_closed(port: u16, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while is_port_listening(port) {
        if Instant::now() >= deadline {
            return Err(format!(
                "port {port} is still busy after managed process exit; no unrelated process was stopped"
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

pub(crate) fn is_port_listening(port: u16) -> bool {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    TcpStream::connect_timeout(&address, Duration::from_millis(180)).is_ok()
}

pub(crate) fn service_line(label: &str, pid: &Path, port: u16, scheme: &str) -> String {
    let address = format!("{scheme}://127.0.0.1:{port}");
    match records::observe(pid, label, Some(port)) {
        Ok(Ownership::Owned(record)) if is_port_listening(port) => {
            format!("{label}: running on {address} (pid {})", record.pid)
        }
        Ok(Ownership::Owned(record)) => format!(
            "{label}: starting on {address} (pid {}; not ready)",
            record.pid
        ),
        Ok(_) if is_port_listening(port) => {
            format!("{label}: blocked on {address} (unmanaged pid)")
        }
        Ok(_) => format!("{label}: stopped"),
        Err(error) => format!("{label}: blocked ({error})"),
    }
}

fn log_tail(path: &Path) -> String {
    let read = (|| -> std::io::Result<String> {
        let mut file = fs::File::open(path)?;
        let size = file.metadata()?.len();
        file.seek(SeekFrom::Start(size.saturating_sub(16 * 1024)))?;
        let mut bytes = Vec::new();
        file.take(16 * 1024).read_to_end(&mut bytes)?;
        let text = String::from_utf8_lossy(&bytes);
        let mut lines: Vec<_> = text.lines().rev().take(8).collect();
        lines.reverse();
        Ok(lines.join(" | "))
    })();
    read.ok()
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "no runtime log output".into())
}
