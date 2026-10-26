//! Descriptor-only protocol fixture. All mutation attempts are counted and rejected.
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct Peer {
    pub port: u16,
    stop: Arc<AtomicBool>,
    methods: Arc<Mutex<Vec<String>>>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

impl Peer {
    pub fn start(descriptor: Value) -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let methods = Arc::new(Mutex::new(Vec::new()));
        let stopped = stop.clone();
        let observed = methods.clone();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false)?;
                        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                        let request = read(&mut stream)?;
                        let method = request["method"].as_str().unwrap_or("missing");
                        observed.lock().unwrap().push(method.into());
                        let reply = if method == "describe_agent" {
                            json!({"rpc_version":1,"id":request["id"],"ok":true,"result":descriptor})
                        } else {
                            json!({"rpc_version":1,"id":request["id"],"ok":false,
                                "error":{"code":"unexpected_dispatch","message":"descriptor fixture rejects mutation"}})
                        };
                        let bytes = serde_json::to_vec(&reply)?;
                        stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
                        stream.write_all(&bytes)?;
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        });
        Ok(Self {
            port,
            stop,
            methods,
            worker: Some(worker),
        })
    }

    pub fn methods(&self) -> Vec<String> {
        self.methods.lock().unwrap().clone()
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let joined = worker.join();
            if !thread::panicking() {
                joined
                    .expect("descriptor fixture thread failed")
                    .expect("descriptor fixture I/O failed");
            }
        }
    }
}

fn read(stream: &mut TcpStream) -> io::Result<Value> {
    let mut header = [0; 4];
    stream.read_exact(&mut header)?;
    let size = u32::from_be_bytes(header) as usize;
    if size > 65536 {
        return Err(io::Error::other("oversized fixture request"));
    }
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
