use crate::OperatorSdkError;
use kyuubiki_protocol::{OperatorRunRequest, OperatorRunResult};
use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;

pub const OPERATOR_JSON_ABI_SCHEMA_VERSION: &str = "kyuubiki.operator-json-c/v1";
pub const OPERATOR_JSON_ABI_FREE_SYMBOL: &str = "kyuubiki_operator_json_free";
pub const OPERATOR_JSON_ABI_OK: i32 = 0;
pub const OPERATOR_JSON_ABI_ERROR: i32 = 1;
pub const OPERATOR_JSON_ABI_INVALID_OUTPUT: i32 = 2;
pub const MAX_OPERATOR_JSON_ABI_BYTES: usize = 64 * 1024 * 1024;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OperatorJsonAbiBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl OperatorJsonAbiBuffer {
    pub const fn empty() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

pub type OperatorJsonEntrypoint =
    unsafe extern "C" fn(*const u8, usize, *mut OperatorJsonAbiBuffer) -> i32;
pub type OperatorJsonFreeEntrypoint = unsafe extern "C" fn(OperatorJsonAbiBuffer);

#[derive(Debug, Serialize, Deserialize)]
struct OperatorJsonAbiResponse {
    schema_version: String,
    ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<OperatorRunResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Executes one operator request without sharing Rust-owned values across the
/// dynamic-library boundary.
///
/// # Safety
///
/// `request_ptr` must reference `request_len` readable bytes when non-null,
/// and `output` must reference writable storage for one ABI buffer.
pub unsafe fn execute_operator_json_abi<F>(
    request_ptr: *const u8,
    request_len: usize,
    output: *mut OperatorJsonAbiBuffer,
    execute: F,
) -> i32
where
    F: FnOnce(OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError>,
{
    if output.is_null() {
        return OPERATOR_JSON_ABI_INVALID_OUTPUT;
    }
    unsafe { output.write(OperatorJsonAbiBuffer::empty()) };

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        if request_len > MAX_OPERATOR_JSON_ABI_BYTES {
            return Err(format!(
                "operator ABI request exceeds {MAX_OPERATOR_JSON_ABI_BYTES} bytes"
            ));
        }
        if request_len > 0 && request_ptr.is_null() {
            return Err("operator ABI request pointer is null".to_string());
        }
        let request_bytes = if request_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(request_ptr, request_len) }
        };
        let request = serde_json::from_slice::<OperatorRunRequest>(request_bytes)
            .map_err(|error| format!("invalid operator ABI request JSON: {error}"))?;
        execute(request).map_err(|error| error.to_string())
    }));

    let response = match outcome {
        Ok(Ok(result)) => OperatorJsonAbiResponse {
            schema_version: OPERATOR_JSON_ABI_SCHEMA_VERSION.to_string(),
            ok: true,
            result: Some(result),
            error: None,
        },
        Ok(Err(error)) => error_response(error),
        Err(_) => error_response("operator panicked inside JSON ABI boundary".to_string()),
    };
    write_response(output, response).unwrap_or(OPERATOR_JSON_ABI_ERROR)
}

/// Releases a buffer produced by `execute_operator_json_abi`.
///
/// # Safety
///
/// The buffer must be returned exactly once to the dynamic library that
/// allocated it.
pub unsafe fn free_operator_json_abi_buffer(buffer: OperatorJsonAbiBuffer) {
    if buffer.ptr.is_null() {
        return;
    }
    if buffer.capacity < buffer.len {
        return;
    }
    unsafe { drop(Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity)) };
}

pub fn decode_operator_json_abi_response(
    status: i32,
    bytes: &[u8],
) -> Result<OperatorRunResult, String> {
    let response = serde_json::from_slice::<OperatorJsonAbiResponse>(bytes)
        .map_err(|error| format!("invalid operator ABI response JSON: {error}"))?;
    if response.schema_version != OPERATOR_JSON_ABI_SCHEMA_VERSION {
        return Err(format!(
            "operator ABI response schema mismatch: {}",
            response.schema_version
        ));
    }
    if status == OPERATOR_JSON_ABI_OK && response.ok {
        return response
            .result
            .ok_or_else(|| "operator ABI success response omitted result".to_string());
    }
    Err(response
        .error
        .unwrap_or_else(|| format!("operator ABI failed with status {status}")))
}

fn error_response(error: String) -> OperatorJsonAbiResponse {
    OperatorJsonAbiResponse {
        schema_version: OPERATOR_JSON_ABI_SCHEMA_VERSION.to_string(),
        ok: false,
        result: None,
        error: Some(error),
    }
}

fn write_response(
    output: *mut OperatorJsonAbiBuffer,
    response: OperatorJsonAbiResponse,
) -> Result<i32, ()> {
    let mut bytes = serde_json::to_vec(&response).map_err(|_| ())?;
    if bytes.len() > MAX_OPERATOR_JSON_ABI_BYTES {
        return Err(());
    }
    let buffer = OperatorJsonAbiBuffer {
        ptr: bytes.as_mut_ptr(),
        len: bytes.len(),
        capacity: bytes.capacity(),
    };
    std::mem::forget(bytes);
    unsafe { output.write(buffer) };
    Ok(if response.ok {
        OPERATOR_JSON_ABI_OK
    } else {
        OPERATOR_JSON_ABI_ERROR
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_protocol::OperatorRunContext;

    #[test]
    fn round_trips_request_and_response_as_owned_json_bytes() {
        let request = OperatorRunRequest {
            operator_id: "extract.abi_fixture".to_string(),
            input: serde_json::json!({"payload": {"value": 42}, "config": {}}),
            context: OperatorRunContext::default(),
        };
        let request = serde_json::to_vec(&request).expect("request JSON");
        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe {
            execute_operator_json_abi(request.as_ptr(), request.len(), &mut output, |request| {
                Ok(OperatorRunResult {
                    operator_id: request.operator_id,
                    summary: request.input,
                    artifacts: Vec::new(),
                })
            })
        };
        let bytes = unsafe { slice::from_raw_parts(output.ptr, output.len) }.to_vec();
        unsafe { free_operator_json_abi_buffer(output) };
        let result = decode_operator_json_abi_response(status, &bytes).expect("ABI response");
        assert_eq!(result.summary["payload"]["value"], 42);
    }

    #[test]
    fn malformed_requests_return_bounded_error_envelopes() {
        let request = b"not-json";
        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe {
            execute_operator_json_abi(request.as_ptr(), request.len(), &mut output, |_| {
                panic!("malformed request must not reach handler")
            })
        };
        let bytes = unsafe { slice::from_raw_parts(output.ptr, output.len) }.to_vec();
        unsafe { free_operator_json_abi_buffer(output) };
        let error = decode_operator_json_abi_response(status, &bytes).expect_err("ABI error");
        assert!(error.contains("invalid operator ABI request JSON"));
    }

    #[test]
    fn rejects_invalid_pointers_without_calling_the_handler() {
        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe {
            execute_operator_json_abi(std::ptr::null(), 1, &mut output, |_| {
                panic!("invalid pointer must not reach handler")
            })
        };
        let bytes = unsafe { slice::from_raw_parts(output.ptr, output.len) }.to_vec();
        unsafe { free_operator_json_abi_buffer(output) };
        let error = decode_operator_json_abi_response(status, &bytes).expect_err("ABI error");
        assert!(error.contains("request pointer is null"));

        let status = unsafe {
            execute_operator_json_abi(std::ptr::null(), 0, std::ptr::null_mut(), |_| {
                panic!("null output must not reach handler")
            })
        };
        assert_eq!(status, OPERATOR_JSON_ABI_INVALID_OUTPUT);
    }

    #[test]
    fn rejects_oversized_requests_before_reading_memory() {
        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe {
            execute_operator_json_abi(
                std::ptr::dangling(),
                MAX_OPERATOR_JSON_ABI_BYTES + 1,
                &mut output,
                |_| panic!("oversized request must not reach handler"),
            )
        };
        let bytes = unsafe { slice::from_raw_parts(output.ptr, output.len) }.to_vec();
        unsafe { free_operator_json_abi_buffer(output) };
        let error = decode_operator_json_abi_response(status, &bytes).expect_err("ABI error");
        assert!(error.contains("request exceeds"));
    }

    #[test]
    fn contains_handler_panics_inside_the_library_boundary() {
        let request = serde_json::to_vec(&OperatorRunRequest {
            operator_id: "extract.panic_fixture".to_string(),
            input: serde_json::json!({}),
            context: OperatorRunContext::default(),
        })
        .expect("request JSON");
        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe {
            execute_operator_json_abi(request.as_ptr(), request.len(), &mut output, |_| {
                panic!("fixture panic")
            })
        };
        let bytes = unsafe { slice::from_raw_parts(output.ptr, output.len) }.to_vec();
        unsafe { free_operator_json_abi_buffer(output) };
        let error = decode_operator_json_abi_response(status, &bytes).expect_err("ABI error");
        assert!(error.contains("panicked inside JSON ABI boundary"));
    }
}
