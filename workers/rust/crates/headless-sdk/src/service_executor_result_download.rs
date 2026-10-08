use crate::HeadlessExecutorError;
use crate::service_executor::{parse_http_url, sanitize_header_value};
use crate::service_executor_deadline::{remaining_timeout, write_before_deadline};
use crate::service_executor_http::connect_service_stream_with_deadline;
use crate::service_executor_result_artifact::{
    RESULT_MEDIA_TYPE, ResultArtifact, ResultReadPolicy, failure,
};
use crate::service_executor_result_stream::receive_content;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn download_result(
    base_url: &str,
    token: Option<&str>,
    artifact: &ResultArtifact,
    policy: ResultReadPolicy,
) -> Result<Value, HeadlessExecutorError> {
    let deadline = Instant::now() + policy.timeout;
    let endpoint =
        parse_http_url(base_url).map_err(|_| failure("invalid result service authority"))?;
    let token = sanitize_header_value(token, "api token")
        .map_err(|_| failure("invalid result service credential header"))?;
    let mut stream = connect_service_stream_with_deadline(
        &endpoint.host,
        endpoint.port,
        policy.timeout,
        "result artifact readback",
        Some(deadline),
    )
    .map_err(|_| failure("could not connect within the result readback deadline"))?;
    // The authenticated destination is always the configured control plane, never a descriptor URL.
    let mut request = format!(
        "GET /api/v1/result-artifacts/{}/content HTTP/1.1\r\nHost: {}:{}\r\nAccept: {RESULT_MEDIA_TYPE}\r\nAccept-Encoding: identity\r\nConnection: close\r\n",
        artifact.id, endpoint.host, endpoint.port
    );
    if let Some(token) = token {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    request.push_str("\r\n");
    write_before_deadline(&mut stream, request.as_bytes(), deadline, policy.timeout)
        .map_err(|_| failure("result read request could not be sent within its deadline"))?;
    let mut reader = BufReader::with_capacity(
        64 * 1024,
        DeadlineReader {
            stream,
            deadline,
            timeout: policy.timeout,
        },
    );
    let mut spool = ResultSpool::create()?;
    let mut digest = Sha256::new();
    receive_content(&mut reader, artifact.size, |bytes| {
        spool.file_mut().write_all(bytes)?;
        digest.update(bytes);
        Ok(())
    })
    .map_err(|cause| failure(format!("result content rejected: {cause}")))?;
    if format!("{:x}", digest.finalize()) != artifact.id {
        return Err(failure(
            "result content SHA-256 does not match its descriptor",
        ));
    }
    remaining_timeout(deadline, policy.timeout)
        .map_err(|_| failure("result readback deadline exhausted before decoding"))?;
    spool
        .file_mut()
        .seek(SeekFrom::Start(0))
        .map_err(|_| failure("could not rewind owned result spool"))?;
    let result = serde_json::from_reader(BufReader::new(spool.file_mut()))
        .map_err(|_| failure("verified result bytes are not valid JSON"))?;
    remaining_timeout(deadline, policy.timeout)
        .map_err(|_| failure("result readback deadline exhausted during decoding"))?;
    Ok(result)
}

struct DeadlineReader {
    stream: TcpStream,
    deadline: Instant,
    timeout: Duration,
}

impl Read for DeadlineReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let timeout = remaining_timeout(self.deadline, self.timeout).map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "result readback deadline exhausted",
            )
        })?;
        self.stream.set_read_timeout(Some(timeout))?;
        self.stream.read(bytes)
    }
}

pub(crate) struct ResultSpool {
    file: Option<File>,
    pub path: PathBuf,
}

impl ResultSpool {
    pub fn create() -> Result<Self, HeadlessExecutorError> {
        for _ in 0..16 {
            let nonce = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "kyuubiki-result-{}-{nonce}.json",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.read(true).write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        file: Some(file),
                        path,
                    });
                }
                Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(failure("could not create private result spool")),
            }
        }
        Err(failure("could not reserve an exclusive result spool"))
    }

    pub fn file_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("owned spool remains open until drop")
    }
}

impl Drop for ResultSpool {
    fn drop(&mut self) {
        // Close before removal so the same ownership cleanup works on Windows.
        drop(self.file.take());
        let _ = std::fs::remove_file(&self.path);
    }
}
