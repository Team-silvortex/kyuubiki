use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};

pub(super) fn read_health_request(stream: &mut TcpStream) -> Vec<u8> {
    // Accepted sockets can inherit nonblocking mode on macOS.
    stream.set_nonblocking(false).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut request = Vec::with_capacity(256);
    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
        assert!(
            request.len() < 4096,
            "health request headers exceed fixture budget"
        );
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "health request header deadline exceeded"
        );
        stream.set_read_timeout(Some(remaining)).unwrap();
        let mut chunk = [0; 256];
        let capacity = chunk.len().min(4096 - request.len());
        let count = match stream.read(&mut chunk[..capacity]) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("read health request headers: {error}"),
        };
        assert_ne!(count, 0, "health request ended before complete headers");
        request.extend_from_slice(&chunk[..count]);
    }
    request
}
