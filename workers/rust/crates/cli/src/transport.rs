use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use kyuubiki_protocol::{JobStatus, ProgressEvent, RpcProgress, RpcResponse};

use crate::agent_lifecycle;
use crate::agent_reply_writer::SharedReplyWriter;
use crate::agent_state::register_execution_cancel;

#[cfg(test)]
#[path = "tests/reply_delivery.rs"]
mod reply_delivery_tests;

pub(crate) enum FrameReadError {
    ConnectionClosed,
    Io(std::io::Error),
}

pub(crate) enum AgentReply {
    Stream(Vec<RpcProgress>, RpcResponse),
}

pub(crate) struct HeartbeatHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

pub(crate) fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, FrameReadError> {
    let mut header = [0_u8; 4];

    match stream.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(FrameReadError::ConnectionClosed);
        }
        Err(error) => return Err(FrameReadError::Io(error)),
    }

    let frame_length = u32::from_be_bytes(header) as usize;
    let mut payload = vec![0_u8; frame_length];
    stream
        .read_exact(&mut payload)
        .map_err(FrameReadError::Io)?;

    Ok(payload)
}

pub(crate) fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let frame_length = u32::try_from(payload.len()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "payload too large for 4-byte frame length",
        )
    })?;

    stream.write_all(&frame_length.to_be_bytes())?;
    stream.write_all(payload)
}

pub(crate) fn frame_error_message(error: FrameReadError) -> String {
    match error {
        FrameReadError::ConnectionClosed => "connection closed".to_string(),
        FrameReadError::Io(error) => error.to_string(),
    }
}

pub(crate) fn write_agent_reply(
    writer: &SharedReplyWriter,
    reply: AgentReply,
) -> Result<(), String> {
    let deadline = Instant::now() + writer.timeout;
    let result = (|| match reply {
        AgentReply::Stream(progress_frames, final_response) => {
            for progress_frame in progress_frames {
                write_json_frame(writer, &progress_frame, deadline)?;
            }

            write_json_frame(writer, &final_response, deadline)?;
            Ok(())
        }
    })();
    writer.finish(result)
}

fn write_json_frame<T: serde::Serialize>(
    writer: &SharedReplyWriter,
    payload: &T,
    deadline: Instant,
) -> Result<(), String> {
    let encoded = serde_json::to_vec(payload)
        .map_err(|error| format!("failed to serialize response frame: {error}"))?;

    let mut guard = writer
        .stream
        .lock()
        .map_err(|_| "failed to lock tcp writer".to_string())?;

    let length = u32::try_from(encoded.len())
        .map_err(|_| "payload too large for 4-byte frame length".to_string())?;
    let result = write_until(&mut guard, &length.to_be_bytes(), deadline)
        .and_then(|()| write_until(&mut guard, &encoded, deadline))
        .map_err(|error| format!("failed to write response frame: {error}"));
    if result.is_err() {
        // A partial heartbeat must not be followed by a fresh final-response frame.
        let _ = guard.shutdown(std::net::Shutdown::Both);
    }
    result
}

fn write_until(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    while !bytes.is_empty() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(Error::new(
                ErrorKind::TimedOut,
                "response delivery deadline exceeded",
            ));
        }
        stream.set_write_timeout(Some(remaining))?;
        match stream.write(bytes) {
            Ok(0) => {
                return Err(Error::new(
                    ErrorKind::WriteZero,
                    "response connection closed",
                ));
            }
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

impl HeartbeatHandle {
    pub(crate) fn spawn(
        writer: SharedReplyWriter,
        request_id: String,
        job_id: String,
        execution_guard: agent_lifecycle::ExecutionGuard,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let join_handle = thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                thread::park_timeout(Duration::from_millis(1_000));

                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                let _ = agent_lifecycle::mark_progress(&execution_guard);

                let heartbeat = RpcProgress::heartbeat(
                    request_id.clone(),
                    ProgressEvent {
                        job_id: job_id.clone(),
                        stage: JobStatus::Solving,
                        progress: 0.7,
                        residual: None,
                        iteration: None,
                        peak_memory: None,
                        message: Some("agent heartbeat: solver still active".to_string()),
                    },
                );

                if write_json_frame(&writer, &heartbeat, Instant::now() + writer.timeout).is_err() {
                    register_execution_cancel(request_id.clone());
                    break;
                }
            }
        });

        Self {
            running,
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn stop(self) {
        drop(self);
    }
}

impl Drop for HeartbeatHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            join_handle.thread().unpark();
            let _ = join_handle.join();
        }
    }
}
