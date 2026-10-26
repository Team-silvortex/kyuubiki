use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use kyuubiki_protocol::{
    DURABLE_AGENT_EXECUTION_SCHEMA, DurableAgentTask, RpcResponse, validate_durable_execution_id,
    validate_rpc_response_envelope,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[path = "agent_execution_store_identity.rs"]
mod identity;
#[path = "agent_execution_store_inventory.rs"]
mod inventory;
#[path = "agent_execution_store_io.rs"]
mod io;
#[path = "agent_execution_store_response.rs"]
mod response;

pub(crate) const STORE_ENV: &str = "KYUUBIKI_AGENT_EXECUTION_STORE";
const MAX_ENTRIES: usize = 1024;
const MAX_RECEIPT_BYTES: usize = 8 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 4096;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    schema_version: String,
    pub execution_id: String,
    request_digest: String,
    pub request_id: String,
    pub process_instance_id: String,
    method: String,
}

impl Record {
    fn validate(&self) -> Result<(), String> {
        validate_durable_execution_id(&self.execution_id)?;
        if self.schema_version != DURABLE_AGENT_EXECUTION_SCHEMA {
            return Err("execution admission identity or schema mismatch".into());
        }
        let bounded_identity = |value: &str| {
            !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
        };
        if self.request_digest.len() != 64
            || !self
                .request_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !bounded_identity(&self.request_id)
            || !bounded_identity(&self.process_instance_id)
            || !bounded_identity(&self.method)
            || !(self
                .method
                .strip_prefix("solve_")
                .is_some_and(|name| !name.is_empty())
                || self.method == "run_operator_task_ir")
        {
            return Err("invalid execution admission identity".into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    record: Record,
    #[serde(deserialize_with = "response::decode")]
    response: RpcResponse,
}

impl Receipt {
    fn prepare(&self) -> Result<io::PreparedRecord, &'static str> {
        validate_response(&self.response).map_err(|_| "invalid_response")?;
        io::prepare(self, MAX_RECEIPT_BYTES).map_err(|error| error.reason())
    }
}

fn validate_response(response: &RpcResponse) -> Result<(), String> {
    validate_rpc_response_envelope(response)
        .map_err(|error| format!("invalid durable response: {}", error.message))?;
    // JSON null decodes to None for Option<Value>; it cannot be a durable solver result.
    if response.ok && response.result.as_ref().is_some_and(Value::is_null) {
        return Err(
            "invalid durable response: successful execution requires a non-null result".into(),
        );
    }
    Ok(())
}

pub(crate) enum Admission {
    New(Record),
    Existing(Value),
}

pub(crate) enum AdmissionFailure {
    Rejected(Value),
    Uncertain(String),
}

impl From<String> for AdmissionFailure {
    fn from(message: String) -> Self {
        Self::Uncertain(message)
    }
}

impl From<&str> for AdmissionFailure {
    fn from(message: &str) -> Self {
        Self::Uncertain(message.into())
    }
}

pub(crate) struct Store {
    root: PathBuf,
    store_id: String,
    lock: File,
    active: HashSet<String>,
    known_admissions: HashSet<String>,
    inventory: inventory::Inventory,
    bytes: u64,
    entries: usize,
    write_failed: bool,
    uncertain_receipts: HashSet<String>,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self, String> {
        io::prepare_root(root)?;
        let mut lock = identity::open_lock(root)?;
        lock.try_lock_exclusive()
            .map_err(|e| format!("execution store is already owned: {e}"))?;
        let store_id = identity::load_or_create(&mut lock, root)?;
        let mut store = Self {
            root: root.to_path_buf(),
            store_id,
            lock,
            active: HashSet::new(),
            known_admissions: HashSet::new(),
            inventory: inventory::Inventory::default(),
            bytes: 0,
            entries: 0,
            write_failed: false,
            uncertain_receipts: HashSet::new(),
        };
        store.scan()?;
        Ok(store)
    }

    fn scan(&mut self) -> Result<(), String> {
        let mut count = 0;
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            count += 1;
            if count > 4 * MAX_ENTRIES + 1 {
                return Err("execution store file count exceeds limit".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "invalid execution store filename")?;
            if name == "owner.lock" {
                continue;
            }
            let (id, suffix) = name
                .split_once('.')
                .ok_or("foreign file in execution store")?;
            validate_durable_execution_id(id)?;
            if !matches!(
                suffix,
                "admission" | "receipt" | "admission.next" | "receipt.next"
            ) {
                return Err("foreign file in execution store".into());
            }
            let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("non-regular entry in execution store".into());
            }
            if suffix.ends_with(".next") {
                // A pending file never authorizes execution or result publication.
                fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
                continue;
            }
            self.bytes = self
                .bytes
                .checked_add(metadata.len())
                .ok_or("execution store size overflow")?;
            if suffix == "admission" {
                self.entries += 1;
                self.known_admissions.insert(id.to_string());
            }
            self.inventory.remember(name, metadata.len());
        }
        if self.bytes > MAX_TOTAL_BYTES || self.entries > MAX_ENTRIES {
            return Err("execution store exceeds configured hard limits".into());
        }
        io::sync_directory(&self.root)
    }

    pub fn admit(
        &mut self,
        task: &DurableAgentTask,
        process: &str,
        request_id: &str,
    ) -> Result<Admission, String> {
        self.admit_with(task, process, request_id, || Ok(()))
            .map(|(admission, _)| admission)
            .map_err(|error| match error {
                AdmissionFailure::Rejected(report) => report["message"]
                    .as_str()
                    .unwrap_or("admission rejected")
                    .into(),
                AdmissionFailure::Uncertain(message) => message,
            })
    }

    pub fn admit_with<T>(
        &mut self,
        task: &DurableAgentTask,
        process: &str,
        request_id: &str,
        reserve: impl FnOnce() -> Result<T, String>,
    ) -> Result<(Admission, Option<T>), AdmissionFailure> {
        let digest = task.request_digest()?;
        let method = serde_json::to_value(&task.method)
            .map_err(|e| e.to_string())?
            .as_str()
            .ok_or("invalid method")?
            .to_owned();
        self.check_owner(task.expected_store_id.as_deref())?;
        if let Some(record) = self.record(&task.execution_id)? {
            if record.request_digest != digest || record.method != method {
                return Err("execution_id is already bound to different inputs or method".into());
            }
            return self
                .lookup_record(&record)
                .map(|report| (Admission::Existing(report), None))
                .map_err(Into::into);
        }
        if self.write_failed {
            return Err("execution store admission blocked by a previous write failure".into());
        }
        let record = Record {
            schema_version: DURABLE_AGENT_EXECUTION_SCHEMA.into(),
            execution_id: task.execution_id.clone(),
            request_digest: digest,
            request_id: request_id.into(),
            process_instance_id: process.into(),
            method,
        };
        record.validate()?;
        let rejected = |reason: &str, message: String| {
            let mut report =
                serde_json::to_value(&record).expect("serializable admission identity");
            report["schema_version"] = json!(kyuubiki_protocol::DURABLE_AGENT_REJECTION_SCHEMA);
            report["status"] = json!("not_admitted");
            report["reason"] = json!(reason);
            report["message"] = json!(message);
            report["execution_started"] = json!(false);
            report["automatic_replay_authorized"] = json!(false);
            report["store_id"] = json!(self.store_id);
            AdmissionFailure::Rejected(report)
        };
        let reserved_bytes = (self.active.len() as u64 + 1) * (MAX_RECEIPT_BYTES as u64 + 65);
        if self.entries >= MAX_ENTRIES
            || self.bytes + reserved_bytes + MAX_RECORD_BYTES as u64 + 65 > MAX_TOTAL_BYTES
        {
            return Err(rejected(
                "store_capacity",
                "execution store admission blocked by capacity".into(),
            ));
        }
        // Cached capacity is safe only while finalized disk membership and sizes
        // still match. Do not scan payloads or reconcile external repairs live.
        self.inventory.check(&self.root)?;
        // Reserve before persisting admission, but never reserve on a duplicate lookup.
        let reservation = match reserve() {
            Ok(reservation) => reservation,
            Err(message) => {
                // Retain this identity as terminal without obtaining a slot or calling the engine.
                // If either commit fails, the caller must inspect; absence is never a safe retry.
                self.persist_admission(&record)?;
                let response = RpcResponse::error_with_details(
                    record.request_id.clone(),
                    "durable_admission_rejected",
                    message,
                    json!({"reason":"execution_unavailable", "execution_started":false,
                        "automatic_replay_authorized":false}),
                );
                let report = self.finish(&record, response);
                self.abandon(&record.execution_id);
                if report.is_err() {
                    self.write_failed = true;
                }
                return Ok((Admission::Existing(report?), None));
            }
        };
        self.persist_admission(&record)?;
        Ok((Admission::New(record), Some(reservation)))
    }

    fn persist_admission(&mut self, record: &Record) -> Result<(), String> {
        self.check_owner(None)?;
        match io::write(
            &self.root,
            &format!("{}.admission", record.execution_id),
            record,
            MAX_RECORD_BYTES,
        ) {
            Ok(bytes) => {
                self.bytes += bytes;
                self.inventory
                    .remember(format!("{}.admission", record.execution_id), bytes);
            }
            Err(error) => {
                self.write_failed = true;
                return Err(error);
            }
        }
        self.entries += 1;
        self.known_admissions.insert(record.execution_id.clone());
        self.active.insert(record.execution_id.clone());
        Ok(())
    }

    pub fn finish(&mut self, record: &Record, response: RpcResponse) -> Result<Value, String> {
        self.check_owner(None)?;
        if !self.active.contains(&record.execution_id)
            || self.record(&record.execution_id)?.as_ref() != Some(record)
            || response.id != record.request_id
        {
            return Err("execution receipt does not match its active admission".into());
        }
        let mut receipt = Receipt {
            record: record.clone(),
            response,
        };
        let outcome = (|| {
            let prepared = match receipt.prepare() {
                Ok(prepared) => prepared,
                Err(reason) => {
                    // Preparation has no filesystem side effects. Isolate an invalid response
                    // without mistaking it for a storage failure or retaining its large payload.
                    receipt.response = RpcResponse::error_with_details(
                        record.request_id.clone(),
                        "durable_response_unrecordable",
                        "execution response cannot be retained within durable receipt policy; response discarded",
                        json!({"phase":"receipt_validation", "reason":reason,
                            "max_receipt_bytes":MAX_RECEIPT_BYTES, "execution_may_have_started":true,
                            "original_response_discarded":true, "external_effects_rolled_back":false,
                            "automatic_replay_authorized":false}),
                    );
                    receipt.prepare().map_err(str::to_string)?
                }
            };
            self.check_owner(None)?;
            io::commit(
                &self.root,
                &format!("{}.receipt", record.execution_id),
                prepared,
            )
        })();
        self.active.remove(&record.execution_id);
        match outcome {
            Ok(bytes) => {
                self.bytes += bytes;
                self.inventory
                    .remember(format!("{}.receipt", record.execution_id), bytes);
            }
            Err(error) => {
                self.write_failed = true;
                self.uncertain_receipts.insert(record.execution_id.clone());
                return Err(error);
            }
        }
        Ok(self.report(record, Some(receipt.response)))
    }

    pub fn abandon(&mut self, id: &str) {
        self.active.remove(id);
    }

    pub fn lookup(&self, id: &str) -> Result<Value, String> {
        validate_durable_execution_id(id)?;
        self.check_owner(None)?;
        match self.record(id)? {
            Some(record) => self.lookup_record(&record),
            None => Ok(
                json!({"schema_version":DURABLE_AGENT_EXECUTION_SCHEMA, "execution_id":id, "store_id":self.store_id,
                "status":"not_found", "automatic_replay_authorized":false}),
            ),
        }
    }

    pub fn list(
        &self,
        query: &kyuubiki_protocol::DurableAgentExecutionList,
    ) -> Result<Value, String> {
        query.validate()?;
        self.check_owner(query.expected_store_id.as_deref())?;
        let mut present = HashSet::new();
        let mut receipts = HashSet::new();
        for (count, entry) in fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .enumerate()
        {
            if count > 4 * MAX_ENTRIES {
                return Err("execution store file count exceeds limit".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name = name.to_str().ok_or("invalid execution store filename")?;
            if let Some(id) = name.strip_suffix(".admission") {
                validate_durable_execution_id(id)?;
                present.insert(id.to_string());
            } else if let Some(id) = name.strip_suffix(".receipt") {
                validate_durable_execution_id(id)?;
                receipts.insert(id.to_string());
            }
        }
        // Check before pagination: a missing admission must not vanish behind the cursor.
        // Receipt names suffice; indexing never reads large result payloads.
        if !self.known_admissions.is_subset(&present) || !receipts.is_subset(&present) {
            return Err("execution recovery index is incomplete: admission missing; restore original records; no replay authorized".into());
        }
        let mut ids: Vec<_> = present
            .into_iter()
            .filter(|id| query.after.as_ref().is_none_or(|after| id > after))
            .collect();
        ids.sort_unstable();
        let more = ids.len() > query.limit;
        ids.truncate(query.limit);
        // Index admissions only. Large result receipts are read on explicit inspection.
        let records = ids
            .iter()
            .map(|id| {
                self.record(id)?
                    .ok_or_else(|| "execution admission disappeared".into())
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(
            json!({"schema_version":kyuubiki_protocol::DURABLE_AGENT_INDEX_SCHEMA,
            "executions":records, "next_cursor":if more { ids.last() } else { None },
            "automatic_replay_authorized":false, "store_id":self.store_id}),
        )
    }

    fn record(&self, id: &str) -> Result<Option<Record>, String> {
        validate_durable_execution_id(id)?;
        let record: Option<Record> =
            io::read(&self.root.join(format!("{id}.admission")), MAX_RECORD_BYTES)?;
        if let Some(record) = &record {
            record.validate()?;
            if record.execution_id != id {
                return Err("execution admission identity or schema mismatch".into());
            }
        } else {
            if self.known_admissions.contains(id) {
                return Err("known execution admission is missing; restore original records; no replay authorized".into());
            }
            match fs::symlink_metadata(self.root.join(format!("{id}.receipt"))) {
                Ok(_) => {
                    return Err(
                        "orphan receipt has no admission; refusing absence or execution_id reuse"
                            .into(),
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(record)
    }

    fn lookup_record(&self, record: &Record) -> Result<Value, String> {
        if self.uncertain_receipts.contains(&record.execution_id) {
            // Publication may have succeeded even when cleanup or directory sync failed.
            // Do not turn that unacknowledged commit into a successful durable read.
            return Ok(self.report(record, None));
        }
        let receipt: Option<Receipt> = io::read(
            &self.root.join(format!("{}.receipt", record.execution_id)),
            MAX_RECEIPT_BYTES,
        )?;
        if let Some(receipt) = receipt {
            if receipt.record != *record || receipt.response.id != record.request_id {
                return Err("execution receipt identity mismatch".into());
            }
            validate_response(&receipt.response)?;
            Ok(self.report(record, Some(receipt.response)))
        } else {
            Ok(self.report(record, None))
        }
    }

    fn report(&self, record: &Record, response: Option<RpcResponse>) -> Value {
        let status = match &response {
            Some(response) if response.ok => "completed",
            Some(_) => "failed",
            None if self.active.contains(&record.execution_id) => "running",
            None => "outcome_unknown",
        };
        json!({"schema_version": DURABLE_AGENT_EXECUTION_SCHEMA,
            "store_id":self.store_id,
            "execution_id":record.execution_id, "request_digest":record.request_digest,
            "request_id":record.request_id, "process_instance_id":record.process_instance_id,
            "method":record.method, "status":status, "response":response,
            "automatic_replay_authorized":false, "resumable":false})
    }

    pub fn policy(&self) -> Value {
        let mut policy = policy(true);
        let bound = self.check_owner(None).is_ok();
        policy["available"] = json!(bound);
        if !bound {
            policy["unavailable_reason"] = json!("store_binding_unavailable");
        }
        policy["store_id"] = json!(self.store_id);
        policy["directory"] = json!(self.root);
        policy["stored_entries"] = json!(self.entries);
        policy["stored_bytes"] = json!(self.bytes);
        policy["active_reservations"] = json!(self.active.len());
        policy["admission_blocked_after_write_failure"] = json!(self.write_failed);
        policy["admission_blocked_after_inventory_change"] = json!(self.inventory.blocked);
        policy
    }

    pub fn check_owner(&self, expected: Option<&str>) -> Result<(), String> {
        identity::check_binding(&self.lock, &self.root, &self.store_id)?;
        if let Some(expected) = expected {
            kyuubiki_protocol::validate_durable_execution_id(expected)?;
            if expected != self.store_id {
                return Err("durable execution store identity mismatch; restore the original Agent store; no replay authorized".into());
            }
        }
        Ok(())
    }
}

pub(crate) fn policy(enabled: bool) -> Value {
    json!({"schema_version":DURABLE_AGENT_EXECUTION_SCHEMA, "enabled":enabled,
        "configuration_env":STORE_ENV, "storage":"immutable_checksummed_files",
        "max_entries":MAX_ENTRIES, "max_receipt_bytes":MAX_RECEIPT_BYTES, "max_total_bytes":MAX_TOTAL_BYTES,
        "automatic_eviction":false, "capacity_policy":"reject_new_admission",
        "inventory_fence":{"check":"before_new_admission_reservation",
            "scope":"finalized_record_names_regular_file_types_and_encoded_lengths",
            "max_directory_entries":4 * MAX_ENTRIES + 1, "reads_receipt_payloads":false,
            "pending_files_in_byte_accounting":false, "content_integrity_verified":false,
            "failure_policy":"block_new_admissions_until_explicit_restart_after_inspection",
            "existing_reads_and_active_completion_preserved":true,
            "descriptor_is_live_scan":false, "automatic_reconciliation":false,
            "concurrent_filesystem_mutation_safe":false},
        "survives_agent_restart":enabled, "automatic_replay_authorized":false,
        "owner_fence":{"request_field":"expected_store_id", "response_field":"store_id",
            "identity_file":"owner.lock", "survives_restart":true, "is_authentication":false,
            "legacy_unfenced_requests":true, "mismatch_policy":"reject_before_operation"},
        "identity_initialization":{"allowed":"directory_empty_except_owner_lock",
            "missing_or_empty_with_retained_entries":"startup_error_restore_original_owner",
            "pending_entries_block_initialization":true, "automatic_legacy_adoption":false,
            "failed_bootstrap_cleans_pending":false, "complete_store_loss_detected":false},
        "live_store_binding":{"check":"before_store_operations_and_receipt_commit",
            "owner_file_binding_checked":cfg!(unix),
            "identity_read_max_bytes":if cfg!(unix) { 161 } else { 0 }, "identity_reinitialized":false,
            "locked_file_identity":if cfg!(unix) { "device_and_inode" } else { "not_verified" },
            "failure_policy":"unavailable_no_redirect_or_replay",
            "concurrent_filesystem_mutation_safe":false},
        "missing_admission":{"known_ids":"startup_scan_and_successful_live_admissions",
            "max_known_ids":MAX_ENTRIES, "known_id_retention":"process_lifetime",
            "known_missing_or_orphan_receipt":"unavailable_no_replay",
            "index_membership_check":"before_pagination_without_reading_receipts",
            "automatic_repair":false, "complete_evidence_deletion_detected_after_restart":false},
        "record_publication":{"strategy":"same_filesystem_hard_link_then_unlink_pending",
            "overwrites_existing_record":false, "unsafe_fallback":false,
            "live_cleanup":"only_after_successful_exclusive_pending_create",
            "restart_cleanup":"remove_pending_names_never_promote",
            "hard_link_support_required":true},
        "directory_sync":cfg!(unix), "physical_power_loss_qualified":false,
        "response_rejection":{"code":"durable_response_unrecordable",
            "reasons":["byte_limit", "serialization", "restart_decode", "invalid_response"],
            "original_response_retained":false, "compact_receipt_commit_required":true,
            "blocks_other_admissions_after_commit":false,
            "invalid_retained_receipt":"inspect_error_without_rewrite_or_replay",
            "commit_failure":"outcome_unknown_and_block_new_admissions"},
        "terminal_record_shape":{"response_fields":["rpc_version", "id", "ok", "result", "error"],
            "error_fields":["code", "message", "details"],
            "unknown_envelope_fields":"reject_without_normalizing_or_rewriting",
            "result_and_error_details":"open_json_payloads",
            "generic_rpc_decoding_changed":false},
        "admission_record_shape":{"validation":"before_new_reservation_and_on_record_read",
            "request_digest":"64_lowercase_hex", "identity_max_bytes":256,
            "identity_text":"nonblank_control_free_without_normalization",
            "method_family":"solve_with_nonempty_suffix_or_run_operator_task_ir",
            "duplicate_binding":"method_and_request_digest", "index_scope":"returned_page_records",
            "payload_digest_recomputed_on_read":false, "operator_registry_consulted":false},
        "completion_boundary":if cfg!(unix) { "receipt_file_and_directory_sync_before_transport" }
            else { "receipt_file_sync_before_transport" },
        "recovery_policy":"retained_receipt_or_outcome_unknown_no_replay"})
}

#[cfg(test)]
#[path = "tests/agent_execution_store.rs"]
mod tests;
