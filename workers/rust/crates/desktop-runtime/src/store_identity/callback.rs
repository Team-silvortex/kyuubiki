use super::{CALLBACK_PATH, IdentityConfig, IdentityResult};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

fn equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

fn parse(
    target: &str,
    config: &IdentityConfig,
    state: &str,
) -> IdentityResult<Option<Zeroizing<String>>> {
    let (path, query) = target.split_once('?').ok_or("invalid_callback")?;
    if path != CALLBACK_PATH || target.len() > 8192 || query.contains('#') {
        return Err("invalid_callback");
    }
    // Reject malformed percent escapes before form decoding (which otherwise repairs them).
    let bytes = query.as_bytes();
    for (index, value) in bytes.iter().enumerate() {
        if *value == b'%'
            && (index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit())
        {
            return Err("invalid_callback");
        }
    }
    let mut fields = HashMap::new();
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        if key.contains('\u{fffd}')
            || value.contains('\u{fffd}')
            || fields.insert(key, value).is_some()
        {
            return Err("invalid_callback");
        }
    }
    let actual_state = fields.get("state").ok_or("invalid_callback")?;
    if !equal(actual_state.as_bytes(), state.as_bytes()) {
        return Ok(None);
    }
    if fields.get("iss").map(|v| v.as_ref()) != Some(config.issuer.as_str()) {
        return Err("issuer_mismatch");
    }
    match (fields.get("code"), fields.get("error")) {
        (Some(code), None)
            if !code.is_empty()
                && code.len() <= 4096
                && code.bytes().all(|b| b.is_ascii_graphic()) =>
        {
            Ok(Some(Zeroizing::new(code.to_string())))
        }
        (None, Some(_)) => Err("authorization_denied"),
        _ => Err("invalid_callback"),
    }
}

fn request(stream: &mut TcpStream, port: u16) -> IdentityResult<Zeroizing<String>> {
    // Accepted sockets can inherit O_NONBLOCK on macOS, unlike Linux.
    stream
        .set_nonblocking(false)
        .map_err(|_| "invalid_callback")?;
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|_| "invalid_callback")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|_| "invalid_callback")?;
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut data = Zeroizing::new(Vec::new());
    let mut buffer = [0; 1024];
    while data.len() <= 12288 && Instant::now() < deadline {
        let size = stream.read(&mut buffer).map_err(|_| "invalid_callback")?;
        if size == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..size]);
        if data.windows(4).any(|part| part == b"\r\n\r\n") {
            break;
        }
    }
    if data.len() > 12288 || !data.ends_with(b"\r\n\r\n") {
        return Err("invalid_callback");
    }
    let text = std::str::from_utf8(&data).map_err(|_| "invalid_callback")?;
    let mut lines = text.split("\r\n");
    let request: Vec<_> = lines.next().ok_or("invalid_callback")?.split(' ').collect();
    if request.len() != 3 || request[0] != "GET" || request[2] != "HTTP/1.1" {
        return Err("invalid_callback");
    }
    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, _)| key.eq_ignore_ascii_case("host"))
        .collect();
    if hosts.len() != 1 || hosts[0].1.trim() != format!("127.0.0.1:{port}") {
        return Err("invalid_callback");
    }
    Ok(Zeroizing::new(request[1].to_string()))
}

pub(super) fn wait(
    listener: TcpListener,
    config: &IdentityConfig,
    state: &str,
    active: impl Fn() -> bool,
    deadline: Instant,
) -> IdentityResult<Zeroizing<String>> {
    while active() && Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let result = request(&mut stream, config.callback_port)
                    .and_then(|target| parse(&target, config, state));
                let accepted = matches!(result, Ok(Some(_)));
                let status = if accepted {
                    "200 OK"
                } else {
                    "400 Bad Request"
                };
                let body = if accepted {
                    "Login response received. Return to Kyuubiki to see the verified result."
                } else {
                    "Login response was not accepted. Return to Kyuubiki."
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                match result {
                    Ok(Some(code)) => return Ok(code),
                    Err(error @ ("issuer_mismatch" | "authorization_denied")) => return Err(error),
                    _ => {} // Unsolicited local traffic cannot consume the pending login.
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(40))
            }
            Err(_) => return Err("callback_unavailable"),
        }
    }
    Err(if active() {
        "login_timeout"
    } else {
        "cancelled"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_rejects_duplicates_mixed_results_wrong_issuer_and_foreign_state() {
        let config: IdentityConfig = serde_json::from_value(
            serde_json::json!({"issuer":"https://id.test","client_id":"client"}),
        )
        .unwrap();
        let valid =
            format!("{CALLBACK_PATH}?state=expected&iss=https%3A%2F%2Fid.test&code=one-time");
        assert_eq!(
            &**parse(&valid, &config, "expected").unwrap().unwrap(),
            "one-time"
        );
        for extra in [
            "&state=expected",
            "&%73tate=expected",
            "&code=second",
            "&error=denied",
            "&x=%ZZ",
        ] {
            assert!(parse(&(valid.clone() + extra), &config, "expected").is_err());
        }
        assert!(parse(&valid, &config, "foreign").unwrap().is_none());
        assert_eq!(
            parse(&valid.replace("id.test", "evil.test"), &config, "expected").unwrap_err(),
            "issuer_mismatch"
        );
    }
}
