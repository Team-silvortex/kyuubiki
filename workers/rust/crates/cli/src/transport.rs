use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use kyuubiki_protocol::{JobStatus, ProgressEvent, RpcProgress, RpcResponse};

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
    writer: &Arc<Mutex<TcpStream>>,
    reply: AgentReply,
) -> Result<(), String> {
    match reply {
        AgentReply::Stream(progress_frames, final_response) => {
            for progress_frame in progress_frames {
                write_json_frame(writer, &progress_frame)?;
            }

            write_json_frame(writer, &final_response)?;
            Ok(())
        }
    }
}

fn write_json_frame<T: serde::Serialize>(
    writer: &Arc<Mutex<TcpStream>>,
    payload: &T,
) -> Result<(), String> {
    let encoded = serde_json::to_vec(payload)
        .map_err(|error| format!("failed to serialize response frame: {error}"))?;

    let mut guard = writer
        .lock()
        .map_err(|_| "failed to lock tcp writer".to_string())?;

    write_frame(&mut guard, &encoded)
        .map_err(|error| format!("failed to write response frame: {error}"))
}

impl HeartbeatHandle {
    pub(crate) fn spawn(writer: Arc<Mutex<TcpStream>>, request_id: String, job_id: String) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let join_handle = thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1_000));

                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

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

                if write_json_frame(&writer, &heartbeat).is_err() {
                    break;
                }
            }
        });

        Self {
            running,
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}
