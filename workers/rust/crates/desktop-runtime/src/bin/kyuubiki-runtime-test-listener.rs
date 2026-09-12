use std::io::Read;
use std::net::TcpListener;

fn main() {
    let argument = std::env::args()
        .nth(1)
        .expect("listener requires an argument");
    if argument == "--exit" {
        std::process::exit(17);
    }
    if let Ok(delay) = std::env::var("KYUUBIKI_TEST_LISTENER_DELAY_MS") {
        std::thread::sleep(std::time::Duration::from_millis(delay.parse().unwrap()));
    }
    let port = argument
        .parse::<u16>()
        .ok()
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
