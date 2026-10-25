//! Test-only bounded loopback relay: forwards real frames, never invents receipts.
use kyuubiki_headless_sdk::SolverRpcClient;
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Default)]
struct Evidence {
    methods: Vec<String>,
    dropped: Option<Value>,
    errors: Vec<String>,
}

pub struct Relay {
    pub port: u16,
    stop: Arc<AtomicBool>,
    evidence: Arc<Mutex<Evidence>>,
    worker: Option<JoinHandle<()>>,
}

impl Relay {
    pub fn start(agent_port: u16, drop_before_dispatch: bool) -> io::Result<Self> {
        Self::start_with_delay(agent_port, drop_before_dispatch, Duration::ZERO)
    }

    pub fn start_with_delay(
        agent_port: u16,
        drop_before_dispatch: bool,
        reply_delay: Duration,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let evidence = Arc::new(Mutex::new(Evidence::default()));
        let stopped = stop.clone();
        let observed = evidence.clone();
        let worker = thread::spawn(move || {
            let mut drop_once = true;
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((socket, _)) => {
                        if let Err(error) = forward(
                            socket,
                            agent_port,
                            &observed,
                            &mut drop_once,
                            drop_before_dispatch,
                            reply_delay,
                        ) {
                            observed.lock().unwrap().errors.push(error.to_string());
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => {
                        observed.lock().unwrap().errors.push(error.to_string());
                        break;
                    }
                }
            }
        });
        Ok(Self {
            port,
            stop,
            evidence,
            worker: Some(worker),
        })
    }

    pub fn client(&self) -> SolverRpcClient {
        SolverRpcClient::new("127.0.0.1", self.port)
    }

    pub fn dropped_receipt(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let evidence = self.evidence.lock().unwrap();
            if evidence.dropped.is_some()
                || !evidence.errors.is_empty()
                || Instant::now() >= deadline
            {
                break;
            }
            drop(evidence);
            thread::sleep(Duration::from_millis(5));
        }
        let evidence = self.evidence.lock().unwrap();
        assert!(evidence.errors.is_empty(), "{:?}", evidence.errors);
        let actual = evidence
            .dropped
            .as_ref()
            .expect("relay must observe a real admission response");
        assert_eq!(actual["ok"], true, "{actual}");
        actual["result"].clone()
    }

    pub fn assert_one_submit(&self) {
        let evidence = self.evidence.lock().unwrap();
        assert!(evidence.errors.is_empty(), "{:?}", evidence.errors);
        assert_eq!(
            evidence
                .methods
                .iter()
                .filter(|m| m.as_str() == "submit_durable_agent_task")
                .count(),
            1
        );
        assert!(
            evidence.methods.iter().all(|m| matches!(
                m.as_str(),
                "submit_durable_agent_task" | "fetch_durable_agent_execution" | "describe_agent"
            )),
            "{:?}",
            evidence.methods
        );
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn timed(socket: &TcpStream) -> io::Result<()> {
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.set_write_timeout(Some(Duration::from_secs(3)))
}

fn read_frame(socket: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut header = [0; 4];
    socket.read_exact(&mut header)?;
    let size = u32::from_be_bytes(header) as usize;
    if size > 16 * 1024 * 1024 {
        return Err(io::Error::other("oversized test frame"));
    }
    let mut bytes = vec![0; size];
    socket.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn write_frame(socket: &mut TcpStream, bytes: &[u8]) -> io::Result<()> {
    socket.write_all(&(bytes.len() as u32).to_be_bytes())?;
    socket.write_all(bytes)
}

fn forward(
    mut incoming: TcpStream,
    port: u16,
    evidence: &Mutex<Evidence>,
    drop_once: &mut bool,
    before: bool,
    reply_delay: Duration,
) -> io::Result<()> {
    // BSD may inherit O_NONBLOCK from the listener; SO_RCVTIMEO alone cannot
    // make read_exact wait for a request whose bytes arrive after accept.
    incoming.set_nonblocking(false)?;
    timed(&incoming)?;
    let request = read_frame(&mut incoming)?;
    let decoded: Value = serde_json::from_slice(&request)?;
    let method = decoded["method"]
        .as_str()
        .ok_or_else(|| io::Error::other("missing method"))?;
    evidence.lock().unwrap().methods.push(method.into());
    let drop_reply = *drop_once && method == "submit_durable_agent_task";
    if drop_reply {
        *drop_once = false;
        if before {
            return Ok(());
        }
    }
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut upstream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
    timed(&upstream)?;
    write_frame(&mut upstream, &request)?;
    for _ in 0..64 {
        let response = read_frame(&mut upstream)?;
        let decoded: Value = serde_json::from_slice(&response)?;
        let terminal = decoded.get("event").is_none();
        if terminal && drop_reply {
            evidence.lock().unwrap().dropped = Some(decoded);
            thread::sleep(reply_delay);
            return Ok(());
        }
        write_frame(&mut incoming, &response)?;
        if terminal {
            return Ok(());
        }
    }
    Err(io::Error::other("too many test progress frames"))
}

#[test]
fn accepted_relay_socket_waits_for_delayed_request_bytes() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    timed(&client).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let accepted = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("test connection was not accepted: {error}"),
        }
    };
    // Reproduce BSD accept inheritance deterministically on every platform.
    accepted.set_nonblocking(true).unwrap();
    let evidence = Arc::new(Mutex::new(Evidence::default()));
    let observed = evidence.clone();
    let (ready, started) = std::sync::mpsc::channel();
    let worker = thread::spawn(move || {
        ready.send(()).unwrap();
        forward(accepted, 0, &observed, &mut true, true, Duration::ZERO)
    });
    started.recv_timeout(Duration::from_secs(3)).unwrap();
    thread::sleep(Duration::from_millis(50));
    let sent = write_frame(&mut client, br#"{"method":"submit_durable_agent_task"}"#);
    let forwarded = worker.join().unwrap();
    assert!(
        forwarded.is_ok(),
        "delayed request was rejected: {forwarded:?}"
    );
    sent.unwrap();
    assert_eq!(
        evidence.lock().unwrap().methods,
        ["submit_durable_agent_task"]
    );
}
