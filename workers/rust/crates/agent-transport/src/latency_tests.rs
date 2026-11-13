use super::*;
use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

const CALLS: usize = 32;
const TIMEOUT: Duration = Duration::from_secs(5);
const TOKEN: &str = "test-only-latency-token-0123456789abcdef";

fn accept(listener: &TcpListener) -> io::Result<TcpStream> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                socket.set_nonblocking(false)?;
                return Ok(socket);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    }
}

fn write_split_frame(stream: &mut Connection, payload: &[u8]) -> io::Result<()> {
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(payload)
}

fn read_frame(stream: &mut Connection) -> io::Result<Vec<u8>> {
    let mut header = [0; 4];
    stream.read_exact(&mut header)?;
    let size = u32::from_be_bytes(header) as usize;
    if size > 256 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut body = vec![0; size];
    stream.read_exact(&mut body)?;
    Ok(body)
}

fn stats(samples: &[Duration]) -> Value {
    assert!(!samples.is_empty());
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let percentile =
        |percent: usize| sorted[(sorted.len() * percent).div_ceil(100) - 1].as_secs_f64() * 1000.0;
    json!({"count":sorted.len(),"p50_ms":percentile(50),"p95_ms":percentile(95)})
}

#[test]
fn latency_percentiles_use_nearest_rank() {
    let values = [4, 1, 3, 2].map(Duration::from_millis);
    assert_eq!(stats(&values), json!({"count":4,"p50_ms":2.0,"p95_ms":4.0}));
}

#[test]
#[ignore = "cold TLS/auth/split-frame latency experiment; run alone on each host"]
fn cold_tls_auth_and_split_frame_latency() {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let server = ServerSecurity::from_pem(
        &certificate.cert.pem(),
        &certificate.signing_key.serialize_pem(),
        TOKEN.into(),
    )
    .unwrap();
    let client =
        ClientSecurity::from_pem(&certificate.cert.pem(), "localhost", TOKEN.into()).unwrap();
    for round in 1..=3 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let mut connections = Vec::new();
        let mut authentication = Vec::new();
        let mut exchange = Vec::new();
        let mut total = Vec::new();
        thread::scope(|scope| {
            let worker = scope.spawn(|| {
                for n in 0..CALLS {
                    let mut stream = server.accept(accept(&listener).unwrap(), TIMEOUT).unwrap();
                    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
                    stream.set_write_timeout(Some(TIMEOUT)).unwrap();
                    let body = read_frame(&mut stream).unwrap();
                    assert_eq!(body, format!("latency-{round}-{n}").as_bytes());
                    write_split_frame(&mut stream, &body).unwrap();
                }
            });
            for n in 0..CALLS {
                let started = Instant::now();
                let socket = TcpStream::connect_timeout(&address, TIMEOUT).unwrap();
                connections.push(started.elapsed());
                let auth_started = Instant::now();
                let mut stream = client.connect(socket, TIMEOUT).unwrap();
                authentication.push(auth_started.elapsed());
                stream.set_read_timeout(Some(TIMEOUT)).unwrap();
                stream.set_write_timeout(Some(TIMEOUT)).unwrap();
                let exchange_started = Instant::now();
                let payload = format!("latency-{round}-{n}");
                write_split_frame(&mut stream, payload.as_bytes()).unwrap();
                assert_eq!(read_frame(&mut stream).unwrap(), payload.as_bytes());
                exchange.push(exchange_started.elapsed());
                total.push(started.elapsed());
            }
            worker.join().unwrap();
        });
        println!(
            "{}",
            json!({"schema_version":"kyuubiki.agent-cold-tls-latency/v1", "round":round,
                "mode":"cold_connection_tls13_token_split_frame", "calls":CALLS,
                "connect":stats(&connections), "tls_and_auth":stats(&authentication),
                "frame_exchange":stats(&exchange), "total":stats(&total)})
        );
    }
}
