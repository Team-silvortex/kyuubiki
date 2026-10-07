use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use kyuubiki_protocol::{RpcRequest, RpcResponse};
use serde::Deserialize;
use serde_json::{Value, json};

const SCHEMA: &str = "kyuubiki.agent-task-result-retention/v1";
const MAX_ENTRIES: usize = 64;
const MAX_ENTRY_BYTES: usize = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    attempt_id: String,
    request_id: String,
    task_id: String,
    task_digest: String,
    operator_id: String,
    program_id: String,
}

impl Identity {
    fn valid(&self) -> bool {
        hex(&self.attempt_id, 32)
            && hex(&self.task_digest, 64)
            && [&self.task_id, &self.operator_id, &self.program_id]
                .iter()
                .all(|s| !s.is_empty() && s.len() <= 1024)
            && !self.request_id.is_empty()
            && self.request_id.len() <= 256
    }

    fn from_request(request: &RpcRequest) -> Option<Self> {
        let params = &request.params;
        if params["mode"] != "execute" {
            return None;
        }
        let task = &params["task_ir"];
        let identity = Self {
            attempt_id: params["dispatch_attempt_id"].as_str()?.into(),
            request_id: request.id.clone(),
            task_id: task["task_id"].as_str()?.into(),
            task_digest: task["integrity"]["task_digest"].as_str()?.into(),
            operator_id: task["operator"]["id"].as_str()?.into(),
            program_id: task["execution_program"]["program_id"].as_str()?.into(),
        };
        identity.valid().then_some(identity)
    }
}

struct Entry {
    identity: Identity,
    generation: u64,
    created: Instant,
    status: &'static str,
    response: Option<Arc<[u8]>>,
}

#[derive(Default)]
struct Store {
    entries: VecDeque<Entry>,
    bytes: usize,
}

impl Store {
    fn prune(&mut self, now: Instant) {
        self.entries
            .retain(|entry| now.duration_since(entry.created) < TTL);
        self.bytes = self.entries.iter().map(entry_bytes).sum();
    }

    fn remove_first(&mut self) {
        if let Some(entry) = self.entries.pop_front() {
            self.bytes -= entry_bytes(&entry);
        }
    }

    fn begin(&mut self, identity: Identity, generation: u64, now: Instant) {
        self.prune(now);
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.identity.attempt_id == identity.attempt_id)
        {
            self.bytes -= entry_bytes(entry);
            entry.response = None;
            entry.status = "attempt_identity_ambiguous";
            return;
        }
        while self.entries.len() >= MAX_ENTRIES {
            self.remove_first();
        }
        self.entries.push_back(Entry {
            identity,
            generation,
            created: now,
            status: "pending",
            response: None,
        });
    }

    fn finish(
        &mut self,
        identity: &Identity,
        generation: u64,
        encoded: Option<Vec<u8>>,
        now: Instant,
    ) {
        self.prune(now);
        if !self
            .entries
            .iter()
            .any(|e| &e.identity == identity && e.generation == generation && e.status == "pending")
        {
            return;
        }
        let size = encoded.as_ref().map_or(0, Vec::len);
        // Never evict an active reservation to make room for a result; pressure
        // loses recovery availability, not the original solver execution.
        while self.bytes + size > MAX_TOTAL_BYTES {
            let Some(oldest) = self.entries.iter().position(|e| e.response.is_some()) else {
                break;
            };
            let entry = self.entries.remove(oldest).expect("index exists");
            self.bytes -= entry_bytes(&entry);
        }
        if let Some(entry) = self.entries.iter_mut().find(|e| {
            &e.identity == identity && e.generation == generation && e.status == "pending"
        }) {
            entry.status = if encoded.is_some() {
                "receipt_retained"
            } else {
                "result_not_retained"
            };
            entry.response = encoded.map(Arc::from);
            self.bytes += size;
        }
    }

    fn lookup(
        &mut self,
        identity: &Identity,
        now: Instant,
    ) -> (&'static str, Option<u64>, Option<Arc<[u8]>>) {
        self.prune(now);
        match self
            .entries
            .iter()
            .find(|entry| &entry.identity == identity)
        {
            Some(entry) => (entry.status, Some(entry.generation), entry.response.clone()),
            None => ("not_retained", None, None),
        }
    }
}

fn entry_bytes(entry: &Entry) -> usize {
    entry.response.as_ref().map_or(0, |bytes| bytes.len())
}

fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::default()))
}

pub(crate) struct Reservation {
    identity: Identity,
    generation: u64,
}

pub(crate) fn begin(request: &RpcRequest, generation: u64) -> Option<Reservation> {
    let identity = Identity::from_request(request)?;
    store()
        .lock()
        .ok()?
        .begin(identity.clone(), generation, Instant::now());
    Some(Reservation {
        identity,
        generation,
    })
}

pub(crate) fn finish(reservation: Option<Reservation>, response: &RpcResponse) {
    let Some(reservation) = reservation else {
        return;
    };
    let encoded = encode_bounded(response).ok();
    if let Ok(mut store) = store().lock() {
        store.finish(
            &reservation.identity,
            reservation.generation,
            encoded,
            Instant::now(),
        );
    }
}

pub(crate) fn policy() -> Value {
    json!({"schema_version":SCHEMA, "max_entries":MAX_ENTRIES, "max_entry_bytes":MAX_ENTRY_BYTES,
        "max_total_bytes":MAX_TOTAL_BYTES, "ttl_ms":TTL.as_millis(), "storage":"process_memory_only",
        "ttl_origin":"reservation_created", "expiry_reclamation":"on_cache_operation",
        "count_eviction":"oldest_reservation_including_pending", "byte_eviction":"oldest_retained_response",
        "oversize_policy":"do_not_cache_original_execution_unchanged",
        "completion_boundary":"computation_receipt_before_transport_delivery",
        "automatic_replay_authorized":false, "survives_agent_restart":false})
}

pub(crate) fn handle_fetch(request: RpcRequest) -> crate::transport::AgentReply {
    let identity = serde_json::from_value::<Identity>(request.params);
    let response = match identity {
        Ok(identity) if identity.valid() => match store().lock() {
            Ok(mut store) => {
                let (status, generation, encoded) = store.lookup(&identity, Instant::now());
                drop(store);
                let response =
                    encoded.and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
                RpcResponse::success(
                    request.id,
                    json!({"schema_version":SCHEMA,
                    "attempt_id":identity.attempt_id, "request_id":identity.request_id,
                    "task_id":identity.task_id, "task_digest":identity.task_digest,
                    "operator_id":identity.operator_id, "program_id":identity.program_id,
                    "process_instance_id":crate::agent_lifecycle::snapshot().process_instance_id,
                    "generation":generation, "status":status, "response":response,
                    "automatic_replay_authorized":false, "retention_policy":policy()}),
                )
            }
            Err(_) => RpcResponse::error(
                request.id,
                "task_result_retention_unavailable",
                "Agent result retention is unavailable",
            ),
        },
        _ => RpcResponse::error(
            request.id,
            "task_result_query_invalid",
            "require an exact attempt and task identity without extra fields",
        ),
    };
    crate::transport::AgentReply::Stream(Vec::new(), response)
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

struct BoundedWriter(Vec<u8>);
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_ENTRY_BYTES.saturating_sub(self.0.len()) {
            return Err(io::Error::other("retained response exceeds byte limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode_bounded(response: &RpcResponse) -> serde_json::Result<Vec<u8>> {
    let mut writer = BoundedWriter(Vec::new());
    serde_json::to_writer(&mut writer, response)?;
    Ok(writer.0)
}

#[cfg(test)]
#[path = "tests/agent_task_results.rs"]
mod tests;
