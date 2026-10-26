use kyuubiki_protocol::{RpcError, RpcResponse};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

// Persisted v1 terminal envelopes are closed even though the shared RPC types
// accept extensions. Decode directly into the payloads without a second Value tree.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredResponse {
    rpc_version: u8,
    id: String,
    ok: bool,
    result: Option<Value>,
    error: Option<StoredError>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredError {
    code: String,
    message: String,
    details: Option<Value>,
}

pub(super) fn decode<'de, D: Deserializer<'de>>(decoder: D) -> Result<RpcResponse, D::Error> {
    let stored = StoredResponse::deserialize(decoder)?;
    Ok(RpcResponse {
        rpc_version: stored.rpc_version,
        id: stored.id,
        ok: stored.ok,
        result: stored.result,
        error: stored.error.map(|error| RpcError {
            code: error.code,
            message: error.message,
            details: error.details,
        }),
    })
}
