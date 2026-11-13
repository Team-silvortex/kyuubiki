use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use kyuubiki_protocol::OPERATOR_DEPLOYMENT_SCHEMA;
use std::io::{Read, Write};

fn pin() -> Value {
    json!({"package_id":"template", "package_version":"1.0.0", "manifest_sha256":"a".repeat(64),
        "entrypoint_sha256":"b".repeat(64), "operators":[{"operator_id":"sum", "kind":"transform"}]})
}
fn gateway(endpoint: &str) -> DirectMeshGateway {
    DirectMeshGateway {
        config: Arc::new(GatewayConfig {
            enabled: true,
            token: Some("gateway-token".into()),
            configured_endpoints: vec![endpoint.into()],
            allow_request_endpoints: false,
        }),
        results: Arc::new(Mutex::new(VecDeque::new())),
    }
}
fn request(endpoint: &str) -> HttpRequest {
    HttpRequest { method:"POST".into(), target:"/api/direct-mesh/operator-package".into(),
        path:"/api/direct-mesh/operator-package".into(),
        headers:[("authorization".into(),"Bearer gateway-token".into())].into(),
        body:serde_json::to_vec(&json!({"endpoints":[endpoint],"request":{
            "package":pin(),"expected_process_instance_id":"process-1","operation":{"action":"inspect"}
        }})).unwrap(),
    }
}

#[test]
fn package_gateway_rejects_plaintext_remote_and_extra_credentials() {
    let endpoint = "192.0.2.1:5001";
    let gateway = gateway(endpoint);
    let mut request = request(endpoint);
    let response = gateway.handle(&request);
    assert_eq!(response.status, 403);
    assert!(
        String::from_utf8(response.body)
            .unwrap()
            .contains("package_deployment_requires_tls")
    );
    let mut body: Value = serde_json::from_slice(&request.body).unwrap();
    body["request"]["store_token"] = json!("never-pass-store-token");
    request.body = serde_json::to_vec(&body).unwrap();
    let response = gateway.handle(&request);
    assert_eq!(response.status, 400);
    assert!(
        !String::from_utf8(response.body)
            .unwrap()
            .contains("never-pass-store-token")
    );
    body["request"]
        .as_object_mut()
        .unwrap()
        .remove("store_token");
    for endpoints in [json!([]), Value::Null] {
        body["endpoints"] = endpoints;
        request.body = serde_json::to_vec(&body).unwrap();
        assert_eq!(
            gateway.handle(&request).status,
            400,
            "no implicit target selection"
        );
    }
    body.as_object_mut().unwrap().remove("endpoints");
    body["store_token"] = json!("never-pass-store-token");
    request.body = serde_json::to_vec(&body).unwrap();
    assert_eq!(gateway.handle(&request).status, 400);
}

#[test]
fn package_gateway_uses_tls_and_checks_loaded_readiness_and_process_fence() {
    for case in [
        "ready",
        "wrong_process",
        "not_loaded",
        "unknown_state",
        "overflow",
    ] {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = certificate.cert.pem();
        let token = "package-test-agent-token-0123456789abcdef";
        let server = kyuubiki_agent_transport::ServerSecurity::from_pem(
            &pem,
            &certificate.signing_key.serialize_pem(),
            token.into(),
        )
        .unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        let worker = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let mut stream = server.accept(socket, Duration::from_secs(5)).unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let size = u32::from_be_bytes(length) as usize;
            assert!(size < 4096);
            let mut bytes = vec![0; size];
            stream.read_exact(&mut bytes).unwrap();
            assert!(!String::from_utf8_lossy(&bytes).contains(token));
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(request["method"], "manage_operator_package");
            let mut status = json!({"schema_version":OPERATOR_DEPLOYMENT_SCHEMA, "package":pin(),
                "platform":"macos-aarch64", "state":"ready", "installed":pin(), "active":pin(),
                "received_bytes":0,"total_bytes":0,"process_instance_id":"process-1","deployment_allowed":true});
            match case {
                "wrong_process" => status["process_instance_id"] = json!("process-2"),
                "not_loaded" => status["active"] = Value::Null,
                "unknown_state" => status["state"] = json!("probably_ready"),
                "overflow" => status["received_bytes"] = json!(33554433_u64),
                _ => {}
            }
            let response = serde_json::to_vec(
                &json!({"rpc_version":1,"id":request["id"],"ok":true,"result":status}),
            )
            .unwrap();
            stream
                .write_all(&(response.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&response).unwrap();
        });
        let mut request = request(&endpoint);
        for (key, value) in [
            ("x-kyuubiki-agent-token", token.into()),
            ("x-kyuubiki-agent-endpoint", endpoint.clone()),
            ("x-kyuubiki-agent-server-name", "localhost".into()),
            ("x-kyuubiki-agent-ca", STANDARD.encode(&pem)),
        ] {
            request.headers.insert(key.into(), value);
        }
        let response = gateway(&endpoint).handle(&request);
        worker.join().unwrap();
        assert_eq!(
            response.status,
            if case == "ready" { 200 } else { 502 },
            "{case}"
        );
        assert!(!String::from_utf8(response.body).unwrap().contains(token));
    }
}
