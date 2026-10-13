use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use kyuubiki_protocol::{
    DURABLE_AGENT_EXECUTION_SCHEMA, DurableAgentTask, RpcResponse, validate_durable_execution_id,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[path = "agent_execution_store_io.rs"]
mod io;

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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    record: Record,
    response: RpcResponse,
}

pub(crate) enum Admission {
    New(Record),
    Existing(Value),
}

pub(crate) struct Store {
    root: PathBuf,
    _lock: File,
    active: HashSet<String>,
    bytes: u64,
    entries: usize,
    write_failed: bool,
    uncertain_receipts: HashSet<String>,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self, String> {
        io::prepare_root(root)?;
        let lock_path = root.join("owner.lock");
        let lock = match io::open_private(&lock_path, true) {
            Ok(file) => file,
            Err(_) => io::open_private(&lock_path, false)?,
        };
        lock.try_lock_exclusive()
            .map_err(|e| format!("execution store is already owned: {e}"))?;
        let mut store = Self {
            root: root.to_path_buf(),
            _lock: lock,
            active: HashSet::new(),
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
            }
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
    }

    pub fn admit_with<T>(
        &mut self,
        task: &DurableAgentTask,
        process: &str,
        request_id: &str,
        reserve: impl FnOnce() -> Result<T, String>,
    ) -> Result<(Admission, Option<T>), String> {
        let digest = task.request_digest()?;
        if let Some(record) = self.record(&task.execution_id)? {
            if record.request_digest != digest {
                return Err("execution_id is already bound to different inputs or method".into());
            }
            return self
                .lookup_record(&record)
                .map(|report| (Admission::Existing(report), None));
        }
        if fs::symlink_metadata(self.root.join(format!("{}.receipt", task.execution_id))).is_ok() {
            return Err("orphan receipt blocks reuse of execution_id".into());
        }
        let reserved_bytes = (self.active.len() as u64 + 1) * (MAX_RECEIPT_BYTES as u64 + 65);
        if self.write_failed
            || self.entries >= MAX_ENTRIES
            || self.bytes + reserved_bytes + MAX_RECORD_BYTES as u64 + 65 > MAX_TOTAL_BYTES
        {
            return Err(
                "execution store admission blocked by capacity or a previous write failure".into(),
            );
        }
        let record = Record {
            schema_version: DURABLE_AGENT_EXECUTION_SCHEMA.into(),
            execution_id: task.execution_id.clone(),
            request_digest: digest,
            request_id: request_id.into(),
            process_instance_id: process.into(),
            method: serde_json::to_value(&task.method)
                .map_err(|e| e.to_string())?
                .as_str()
                .ok_or("invalid method")?
                .into(),
        };
        // Reserve before persisting admission, but never reserve on a duplicate lookup.
        let reservation = reserve()?;
        match io::write(
            &self.root,
            &format!("{}.admission", task.execution_id),
            &record,
            MAX_RECORD_BYTES,
        ) {
            Ok(bytes) => self.bytes += bytes,
            Err(error) => {
                self.write_failed = true;
                return Err(error);
            }
        }
        self.entries += 1;
        self.active.insert(record.execution_id.clone());
        Ok((Admission::New(record), Some(reservation)))
    }

    pub fn finish(&mut self, record: &Record, response: RpcResponse) -> Result<Value, String> {
        if !self.active.contains(&record.execution_id)
            || self.record(&record.execution_id)?.as_ref() != Some(record)
            || response.id != record.request_id
        {
            return Err("execution receipt does not match its active admission".into());
        }
        let receipt = Receipt {
            record: record.clone(),
            response,
        };
        let outcome = io::write(
            &self.root,
            &format!("{}.receipt", record.execution_id),
            &receipt,
            MAX_RECEIPT_BYTES,
        );
        self.active.remove(&record.execution_id);
        match outcome {
            Ok(bytes) => self.bytes += bytes,
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
        match self.record(id)? {
            Some(record) => self.lookup_record(&record),
            None => Ok(
                json!({"schema_version":DURABLE_AGENT_EXECUTION_SCHEMA, "execution_id":id,
                "status":"not_found", "automatic_replay_authorized":false}),
            ),
        }
    }

    pub fn list(
        &self,
        query: &kyuubiki_protocol::DurableAgentExecutionList,
    ) -> Result<Value, String> {
        query.validate()?;
        let mut ids = Vec::new();
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
                if query.after.as_deref().is_none_or(|after| id > after) {
                    ids.push(id.to_string());
                }
            }
        }
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
            "automatic_replay_authorized":false}),
        )
    }

    fn record(&self, id: &str) -> Result<Option<Record>, String> {
        validate_durable_execution_id(id)?;
        let record: Option<Record> =
            io::read(&self.root.join(format!("{id}.admission")), MAX_RECORD_BYTES)?;
        if let Some(record) = &record {
            if record.execution_id != id || record.schema_version != DURABLE_AGENT_EXECUTION_SCHEMA
            {
                return Err("execution admission identity or schema mismatch".into());
            }
        }
        Ok(record)
    }

    fn lookup_record(&self, record: &Record) -> Result<Value, String> {
        if self.uncertain_receipts.contains(&record.execution_id) {
            // A rename may have succeeded even when the directory sync failed.
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
            "execution_id":record.execution_id, "request_digest":record.request_digest,
            "request_id":record.request_id, "process_instance_id":record.process_instance_id,
            "method":record.method, "status":status, "response":response,
            "automatic_replay_authorized":false, "resumable":false})
    }

    pub fn policy(&self) -> Value {
        let mut policy = policy(true);
        policy["directory"] = json!(self.root);
        policy["stored_entries"] = json!(self.entries);
        policy["stored_bytes"] = json!(self.bytes);
        policy["active_reservations"] = json!(self.active.len());
        policy["admission_blocked_after_write_failure"] = json!(self.write_failed);
        policy
    }
}

pub(crate) fn policy(enabled: bool) -> Value {
    json!({"schema_version":DURABLE_AGENT_EXECUTION_SCHEMA, "enabled":enabled,
        "configuration_env":STORE_ENV, "storage":"immutable_checksummed_files",
        "max_entries":MAX_ENTRIES, "max_receipt_bytes":MAX_RECEIPT_BYTES, "max_total_bytes":MAX_TOTAL_BYTES,
        "automatic_eviction":false, "capacity_policy":"reject_new_admission",
        "survives_agent_restart":enabled, "automatic_replay_authorized":false,
        "directory_sync":cfg!(unix), "physical_power_loss_qualified":false,
        "completion_boundary":if cfg!(unix) { "receipt_file_and_directory_sync_before_transport" }
            else { "receipt_file_sync_before_transport" },
        "recovery_policy":"retained_receipt_or_outcome_unknown_no_replay"})
}

#[cfg(test)]
#[path = "tests/agent_execution_store.rs"]
mod tests;
