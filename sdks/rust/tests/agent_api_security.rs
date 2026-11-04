use kyuubiki_agent_transport::ServerSecurity;
use kyuubiki_headless_sdk::{AgentApiSecurity, SolverRpcClient};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

const TOKEN: &str = "sdk-test-agent-token-0123456789abcdef";

#[test]
fn rust_headless_uses_tls_auth_without_putting_credentials_into_rpc() {
    for valid in [true, false] {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = certificate.cert.pem();
        let server =
            ServerSecurity::from_pem(&pem, &certificate.signing_key.serialize_pem(), TOKEN.into())
                .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let connection = server.accept(socket, Duration::from_secs(2));
            if !valid {
                assert!(connection.is_err());
                return;
            }
            let mut stream = connection.unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let size = u32::from_be_bytes(length) as usize;
            assert!(size < 4096);
            let mut bytes = vec![0; size];
            stream.read_exact(&mut bytes).unwrap();
            assert!(!String::from_utf8_lossy(&bytes).contains(TOKEN));
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(request["method"], "describe_agent");
            let reply = serde_json::to_vec(&json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"authenticated":true}})).unwrap();
            stream
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&reply).unwrap();
        });
        let token = if valid {
            TOKEN
        } else {
            "wrong-sdk-token-01234567890123456789"
        };
        let security = AgentApiSecurity::from_pem(&pem, "localhost", token.into()).unwrap();
        assert!(!format!("{security:?}").contains(token));
        let result = SolverRpcClient::new("127.0.0.1", port)
            .with_api_security(security)
            .describe_agent();
        if valid {
            assert_eq!(result.unwrap().result["authenticated"], true);
        } else {
            assert!(!result.err().unwrap().to_string().contains(token));
        }
        worker.join().unwrap();
    }
}
