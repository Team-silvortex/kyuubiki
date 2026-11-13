use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

const TOKEN: &str = "test-only-agent-token-0123456789abcdef";

fn fixture() -> (String, String) {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    (
        certificate.cert.pem(),
        certificate.signing_key.serialize_pem(),
    )
}

#[test]
fn api_tokens_are_validated_and_debug_redacted() {
    for token in ["", "short", "token with whitespace", &"x".repeat(257)] {
        assert!(ApiToken::new(token.to_owned()).is_err());
    }
    let token = ApiToken::new(TOKEN.to_owned()).unwrap();
    assert!(!format!("{token:?}").contains(TOKEN));
}

#[test]
fn tls_handshake_enables_low_latency_on_both_endpoints() {
    let (certificate, key) = fixture();
    let server = ServerSecurity::from_pem(&certificate, &key, TOKEN.into()).unwrap();
    let client = ClientSecurity::from_pem(&certificate, "localhost", TOKEN.into()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let (socket, _) = listener.accept().unwrap();
            socket.set_nodelay(false).unwrap();
            let observer = socket.try_clone().unwrap();
            let stream = server.accept(socket, Duration::from_secs(2)).unwrap();
            assert!(stream.authenticated());
            assert!(
                observer.nodelay().unwrap(),
                "server must not delay small TLS frames"
            );
        });
        let socket = TcpStream::connect(address).unwrap();
        socket.set_nodelay(false).unwrap();
        let observer = socket.try_clone().unwrap();
        let stream = client.connect(socket, Duration::from_secs(2)).unwrap();
        assert!(stream.authenticated());
        assert!(
            observer.nodelay().unwrap(),
            "client must not delay small TLS frames"
        );
        worker.join().unwrap();
    });
}

#[test]
fn secure_session_authenticates_before_application_bytes() {
    let (certificate, key) = fixture();
    let server = ServerSecurity::from_pem(&certificate, &key, TOKEN.to_owned()).unwrap();
    let client = ClientSecurity::from_pem(&certificate, "localhost", TOKEN.to_owned()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        let mut stream = server.accept(socket, Duration::from_secs(2)).unwrap();
        assert!(stream.authenticated());
        let mut request = [0; 4];
        stream.read_exact(&mut request).unwrap();
        assert_eq!(&request, b"ping");
        stream.try_clone().unwrap().write_all(b"pong").unwrap();
    });
    let mut stream = client
        .connect(TcpStream::connect(address).unwrap(), Duration::from_secs(2))
        .unwrap();
    stream.write_all(b"ping").unwrap();
    let mut reply = [0; 4];
    stream.read_exact(&mut reply).unwrap();
    assert_eq!(&reply, b"pong");
    worker.join().unwrap();
}

#[test]
fn wrong_token_untrusted_certificate_and_wrong_name_fail_closed() {
    for failure in ["token", "certificate", "name"] {
        let (certificate, key) = fixture();
        let server = ServerSecurity::from_pem(&certificate, &key, TOKEN.to_owned()).unwrap();
        let trust = if failure == "certificate" {
            fixture().0
        } else {
            certificate
        };
        let name = if failure == "name" {
            "wrong.example"
        } else {
            "localhost"
        };
        let token = if failure == "token" {
            "wrong-token-0123456789abcdef0123456789"
        } else {
            TOKEN
        };
        let client = ClientSecurity::from_pem(&trust, name, token.to_owned()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            assert!(server.accept(socket, Duration::from_secs(2)).is_err());
        });
        let error = client
            .connect(TcpStream::connect(address).unwrap(), Duration::from_secs(2))
            .err()
            .unwrap();
        assert!(!error.to_string().contains(token));
        worker.join().unwrap();
    }
}

#[test]
fn tls_without_authentication_cannot_send_rpc_or_oversized_prefaces() {
    use rustls::pki_types::{CertificateDer, pem::PemObject};
    use std::sync::Arc;
    for payload in [
        Some(br#"{"rpc_version":1,"id":"bypass","method":"ping","params":{}}"#.as_slice()),
        None,
    ] {
        let (certificate, key) = fixture();
        let server = ServerSecurity::from_pem(&certificate, &key, TOKEN.into()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            assert!(server.accept(socket, Duration::from_secs(2)).is_err());
        });
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(CertificateDer::from_pem_slice(certificate.as_bytes()).unwrap())
            .unwrap();
        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        config.alpn_protocols = vec![b"kyuubiki-agent/1".to_vec()];
        let tls = rustls::ClientConnection::new(Arc::new(config), "localhost".try_into().unwrap())
            .unwrap();
        let mut stream = Connection::handshake(
            TcpStream::connect(address).unwrap(),
            rustls::Connection::Client(tls),
            Duration::from_secs(2),
        )
        .unwrap();
        stream
            .write_all(
                &payload
                    .map(|bytes| bytes.len() as u32)
                    .unwrap_or(u32::MAX)
                    .to_be_bytes(),
            )
            .unwrap();
        if let Some(payload) = payload {
            stream.write_all(payload).unwrap();
        }
        let mut reply = [0; 4];
        assert!(stream.read_exact(&mut reply).is_err());
        worker.join().unwrap();
    }
}
