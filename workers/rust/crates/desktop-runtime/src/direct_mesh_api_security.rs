use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use kyuubiki_agent_transport::ClientSecurity;

const HEADERS: [&str; 4] = [
    "x-kyuubiki-agent-token",
    "x-kyuubiki-agent-endpoint",
    "x-kyuubiki-agent-server-name",
    "x-kyuubiki-agent-ca",
];

pub(super) fn security_for(
    request: &HttpRequest,
    endpoints: &[String],
) -> Result<Option<Arc<ClientSecurity>>, HttpResponse> {
    let values = HEADERS.map(|name| request.headers.get(name));
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    let reject = || {
        json_response(
            400,
            json!({"error":"invalid_agent_api_security", "message":"TLS trust, server name, token and one exact pinned Agent endpoint are required", "dispatched":false}),
        )
    };
    let [Some(token), Some(endpoint), Some(server_name), Some(ca)] = values else {
        return Err(reject());
    };
    if endpoints.len() != 1
        || endpoints[0] != *endpoint
        || ca.len() > 48 * 1024
        || server_name.len() > 253
    {
        return Err(reject());
    }
    let pem = STANDARD.decode(ca).map_err(|_| reject())?;
    let pem = std::str::from_utf8(&pem).map_err(|_| reject())?;
    ClientSecurity::from_pem(pem, server_name, token.clone())
        .map(|security| Some(Arc::new(security)))
        .map_err(|_| reject())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn gui_gateway_verifies_native_tls_and_keeps_token_out_of_task_and_response() {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = certificate.cert.pem();
        let token = "gui-test-agent-token-0123456789abcdef";
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
            let mut stream = server.accept(socket, Duration::from_secs(2)).unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let size = u32::from_be_bytes(length) as usize;
            assert!(size < 4096);
            let mut bytes = vec![0; size];
            stream.read_exact(&mut bytes).unwrap();
            assert!(!String::from_utf8_lossy(&bytes).contains(token));
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(request["method"], "describe_agent");
            let access = kyuubiki_protocol::AgentNetworkAccessPolicy::default()
                .connection_access(true)
                .with_tls_token();
            let response = serde_json::to_vec(&json!({"rpc_version":1,"id":request["id"],"ok":true,"result":{"connection_access":access}})).unwrap();
            stream
                .write_all(&(response.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&response).unwrap();
        });
        let gateway = DirectMeshGateway {
            config: Arc::new(GatewayConfig {
                enabled: true,
                token: Some("gateway-token".into()),
                configured_endpoints: vec![endpoint.clone()],
                allow_request_endpoints: false,
            }),
            results: Arc::new(Mutex::new(VecDeque::new())),
        };
        let mut request = HttpRequest {
            method: "POST".into(),
            target: "/api/direct-mesh/agents".into(),
            path: "/api/direct-mesh/agents".into(),
            headers: Default::default(),
            body: serde_json::to_vec(&json!({"endpoints":[endpoint]})).unwrap(),
        };
        for (name, value) in HEADERS.into_iter().zip([
            token.into(),
            endpoint,
            "localhost".into(),
            STANDARD.encode(&pem),
        ]) {
            request.headers.insert(name.into(), value);
        }
        assert_eq!(
            gateway.handle(&request).status,
            401,
            "Agent token must not authorize the GUI gateway"
        );
        request
            .headers
            .insert("authorization".into(), "Bearer gateway-token".into());
        let response = gateway.handle(&request);
        assert_eq!(response.status, 200);
        assert!(!String::from_utf8_lossy(&response.body).contains(token));
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            body["agents"][0]["descriptor"]["connection_access"]["authenticated"],
            true
        );
        worker.join().unwrap();
    }
    #[test]
    fn partial_credentials_and_endpoint_migration_fail_before_network_io() {
        let mut request = HttpRequest {
            method: "POST".into(),
            target: "/api/direct-mesh/agents".into(),
            path: "/api/direct-mesh/agents".into(),
            headers: Default::default(),
            body: vec![],
        };
        assert!(
            security_for(&request, &["localhost:5001".into()])
                .unwrap()
                .is_none()
        );
        request
            .headers
            .insert(HEADERS[0].into(), "never-echo-this-secret".into());
        let error = security_for(&request, &["localhost:5001".into()]).unwrap_err();
        assert!(
            !String::from_utf8(error.body)
                .unwrap()
                .contains("never-echo")
        );
        for (name, value) in HEADERS.into_iter().zip([
            "a".repeat(32),
            "localhost:5001".into(),
            "localhost".into(),
            "not a certificate".into(),
        ]) {
            request.headers.insert(name.into(), value);
        }
        assert!(security_for(&request, &["localhost:5002".into()]).is_err());
        assert!(
            security_for(
                &request,
                &["localhost:5001".into(), "localhost:5002".into()]
            )
            .is_err()
        );
    }
}
