// Test-only descendant. The test's socket is a lifeline, including on assertion failure.
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

fn main() {
    let mut stream = TcpStream::connect(std::env::args().nth(1).unwrap()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    stream.write_all(&std::process::id().to_be_bytes()).unwrap();
    let mut ready = std::io::stdout();
    ready.write_all(&[1]).unwrap();
    ready.flush().unwrap();
    let _ = stream.read(&mut [0]);
}
