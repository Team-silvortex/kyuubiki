use std::io::Read;
use std::net::TcpListener;

fn main() {
    let port = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .expect("listener requires a TCP port");
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("failed to bind test listener");
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            break;
        };
        let mut buffer = [0_u8; 32];
        let _ = stream.read(&mut buffer);
    }
}
