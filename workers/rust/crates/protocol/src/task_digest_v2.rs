use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const TASK_DIGEST_TYPED_V2: &str = "kyuubiki.task-digest/typed-ieee754-v2";
pub const TASK_DIGEST_FIXED15_V1: &str = "kyuubiki.task-digest/fixed15-v1";
const DOMAIN: &[u8] = b"kyuubiki.task-digest/typed-ieee754-v2\0";
const MAX_DEPTH: usize = 128;

pub(crate) fn agent_execution_digest(
    method: &crate::RpcMethod,
    params: &Value,
) -> Result<String, String> {
    let mut hash = Sha256::new();
    hash.update(b"kyuubiki.agent-durable-execution/v1\0");
    encode(
        &mut hash,
        &serde_json::to_value(method).map_err(|e| e.to_string())?,
        0,
    )?;
    encode(&mut hash, params, 0)?;
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) fn task_digest(object: &Map<String, Value>, fields: &[&str]) -> Result<String, String> {
    // Borrow the projection; do not duplicate large input artifacts or encoded JSON.
    let mut entries = fields
        .iter()
        .filter_map(|key| object.get(*key).map(|value| (*key, value)))
        .collect::<Vec<_>>();
    entries.sort_unstable_by_key(|(key, _)| *key);
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    header(&mut hash, b'o', entries.len());
    for (key, value) in entries {
        bytes(&mut hash, b's', key.as_bytes());
        encode(&mut hash, value, 1)?;
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn header(hash: &mut Sha256, tag: u8, length: usize) {
    hash.update([tag]);
    hash.update((length as u64).to_be_bytes());
}

fn bytes(hash: &mut Sha256, tag: u8, value: &[u8]) {
    header(hash, tag, value.len());
    hash.update(value);
}

fn encode(hash: &mut Sha256, value: &Value, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("typed TaskIR digest exceeds maximum depth 128".to_string());
    }
    match value {
        Value::Null => hash.update(b"n"),
        Value::Bool(true) => hash.update(b"t"),
        Value::Bool(false) => hash.update(b"f"),
        Value::String(value) => bytes(hash, b's', value.as_bytes()),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            bytes(hash, b'i', number.to_string().as_bytes());
        }
        Value::Number(number) => {
            let value = number
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    "typed TaskIR digest requires finite binary64 numbers".to_string()
                })?;
            hash.update(b"d");
            hash.update(value.to_bits().to_be_bytes());
        }
        Value::Array(values) => {
            header(hash, b'a', values.len());
            for value in values {
                encode(hash, value, depth + 1)?;
            }
        }
        Value::Object(values) => {
            header(hash, b'o', values.len());
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_unstable_by_key(|(key, _)| *key);
            for (key, value) in entries {
                bytes(hash, b's', key.as_bytes());
                encode(hash, value, depth + 1)?;
            }
        }
    }
    Ok(())
}
