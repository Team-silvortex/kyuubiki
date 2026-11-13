//! Private child-only native host. No Agent, Store or controller startup here.
use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, load_external_operator_packages_with_dynamic_host,
};
use kyuubiki_protocol::{
    OperatorRunRequest, OperatorRunResult, OperatorValidationStatus, ProjectOperatorEntry,
    ProjectOperatorPin,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
};

pub(super) const CHILD_MODE: &str = "__native-operator-process-v1";
pub(super) const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoadedOperatorPackage {
    pub project_pin: ProjectOperatorPin,
    pub package_id: String,
    pub package_version: String,
    pub sdk_api_version: String,
    pub execution_abi: String,
    pub runtime: String,
    pub validation_status: String,
    pub entrypoint_sha256: String,
    pub operator_kinds: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub(super) enum Request {
    Fetch(super::fetch::FetchRequest),
    Probe {
        root: PathBuf,
    },
    Run {
        root: PathBuf,
        expected: Vec<LoadedOperatorPackage>,
        request: OperatorRunRequest,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) enum Reply {
    Fetched,
    Probed(Vec<LoadedOperatorPackage>),
    Executed(OperatorRunResult),
    Failed { kind: FailureKind, message: String },
}

#[derive(Serialize, Deserialize)]
pub(super) enum FailureKind {
    Fetch,
    Activation,
    Identity,
    Execution,
}

pub(crate) fn maybe_run() -> bool {
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new(CHILD_MODE)) {
        return false;
    }
    let status = if serve().is_ok() { 0 } else { 70 };
    terminate(status)
}

fn terminate(status: i32) -> ! {
    // Never run untrusted atexit handlers on parent loss or after the bounded call.
    #[cfg(unix)]
    unsafe {
        libc::_exit(status)
    }
    #[cfg(not(unix))]
    std::process::exit(status)
}

fn serve() -> Result<(), String> {
    #[cfg(unix)]
    let owned_group = unsafe {
        let pid = libc::getpid();
        if pid <= 1 || libc::getpgrp() != pid {
            return Err("native child requires its own process group".into());
        }
        pid
    };
    #[cfg(unix)]
    unsafe {
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::setrlimit(libc::RLIMIT_CORE, &limit) != 0 {
            return Err("cannot disable native child core dumps".into());
        }
    }
    let address: SocketAddr = std::env::args()
        .nth(2)
        .ok_or("missing IPC address")?
        .parse()
        .map_err(|_| "invalid IPC address")?;
    if !address.ip().is_loopback() {
        return Err("IPC must be loopback".into());
    }
    let mut nonce = [0u8; 32];
    std::io::stdin()
        .read_exact(&mut nonce)
        .map_err(|e| e.to_string())?;
    // Parent owns the pipe write end until this call finishes. EOF also handles SIGKILL.
    std::thread::Builder::new()
        .name("native-parent-liveness".into())
        .spawn(move || {
            let mut byte = [0];
            loop {
                match std::io::stdin().read(&mut byte) {
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    _ => {
                        // Use the original owned group, never the Agent's or a later group.
                        #[cfg(unix)]
                        unsafe {
                            libc::kill(-owned_group, libc::SIGKILL);
                        }
                        terminate(71)
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    let mut stream = TcpStream::connect(address).map_err(|e| e.to_string())?;
    stream.write_all(&nonce).map_err(|e| e.to_string())?;
    let mut header = [0; 4];
    stream.read_exact(&mut header).map_err(|e| e.to_string())?;
    let mut bytes = zeroize::Zeroizing::new(vec![0; frame_length(header)?]);
    stream.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    drop(bytes);
    let reply = execute(request).unwrap_or_else(|(kind, message)| Reply::Failed {
        kind,
        message: message.chars().take(4096).collect(),
    });
    let bytes = encode(&reply)?;
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(&bytes).map_err(|e| e.to_string())
}

fn execute(request: Request) -> Result<Reply, (FailureKind, String)> {
    // Download credentials must never enter a process that loads operator code.
    let request = match request {
        Request::Fetch(request) => return super::fetch::execute(request),
        other => other,
    };
    let root = match &request {
        Request::Probe { root } | Request::Run { root, .. } => root,
        Request::Fetch(_) => unreachable!("download handled before loading libraries"),
    };
    let session = load_external_operator_packages_with_dynamic_host(
        BuiltInOperatorRegistryKind::Extract,
        root,
    )
    .map_err(|e| (FailureKind::Activation, e.to_string()))?;
    let packages = session
        .report()
        .activated_packages
        .iter()
        .map(|plan| {
            let manifest = &plan.manifest;
            let entrypoint_sha256 = sha256_file(&plan.entrypoint_path)?;
            Ok(LoadedOperatorPackage {
                project_pin: ProjectOperatorPin {
                    package_id: manifest.package_id.clone(),
                    package_version: manifest.package_version.clone(),
                    manifest_sha256: sha256_file(&plan.manifest_path)?,
                    entrypoint_sha256: entrypoint_sha256.clone(),
                    operators: manifest
                        .operators
                        .iter()
                        .map(|o| ProjectOperatorEntry {
                            operator_id: o.operator_id.clone(),
                            kind: o.kind.clone(),
                        })
                        .collect(),
                },
                package_id: manifest.package_id.clone(),
                package_version: manifest.package_version.clone(),
                sdk_api_version: manifest.sdk_api_version.clone(),
                execution_abi: manifest.execution_abi.clone(),
                runtime: manifest.runtime.clone(),
                validation_status: match manifest.validation_status {
                    OperatorValidationStatus::Verified => "verified",
                    OperatorValidationStatus::Partial => "partial",
                    OperatorValidationStatus::Unverified => "unverified",
                }
                .into(),
                entrypoint_sha256,
                operator_kinds: manifest
                    .operators
                    .iter()
                    .map(|o| (o.operator_id.clone(), o.kind.clone()))
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| (FailureKind::Activation, e))?;
    let reply = match request {
        Request::Fetch(_) => unreachable!("download handled before loading libraries"),
        Request::Probe { .. } => Reply::Probed(packages),
        Request::Run {
            expected, request, ..
        } => {
            if expected != packages {
                return Err((
                    FailureKind::Identity,
                    "native package generation changed after activation".into(),
                ));
            }
            Reply::Executed(
                session
                    .run_operator(request)
                    .map_err(|e| (FailureKind::Execution, e))?,
            )
        }
    };
    // Unload/destructors are inside the same process/time boundary, before success.
    drop(session);
    Ok(reply)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(super) fn frame_length(header: [u8; 4]) -> Result<usize, String> {
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        Err("native IPC frame size out of range".into())
    } else {
        Ok(length)
    }
}

pub(super) fn encode(value: &impl Serialize) -> Result<Vec<u8>, String> {
    struct Bounded(Vec<u8>);
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_FRAME_BYTES.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("native IPC frame too large"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut bytes = Bounded(Vec::new());
    serde_json::to_writer(&mut bytes, value).map_err(|e| e.to_string())?;
    Ok(bytes.0)
}
