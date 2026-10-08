use crate::service_executor_response::response_body_length;
use crate::service_executor_result_artifact::RESULT_MEDIA_TYPE;
use std::io::{self, BufRead, Read};

const HEADER_LIMIT: usize = 64 * 1024;

pub(crate) fn receive_content(
    reader: &mut impl BufRead,
    expected_size: u64,
    mut accept: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()> {
    let mut head = Vec::new();
    loop {
        let line = bounded_line(reader, HEADER_LIMIT - head.len())?;
        head.extend_from_slice(&line);
        if line == b"\r\n" {
            break;
        }
    }
    let head = std::str::from_utf8(&head).map_err(|_| invalid("response header is not UTF-8"))?;
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap_or_default();
    let mut status_parts = status.splitn(3, ' ');
    if !matches!(status_parts.next(), Some("HTTP/1.0" | "HTTP/1.1"))
        || status_parts.next() != Some("200")
    {
        return Err(invalid(
            "result endpoint did not return HTTP 200; redirects are not followed",
        ));
    }
    let mut media = None;
    let mut transfer = None;
    let mut encoding = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("malformed response header"))?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(invalid("malformed response header name"));
        }
        let value = value.trim();
        let slot = if name.eq_ignore_ascii_case("content-type") {
            Some(&mut media)
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            Some(&mut transfer)
        } else if name.eq_ignore_ascii_case("content-encoding") {
            Some(&mut encoding)
        } else {
            None
        };
        if let Some(slot) = slot
            && slot.replace(value).is_some()
        {
            return Err(invalid("duplicate result representation header"));
        }
    }
    if !media.is_some_and(|value| {
        value
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .eq_ignore_ascii_case(RESULT_MEDIA_TYPE)
    }) {
        return Err(invalid("unexpected result content type"));
    }
    if encoding.is_some_and(|value| !value.eq_ignore_ascii_case("identity")) {
        return Err(invalid("encoded result bodies are not supported"));
    }
    let length = response_body_length(head, "result artifact content")
        .map_err(|_| invalid("ambiguous or invalid result body length"))?;
    let chunked = match (length, transfer) {
        (Some(length), None) if length as u64 == expected_size => false,
        (None, Some(value)) if value.eq_ignore_ascii_case("chunked") => true,
        _ => {
            return Err(invalid(
                "result response must have an exact content length or chunked framing",
            ));
        }
    };
    let mut received = 0;
    if chunked {
        loop {
            let line = bounded_line(reader, 1024)?;
            let text = std::str::from_utf8(&line[..line.len() - 2])
                .map_err(|_| invalid("invalid chunk size"))?;
            let size_text = text.split(';').next().unwrap_or_default();
            if size_text.is_empty() || !size_text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(invalid("invalid chunk size"));
            }
            let size =
                u64::from_str_radix(size_text, 16).map_err(|_| invalid("chunk size overflow"))?;
            if size == 0 {
                if bounded_line(reader, HEADER_LIMIT)? != b"\r\n" {
                    return Err(invalid("result chunk trailers are not supported"));
                }
                break;
            }
            if size > expected_size.saturating_sub(received) {
                return Err(invalid("chunked result exceeds descriptor size"));
            }
            copy_exact(reader, size, &mut accept)?;
            received += size;
            let mut end = [0; 2];
            reader.read_exact(&mut end)?;
            if end != *b"\r\n" {
                return Err(invalid("invalid chunk terminator"));
            }
        }
    } else {
        copy_exact(reader, expected_size, &mut accept)?;
        received = expected_size;
    }
    if received != expected_size {
        return Err(invalid("result content is shorter than its descriptor"));
    }
    let mut extra = [0; 1];
    if reader.read(&mut extra)? != 0 {
        return Err(invalid("unexpected bytes after result content"));
    }
    Ok(())
}

fn copy_exact(
    reader: &mut impl Read,
    mut remaining: u64,
    accept: &mut impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()> {
    let mut buffer = [0; 64 * 1024];
    while remaining > 0 {
        let length = remaining.min(buffer.len() as u64) as usize;
        reader.read_exact(&mut buffer[..length])?;
        accept(&buffer[..length])?;
        remaining -= length as u64;
    }
    Ok(())
}

fn bounded_line(reader: &mut impl BufRead, maximum: usize) -> io::Result<Vec<u8>> {
    let mut line = Vec::new();
    reader.take(maximum as u64).read_until(b'\n', &mut line)?;
    if !line.ends_with(b"\r\n") {
        return Err(invalid("unterminated or oversized response line"));
    }
    Ok(line)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
