use crate::HeadlessExecutorError;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const REQUEST_IO_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const ARTIFACT_IO_TIMEOUT: Duration = Duration::from_secs(600);

pub(crate) fn connect_service_stream(
    host: &str,
    port: u16,
    io_timeout: Duration,
    context: &str,
) -> Result<TcpStream, HeadlessExecutorError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to resolve {host}:{port} for {context}: {error}"),
        })?
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(HeadlessExecutorError {
            message: format!("no network addresses resolved for {host}:{port}"),
        });
    }

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(io_timeout))
                    .and_then(|_| stream.set_write_timeout(Some(io_timeout)))
                    .map_err(|error| HeadlessExecutorError {
                        message: format!("failed to configure {context} timeout: {error}"),
                    })?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(HeadlessExecutorError {
        message: format!(
            "failed to connect to {host}:{port} for {context} within {} ms: {}",
            CONNECT_TIMEOUT.as_millis(),
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "unknown connection error".to_string())
        ),
    })
}

pub(crate) fn decode_http_response_body(
    head: &str,
    body: &str,
    context: &str,
) -> Result<String, HeadlessExecutorError> {
    let chunked = head.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && value
                    .split(',')
                    .any(|encoding| encoding.trim().eq_ignore_ascii_case("chunked"))
        })
    });
    if !chunked {
        return Ok(body.to_string());
    }

    let bytes = body.as_bytes();
    let mut cursor = 0;
    let mut decoded = Vec::new();
    loop {
        let line_end = bytes[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor + offset)
            .ok_or_else(|| response_error(context, "missing chunk-size delimiter"))?;
        let size_text = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|_| response_error(context, "chunk size is not UTF-8"))?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or_default(), 16)
            .map_err(|_| response_error(context, "invalid chunk size"))?;
        cursor = line_end + 2;
        if size == 0 {
            return String::from_utf8(decoded)
                .map_err(|_| response_error(context, "decoded body is not UTF-8"));
        }
        let chunk_end = cursor
            .checked_add(size)
            .filter(|end| end + 2 <= bytes.len())
            .ok_or_else(|| response_error(context, "truncated chunk body"))?;
        if &bytes[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(response_error(context, "missing chunk terminator"));
        }
        decoded.extend_from_slice(&bytes[cursor..chunk_end]);
        cursor = chunk_end + 2;
    }
}

fn response_error(context: &str, detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("invalid chunked HTTP response for {context}: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_http_response_body;

    #[test]
    fn decodes_chunked_error_bodies() {
        let body = decode_http_response_body(
            "HTTP/1.1 500 Internal Server Error\r\nTransfer-Encoding: chunked",
            "15\r\nInternal Server Error\r\n0\r\n\r\n",
            "/api/v1/model-artifacts",
        )
        .unwrap();

        assert_eq!(body, "Internal Server Error");
    }
}
