use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::frontend_http::HttpRequest;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub(crate) struct ProxyError {
    pub(crate) message: String,
    pub(crate) response_started: bool,
}

impl ProxyError {
    fn before_response(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            response_started: false,
        }
    }

    fn after_response(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            response_started: true,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LoopbackUpstream {
    authority: String,
    addresses: Vec<SocketAddr>,
}

impl LoopbackUpstream {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let authority = value
            .trim()
            .strip_prefix("http://")
            .ok_or_else(|| "orchestrator URL must use loopback HTTP".to_string())?
            .trim_end_matches('/');
        if authority.is_empty()
            || authority.contains(['/', '?', '#', '@'])
            || authority.contains("..")
            || !explicit_loopback_authority(authority)
        {
            return Err("invalid orchestrator URL".to_string());
        }
        let addresses = authority
            .to_socket_addrs()
            .map_err(|error| format!("failed to resolve orchestrator URL: {error}"))?
            .collect::<Vec<_>>();
        if addresses.is_empty() || addresses.iter().any(|address| !address.ip().is_loopback()) {
            return Err("orchestrator URL must resolve only to loopback".to_string());
        }
        Ok(Self {
            authority: authority.to_string(),
            addresses,
        })
    }
}

fn explicit_loopback_authority(authority: &str) -> bool {
    let (host, port) = if let Some(tail) = authority.strip_prefix("[::1]:") {
        ("::1", tail)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        (host, port)
    } else {
        return false;
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
        && port.parse::<u16>().is_ok_and(|port| port > 0)
}

pub(crate) fn proxy_request(
    client: &mut TcpStream,
    request: &HttpRequest,
    upstream: &LoopbackUpstream,
) -> Result<(), ProxyError> {
    let mut server = connect(upstream).map_err(ProxyError::before_response)?;
    server.set_read_timeout(Some(IO_TIMEOUT)).map_err(|error| {
        ProxyError::before_response(format!(
            "failed to configure orchestrator read timeout: {error}"
        ))
    })?;
    server
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|error| {
            ProxyError::before_response(format!(
                "failed to configure orchestrator write timeout: {error}"
            ))
        })?;

    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        request.method,
        request.target,
        upstream.authority,
        request.body.len()
    );
    for name in [
        "accept",
        "accept-language",
        "authorization",
        "content-type",
        "if-none-match",
        "x-kyuubiki-token",
        "x-kyuubiki-cluster-token",
        "x-kyuubiki-client-cert-fingerprint",
        "x-kyuubiki-agent-id",
        "x-kyuubiki-agent-cert-fingerprint",
        "x-kyuubiki-cluster-nonce",
        "x-kyuubiki-cluster-timestamp",
        "x-kyuubiki-cluster-signature",
    ] {
        if let Some(value) = request.headers.get(name) {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(&value.replace(['\r', '\n'], ""));
            head.push_str("\r\n");
        }
    }
    head.push_str("X-Forwarded-For: 127.0.0.1\r\n\r\n");
    server
        .write_all(head.as_bytes())
        .and_then(|_| server.write_all(&request.body))
        .map_err(|error| {
            ProxyError::before_response(format!("failed to forward request to Orchestra: {error}"))
        })?;
    server
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| {
            ProxyError::before_response(format!("failed to finish Orchestra request: {error}"))
        })?;

    let mut buffer = [0_u8; 64 * 1024];
    let mut response_started = false;
    loop {
        let read = server.read(&mut buffer).map_err(|error| {
            let message = format!("failed to read Orchestra response: {error}");
            if response_started {
                ProxyError::after_response(message)
            } else {
                ProxyError::before_response(message)
            }
        })?;
        if read == 0 {
            if !response_started {
                return Err(ProxyError::before_response(
                    "Orchestra closed the connection without an HTTP response",
                ));
            }
            break;
        }
        response_started = true;
        client.write_all(&buffer[..read]).map_err(|error| {
            ProxyError::after_response(format!("failed to stream Orchestra response: {error}"))
        })?;
    }
    Ok(())
}

fn connect(upstream: &LoopbackUpstream) -> Result<TcpStream, String> {
    let mut last_error = None;
    for address in &upstream.addresses {
        match TcpStream::connect_timeout(address, CONNECT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    Err(format!(
        "failed to connect to Orchestra at {}: {}",
        upstream.authority,
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no loopback address".to_string())
    ))
}

#[cfg(test)]
mod tests {
    use super::LoopbackUpstream;

    #[test]
    fn proxy_accepts_only_loopback_http() {
        assert!(LoopbackUpstream::parse("http://127.0.0.1:4000").is_ok());
        assert!(LoopbackUpstream::parse("http://localhost:4000").is_ok());
        assert!(LoopbackUpstream::parse("https://127.0.0.1:4000").is_err());
        assert!(LoopbackUpstream::parse("http://8.8.8.8:4000").is_err());
        assert!(LoopbackUpstream::parse("http://example.test:4000").is_err());
    }

    #[test]
    fn ipv6_loopback_is_recognized_by_the_standard_library() {
        assert!(Ipv6Addr::LOCALHOST.is_loopback());
    }

    use std::net::Ipv6Addr;
}
