use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Only forwards GETs to an isolated test service; one real content body is corrupted.
pub(super) struct ResultContentProxy {
    pub port: u16,
    pub paths: Arc<Mutex<Vec<String>>>,
    pub errors: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ResultContentProxy {
    pub fn start(upstream: u16, content_path: String) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let paths = Arc::new(Mutex::new(vec![]));
        let errors = Arc::new(Mutex::new(vec![]));
        let stop = Arc::new(AtomicBool::new(false));
        let (captured, failures, signal) =
            (Arc::clone(&paths), Arc::clone(&errors), Arc::clone(&stop));
        let worker = thread::spawn(move || {
            while !signal.load(Ordering::Acquire) {
                let (client, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(cause) if cause.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(cause) => {
                        failures.lock().unwrap().push(cause.to_string());
                        break;
                    }
                };
                if let Err(cause) = relay(client, upstream, &content_path, &captured) {
                    failures.lock().unwrap().push(cause.to_string());
                }
            }
        });
        Ok(Self {
            port,
            paths,
            errors,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for ResultContentProxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn header(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= 64 * 1024 {
            return Err(std::io::Error::other("test relay header cap"));
        }
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    Ok(head)
}

fn relay(
    mut client: TcpStream,
    port: u16,
    content_path: &str,
    paths: &Mutex<Vec<String>>,
) -> std::io::Result<()> {
    client.set_nonblocking(false)?;
    client.set_read_timeout(Some(Duration::from_secs(3)))?;
    client.set_write_timeout(Some(Duration::from_secs(3)))?;
    let request = header(&mut client)?;
    let text = std::str::from_utf8(&request).map_err(std::io::Error::other)?;
    let first = text.lines().next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    if parts.next() != Some("GET") {
        return Err(std::io::Error::other(
            "unexpected write reached read-only test relay",
        ));
    }
    let path = parts
        .next()
        .ok_or_else(|| std::io::Error::other("missing test route"))?;
    paths.lock().unwrap().push(path.into());
    let mut upstream = TcpStream::connect(("127.0.0.1", port))?;
    upstream.set_read_timeout(Some(Duration::from_secs(3)))?;
    upstream.set_write_timeout(Some(Duration::from_secs(3)))?;
    upstream.write_all(&request)?;
    let head = header(&mut upstream)?;
    let text = std::str::from_utf8(&head).map_err(std::io::Error::other)?;
    let size = text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>())
        })
        .ok_or_else(|| std::io::Error::other("missing test content length"))?
        .map_err(std::io::Error::other)?;
    if size > 1024 * 1024 {
        return Err(std::io::Error::other("test relay response cap"));
    }
    let mut body = vec![0; size];
    upstream.read_exact(&mut body)?;
    if path == content_path {
        let index = body
            .iter()
            .rposition(u8::is_ascii_digit)
            .ok_or_else(|| std::io::Error::other("no numeric content to corrupt"))?;
        body[index] = if body[index] == b'0' { b'1' } else { b'0' };
    }
    client.write_all(&head)?;
    client.write_all(&body)
}
