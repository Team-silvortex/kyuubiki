//! Typed digests; request encoding is checked against shared SDK/Agent golden vectors.
use crate::{SdkResult, durable_execution::invalid};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn request_digest(method: &str, params: &Value) -> SdkResult<String> {
    if method.len() > 256
        || !params.is_object()
        || !(method.starts_with("solve_")
            || (method == "run_operator_task_ir" && params["mode"] == "execute"))
    {
        return Err(invalid(
            "durable task requires a solver or execute-mode TaskIR",
        ));
    }
    let mut hash = Sha256::new();
    hash.update(b"kyuubiki.agent-durable-execution/v1\0");
    bytes(&mut hash, b's', method.as_bytes());
    encode(&mut hash, params, 0)?;
    Ok(format!("{:x}", hash.finalize()))
}

// Compare v1 terminal meaning without cloning/serializing large result payloads.
// Optional absent/null fields normalize together; nested numeric bits do not.
pub(crate) fn terminal_digest(report: &Value) -> SdkResult<[u8; 32]> {
    let response = &report["response"];
    let mut hash = Sha256::new();
    hash.update(b"kyuubiki.sdk-terminal-observation/v1\0");
    for value in [
        &report["status"],
        &response["rpc_version"],
        &response["id"],
        &response["ok"],
        &response["result"],
        &response["error"]["code"],
        &response["error"]["message"],
        &response["error"]["details"],
    ] {
        encode(&mut hash, value, 0)?;
    }
    Ok(hash.finalize().into())
}

pub(crate) fn result_digest(result: &Value) -> SdkResult<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(b"kyuubiki.sdk-result-content/v1\0");
    encode(&mut hash, result, 0)?;
    Ok(hash.finalize().into())
}

fn header(hash: &mut Sha256, tag: u8, length: usize) {
    hash.update([tag]);
    hash.update((length as u64).to_be_bytes());
}

fn bytes(hash: &mut Sha256, tag: u8, value: &[u8]) {
    header(hash, tag, value.len());
    hash.update(value);
}

fn encode(hash: &mut Sha256, value: &Value, depth: usize) -> SdkResult<()> {
    if depth > 128 {
        return Err(invalid("durable value digest exceeds maximum depth 128"));
    }
    match value {
        Value::Null => hash.update(b"n"),
        Value::Bool(true) => hash.update(b"t"),
        Value::Bool(false) => hash.update(b"f"),
        Value::String(value) => bytes(hash, b's', value.as_bytes()),
        Value::Number(value) if value.is_i64() || value.is_u64() => {
            bytes(hash, b'i', value.to_string().as_bytes());
        }
        Value::Number(value) => {
            let finite = value
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| invalid("durable value digest requires finite binary64"))?;
            hash.update(b"d");
            hash.update(finite.to_bits().to_be_bytes());
        }
        Value::Array(values) => {
            header(hash, b'a', values.len());
            for value in values {
                encode(hash, value, depth + 1)?;
            }
        }
        Value::Object(values) => {
            header(hash, b'o', values.len());
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by_key(|(key, _)| *key);
            for (key, value) in entries {
                bytes(hash, b's', key.as_bytes());
                encode(hash, value, depth + 1)?;
            }
        }
    }
    Ok(())
}
