use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

pub(super) const MAX_RESOLVERS: usize = 4;
pub(super) const MAX_ADDRESSES: usize = 8;
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
static RESOLVERS: OnceLock<Arc<AtomicUsize>> = OnceLock::new();

pub(super) fn remaining(deadline: Instant) -> io::Result<Duration> {
    let duration = deadline.saturating_duration_since(Instant::now());
    if duration.is_zero() {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "direct mesh request deadline exhausted",
        ))
    } else {
        Ok(duration)
    }
}

struct ResolverSlot(Arc<AtomicUsize>);
impl Drop for ResolverSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn resolve_with(
    active: &Arc<AtomicUsize>,
    deadline: Instant,
    lookup: impl FnOnce() -> io::Result<Vec<SocketAddr>> + Send + 'static,
) -> Result<Vec<SocketAddr>, String> {
    loop {
        let remaining = remaining(deadline)
            .map_err(|_| "direct mesh resolver capacity unavailable before deadline")?;
        if active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < MAX_RESOLVERS).then_some(count + 1)
            })
            .is_ok()
        {
            break;
        }
        std::thread::sleep(remaining.min(Duration::from_millis(10)));
    }
    let slot = ResolverSlot(Arc::clone(active));
    let (send, receive) = mpsc::sync_channel(1);
    // System DNS cannot be cancelled. Retain its slot until completion; late work never connects.
    std::thread::Builder::new()
        .name("direct-mesh-dns".into())
        .spawn(move || {
            let _slot = slot;
            if Instant::now() < deadline {
                let _ = send.send(lookup());
            }
        })
        .map_err(|e| e.to_string())?;
    let result = receive
        .recv_timeout(remaining(deadline).map_err(|e| e.to_string())?)
        .map_err(|_| "direct mesh DNS unavailable before deadline")?;
    remaining(deadline).map_err(|e| e.to_string())?;
    result.map_err(|e| e.to_string())
}

pub(super) fn connect(endpoint: &str, deadline: Instant) -> Result<TcpStream, String> {
    let addresses = match endpoint.parse::<SocketAddr>() {
        Ok(address) => vec![address],
        Err(_) => {
            let endpoint = endpoint.to_owned();
            resolve_with(
                RESOLVERS.get_or_init(|| Arc::new(AtomicUsize::new(0))),
                deadline,
                move || {
                    endpoint
                        .to_socket_addrs()
                        .map(|addresses| addresses.take(MAX_ADDRESSES).collect())
                },
            )?
        }
    };
    for address in addresses.into_iter().take(MAX_ADDRESSES) {
        let ip = address.ip().to_canonical();
        if address.port() == 0
            || ip.is_unspecified()
            || ip.is_multicast()
            || matches!(ip, std::net::IpAddr::V4(ip) if ip.is_broadcast())
        {
            continue;
        }
        let timeout = remaining(deadline)
            .map_err(|e| e.to_string())?
            .min(CONNECT_TIMEOUT);
        if let Ok(stream) = TcpStream::connect_timeout(&address, timeout) {
            return Ok(stream);
        }
    }
    Err(format!(
        "failed to connect to direct mesh agent {endpoint} before deadline"
    ))
}

pub(super) fn write_before(
    stream: &mut TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> io::Result<()> {
    while !bytes.is_empty() {
        stream.set_write_timeout(Some(remaining(deadline)?))?;
        match stream.write(bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    remaining(deadline).map(|_| ())
}

fn read_before(stream: &mut TcpStream, mut bytes: &mut [u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        match stream.read(bytes) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    remaining(deadline).map(|_| ())
}

pub(super) fn read_frame(
    stream: &mut TcpStream,
    deadline: Instant,
    max_bytes: usize,
) -> Result<(serde_json::Value, usize), String> {
    let mut header = [0; 4];
    read_before(stream, &mut header, deadline).map_err(|e| e.to_string())?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > max_bytes {
        return Err("direct mesh response frame exceeds limit".into());
    }
    let mut bytes = vec![0; length];
    read_before(stream, &mut bytes, deadline).map_err(|e| e.to_string())?;
    let value =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid direct mesh JSON: {e}"))?;
    remaining(deadline).map_err(|e| e.to_string())?;
    Ok((value, length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_dns_retains_bounded_slots_until_the_resolver_really_finishes() {
        let active = Arc::new(AtomicUsize::new(0));
        let mut releases = Vec::new();
        for _ in 0..MAX_RESOLVERS {
            let (send, receive) = mpsc::channel();
            let (started, observed) = mpsc::channel();
            releases.push(send);
            let slots = Arc::clone(&active);
            let request = std::thread::spawn(move || {
                resolve_with(
                    &slots,
                    Instant::now() + Duration::from_millis(150),
                    move || {
                        started.send(()).unwrap();
                        let _ = receive.recv_timeout(Duration::from_secs(3));
                        Ok(vec!["127.0.0.1:5001".parse().unwrap()])
                    },
                )
            });
            observed.recv_timeout(Duration::from_secs(1)).unwrap();
            assert!(request.join().unwrap().is_err());
        }
        assert_eq!(active.load(Ordering::Acquire), MAX_RESOLVERS);
        assert!(
            resolve_with(
                &active,
                Instant::now() + Duration::from_millis(20),
                || panic!("must not launch excess DNS")
            )
            .unwrap_err()
            .contains("capacity")
        );
        let queued_slots = Arc::clone(&active);
        let queued = std::thread::spawn(move || {
            resolve_with(
                &queued_slots,
                Instant::now() + Duration::from_secs(2),
                || Ok(vec!["127.0.0.1:5002".parse().unwrap()]),
            )
        });
        for release in releases {
            release.send(()).unwrap();
        }
        assert_eq!(
            queued.join().unwrap().unwrap(),
            vec!["127.0.0.1:5002".parse::<SocketAddr>().unwrap()]
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while active.load(Ordering::Acquire) != 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(active.load(Ordering::Acquire), 0);
        assert!(
            resolve_with(&active, Instant::now() + Duration::from_secs(1), || Ok(
                vec![]
            ))
            .unwrap()
            .is_empty()
        );
    }
}
