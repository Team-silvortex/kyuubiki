use crate::frontend_http::{HttpRequest, HttpResponse, json_response};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelRequest {
    base_url: String,
    #[serde(default)]
    api_key: String,
    operation: String,
    payload: Option<Value>,
}

fn failure(status: u16, code: &str) -> HttpResponse {
    json_response(status, json!({"error":code}))
}

fn loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn same_origin(request: &HttpRequest) -> bool {
    let (Some(host), Some(origin)) = (request.headers.get("host"), request.headers.get("origin"))
    else {
        return false;
    };
    let Ok(uri) = format!("http://{host}").parse::<ureq::http::Uri>() else {
        return false;
    };
    uri.host().is_some_and(loopback)
        && origin == &format!("http://{host}")
        && request
            .headers
            .get("content-type")
            .is_some_and(|value| value.split(';').next() == Some("application/json"))
}

fn endpoint(base: &str, operation: &str) -> Option<String> {
    if base.len() > 4096 || base.contains(['?', '#', '@']) || base.chars().any(char::is_whitespace)
    {
        return None;
    }
    let uri = base.parse::<ureq::http::Uri>().ok()?;
    let host = uri.host()?;
    if uri.scheme_str() != Some("https") && !(uri.scheme_str() == Some("http") && loopback(host)) {
        return None;
    }
    let suffix = match operation {
        "models" => "models",
        "plan" => "chat/completions",
        _ => return None,
    };
    Some(format!("{}/{suffix}", base.trim_end_matches('/')))
}

pub(crate) fn handle(request: &HttpRequest) -> HttpResponse {
    if request.path == "/api/assistant/capabilities" && request.method == "GET" {
        return json_response(
            200,
            json!({"schema_version":"kyuubiki.model-gateway/v1", "protocol":"chat_completions", "credential_storage":"none", "automatic_retry":false}),
        );
    }
    if request.path != "/api/assistant/request" || request.method != "POST" {
        return failure(405, "method_not_allowed");
    }
    // This is a same-origin desktop GUI bridge, not a public arbitrary-URL proxy.
    if !same_origin(request) {
        return failure(403, "model_gateway_origin_required");
    }
    if request.body.len() > 2 * MAX_BYTES {
        return failure(413, "model_request_too_large");
    }
    let Ok(input) = serde_json::from_slice::<ModelRequest>(&request.body) else {
        return failure(400, "invalid_model_configuration");
    };
    let Some(url) = endpoint(&input.base_url, &input.operation) else {
        return failure(400, "invalid_model_endpoint");
    };
    if input.api_key.len() > 8192 || !input.api_key.bytes().all(|b| b.is_ascii_graphic()) {
        return failure(400, "invalid_model_credentials");
    }
    if input.operation == "models" && input.payload.is_some() {
        return failure(400, "unexpected_model_payload");
    }
    if input.operation == "plan"
        && !input.payload.as_ref().is_some_and(|value| {
            value["model"]
                .as_str()
                .is_some_and(|model| !model.trim().is_empty() && model.len() <= 256)
                && value["messages"].is_array()
                && value.get("stream").is_none_or(|stream| stream == false)
        })
    {
        return failure(400, "invalid_model_payload");
    }
    if ACTIVE.fetch_add(1, Ordering::AcqRel) >= 4 {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
        return failure(429, "model_gateway_busy");
    }
    struct Permit;
    impl Drop for Permit {
        fn drop(&mut self) {
            ACTIVE.fetch_sub(1, Ordering::AcqRel);
        }
    }
    let _permit = Permit;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(if input.operation == "models" {
            15
        } else {
            60
        })))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into();
    let response = if input.operation == "models" {
        let mut call = agent.get(&url).header("Accept", "application/json");
        if !input.api_key.is_empty() {
            call = call.header("Authorization", &format!("Bearer {}", input.api_key));
        }
        call.call()
    } else {
        let mut call = agent.post(&url).header("Accept", "application/json");
        if !input.api_key.is_empty() {
            call = call.header("Authorization", &format!("Bearer {}", input.api_key));
        }
        call.send_json(input.payload.as_ref().expect("validated model payload"))
    };
    let Ok(mut response) = response else {
        return failure(502, "model_connection_failed");
    };
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return failure(
            if (400..600).contains(&status) {
                status
            } else {
                502
            },
            "model_provider_rejected_request",
        );
    }
    let Ok(bytes) = response
        .body_mut()
        .with_config()
        .limit((MAX_BYTES + 1) as u64)
        .read_to_vec()
    else {
        return failure(502, "invalid_model_response");
    };
    if bytes.len() > MAX_BYTES {
        return failure(502, "model_response_too_large");
    }
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return failure(502, "invalid_model_response");
    };
    json_response(200, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend_http::HttpRequest;
    use serde_json::json;
    use std::io::{Read, Write};

    fn request(base: &str) -> HttpRequest {
        HttpRequest {
            method: "POST".into(),
            path: "/api/assistant/request".into(),
            target: "/api/assistant/request".into(),
            headers: [
                ("host".into(), "127.0.0.1:3000".into()),
                ("origin".into(), "http://127.0.0.1:3000".into()),
                ("content-type".into(), "application/json".into()),
            ]
            .into(),
            body: serde_json::to_vec(
                &json!({"base_url":base,"api_key":"model-test-secret","operation":"models"}),
            )
            .unwrap(),
        }
    }

    #[test]
    fn model_gateway_rejects_cross_origin_and_unsafe_targets_before_contact() {
        let mut req = request("http://127.0.0.1:1/v1");
        req.headers
            .insert("origin".into(), "https://foreign.example".into());
        assert_eq!(handle(&req).status, 403);
        req.headers.remove("origin");
        assert_eq!(handle(&req).status, 403);
        for target in [
            "http://remote.example/v1",
            "https://user:secret@example.com/v1",
            "https://example.com/v1?key=secret",
            "https://example.com/v1#fragment",
        ] {
            let response = handle(&request(target));
            assert_eq!(response.status, 400, "{target}");
            assert!(!String::from_utf8(response.body).unwrap().contains("secret"));
        }
    }

    #[test]
    fn native_model_gateway_does_not_follow_redirects_or_echo_provider_errors() {
        for (status, body) in [
            (200, r#"{"data":[{"id":"local-model"}]}"#),
            (401, "model-test-secret"),
            (302, "model-test-secret"),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let worker = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = vec![];
                let mut byte = [0];
                while !bytes.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                }
                let head = String::from_utf8(bytes).unwrap();
                assert!(head.starts_with("GET /v1/models HTTP/1.1"));
                assert!(
                    head.to_lowercase()
                        .contains("authorization: bearer model-test-secret")
                );
                assert!(!head.contains("x-kyuubiki-agent-token"));
                write!(socket, "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nContent-Type: application/json\r\nLocation: http://127.0.0.1:1/never\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let response = handle(&request(&format!("http://{address}/v1")));
            assert_eq!(response.status, if status == 302 { 502 } else { status });
            assert!(
                !String::from_utf8(response.body)
                    .unwrap()
                    .contains("model-test-secret")
            );
            worker.join().unwrap();
        }
    }

    #[test]
    fn native_model_plan_forwards_only_explicit_model_credentials_and_payload() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let payload =
            json!({"model":"manual-model", "messages":[{"role":"user", "content":"test study"}]});
        let expected = payload.clone();
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let forwarded = crate::frontend_http::read_request(&mut socket).unwrap();
            assert_eq!(forwarded.method, "POST");
            assert_eq!(forwarded.path, "/v1/chat/completions");
            assert_eq!(
                forwarded.headers.get("authorization").unwrap(),
                "Bearer model-test-secret"
            );
            assert!(!forwarded.headers.contains_key("cookie"));
            assert!(!forwarded.headers.contains_key("x-kyuubiki-agent-token"));
            assert_eq!(
                serde_json::from_slice::<Value>(&forwarded.body).unwrap(),
                expected
            );
            let body = r#"{"choices":[{"message":{"content":"test response"}}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        let mut req = request(&format!("http://{address}/v1"));
        req.headers
            .insert("cookie".into(), "unrelated-cookie".into());
        req.headers.insert(
            "x-kyuubiki-agent-token".into(),
            "unrelated-agent-key".into(),
        );
        req.body = serde_json::to_vec(&json!({
            "base_url":format!("http://{address}/v1"), "api_key":"model-test-secret", "operation":"plan", "payload":payload
        })).unwrap();
        let response = handle(&req);
        assert_eq!(response.status, 200);
        assert_eq!(
            serde_json::from_slice::<Value>(&response.body).unwrap()["choices"][0]["message"]["content"],
            "test response"
        );
        worker.join().unwrap();
    }
}
