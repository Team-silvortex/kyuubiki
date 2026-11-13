//! Bounded real-byte HTTP transfers, with an explicit mid-artifact interruption barrier.
use kyuubiki_operator_sdk::{current_platform_library_file_name, current_platform_target_id};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const PACKAGE: &str = "extract.template_summary";
pub const TOTAL: usize = 64 * 1024 * 1024;
pub const PREFIX: usize = 16 * 1024 * 1024;
static BLOCK: [u8; 64 * 1024] = [0x5a; 64 * 1024];

pub struct DownloadFixture {
    pub url: String,
    pub digest: String,
    pub entered: Receiver<()>,
    pub closed: Receiver<()>,
    interrupt: Sender<()>,
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<String>>>,
    server: Option<JoinHandle<()>>,
}

impl DownloadFixture {
    pub fn large() -> Self {
        Self::start(None, true)
    }
    pub fn complete_large() -> Self {
        Self::start(None, false)
    }
    pub fn complete(bytes: Vec<u8>) -> Self {
        Self::start(Some(bytes), false)
    }

    fn start(bytes: Option<Vec<u8>>, hold: bool) -> Self {
        let size = bytes.as_ref().map_or(TOTAL, Vec::len);
        let digest = if let Some(bytes) = &bytes {
            sha256(bytes)
        } else {
            let mut hash = Sha256::new();
            for _ in 0..TOTAL / BLOCK.len() {
                hash.update(&BLOCK[..]);
            }
            format!("{:x}", hash.finalize())
        };
        let target = current_platform_target_id();
        let library = current_platform_library_file_name("kyuubiki_operator_template");
        let base = format!("/api/v1/central/operator-packages/{PACKAGE}/0.1.0/{target}");
        let manifest = serde_json::to_vec(&json!({
            "schema_version":"kyuubiki.operator-package/v1",
            "sdk_api_version":"kyuubiki.operator-sdk/v1",
            "execution_abi":"kyuubiki.operator-json-c/v1",
            "package_id":PACKAGE, "package_version":"0.1.0",
            "minimum_host_version":"1.15.0", "validation_status":"partial",
            "validation_notes":"Interrupted download qualification fixture.",
            "runtime":"rust_crate", "entrypoint":format!("lib/{library}"),
            "operators":[{"operator_id":PACKAGE,"kind":"extract",
                "entry_symbol":"run_template_operator_json"}]
        }))
        .unwrap();
        let resolution = serde_json::to_vec(&json!({
            "schema_version":"kyuubiki.operator-package-resolution/v1",
            "package_ref":format!("orchestra://operator-package/{PACKAGE}"),
            "package_id":PACKAGE, "package_version":"0.1.0",
            "sdk_api_version":"kyuubiki.operator-sdk/v1",
            "execution_abi":"kyuubiki.operator-json-c/v1", "target":target,
            "authority_mode":"bound_orchestra", "cache_scope":"task_required_disposable",
            "distribution_sha256":"d".repeat(64),
            "manifest":{"path":"kyuubiki-operator.json", "sha256":sha256(&manifest),
                "size_bytes":manifest.len(), "download_path":format!("{base}/manifest")},
            "entrypoint":{"path":library,"sha256":digest,"size_bytes":size,
                "download_path":format!("{base}/entrypoint")}
        }))
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (entered_tx, entered) = mpsc::channel();
        let (closed_tx, closed) = mpsc::channel();
        let (interrupt, interrupt_rx) = mpsc::channel();
        let stopping = stop.clone();
        let seen = requests.clone();
        let server = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let path = read_path(&mut stream);
                if !path.starts_with(&base) {
                    let _ = response(&mut stream, b"{}");
                    continue;
                }
                seen.lock().unwrap().push(path.clone());
                if path == format!("{base}/resolve") {
                    response(&mut stream, &resolution).unwrap();
                } else if path == format!("{base}/manifest") {
                    response(&mut stream, &manifest).unwrap();
                } else {
                    assert_eq!(path, format!("{base}/entrypoint"));
                    if let Some(bytes) = &bytes {
                        response(&mut stream, bytes).unwrap();
                        continue;
                    }
                    headers(&mut stream, TOTAL).unwrap();
                    for _ in 0..(if hold { PREFIX } else { TOTAL }) / BLOCK.len() {
                        stream.write_all(&BLOCK).unwrap();
                    }
                    stream.flush().unwrap();
                    if !hold {
                        continue;
                    }
                    entered_tx.send(()).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_millis(50)))
                        .unwrap();
                    let deadline = Instant::now() + Duration::from_secs(15);
                    while !stopping.load(Ordering::Acquire) && Instant::now() < deadline {
                        if interrupt_rx.try_recv().is_ok() {
                            break;
                        }
                        match stream.read(&mut [0]) {
                            Ok(0) => {
                                let _ = closed_tx.send(());
                                break;
                            }
                            Err(e)
                                if matches!(
                                    e.kind(),
                                    std::io::ErrorKind::ConnectionReset
                                        | std::io::ErrorKind::ConnectionAborted
                                ) =>
                            {
                                let _ = closed_tx.send(());
                                break;
                            }
                            Err(e)
                                if matches!(
                                    e.kind(),
                                    std::io::ErrorKind::WouldBlock
                                        | std::io::ErrorKind::TimedOut
                                        | std::io::ErrorKind::Interrupted
                                ) => {}
                            other => panic!("unexpected download socket result: {other:?}"),
                        }
                    }
                }
            }
        });
        Self {
            url,
            digest,
            entered,
            closed,
            interrupt,
            stop,
            requests,
            server: Some(server),
        }
    }

    pub fn disconnect(&self) {
        self.interrupt.send(()).unwrap();
    }
    pub fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Drop for DownloadFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

fn read_path(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        assert!(bytes.len() < 16384);
    }
    String::from_utf8(bytes)
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .into()
}

fn headers(stream: &mut TcpStream, size: usize) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"
    )
}

fn response(stream: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    headers(stream, bytes.len())?;
    stream.write_all(bytes)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
