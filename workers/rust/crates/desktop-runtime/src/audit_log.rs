use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const RECORD_SCHEMA: &str = "kyuubiki.desktop-audit-record/v1";
const STATUS_SCHEMA: &str = "kyuubiki.desktop-audit-ledger-status/v1";
const GENESIS_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

static LEDGER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DesktopAuditLedgerStatus {
    pub schema_version: String,
    pub ledger_schema: String,
    pub status: String,
    pub digest_algorithm: String,
    pub record_count: usize,
    pub head_digest: Option<String>,
    pub ledger_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DesktopAuditRecord {
    schema_version: String,
    ledger_id: String,
    sequence: u64,
    previous_digest: String,
    payload: Value,
    record_digest: String,
}

#[derive(Serialize)]
struct DigestMaterial<'a> {
    schema_version: &'a str,
    ledger_id: &'a str,
    sequence: u64,
    previous_digest: &'a str,
    payload: &'a Value,
}

pub fn append_desktop_provenance_record(
    file_name: &str,
    payload: &Value,
) -> Result<DesktopAuditLedgerStatus, String> {
    validate_file_name(file_name)?;
    if !payload.is_object() {
        return Err("desktop audit payload must be a JSON object".to_string());
    }
    let path = super::desktop_audit_path(file_name)?;
    append_record_at(&path, file_name, payload)
}

pub fn desktop_provenance_status(file_name: &str) -> Result<DesktopAuditLedgerStatus, String> {
    validate_file_name(file_name)?;
    let path = super::desktop_audit_path(file_name)?;
    with_ledger_lock(|| status_at(&path, file_name))
}

pub fn prepare_desktop_provenance_ledger(
    file_name: &str,
) -> Result<DesktopAuditLedgerStatus, String> {
    validate_file_name(file_name)?;
    let path = super::desktop_audit_path(file_name)?;
    with_ledger_lock(|| prepare_at(&path, file_name))
}

fn prepare_at(path: &Path, ledger_id: &str) -> Result<DesktopAuditLedgerStatus, String> {
    let records = load_or_migrate_records(path, ledger_id)?;
    Ok(status_from_records(
        path,
        records.len(),
        records.last().map(|record| record.record_digest.clone()),
    ))
}

fn append_record_at(
    path: &Path,
    ledger_id: &str,
    payload: &Value,
) -> Result<DesktopAuditLedgerStatus, String> {
    with_ledger_lock(|| {
        let records = load_or_migrate_records(path, ledger_id)?;
        let previous_digest = records
            .last()
            .map(|record| record.record_digest.clone())
            .unwrap_or_else(|| GENESIS_DIGEST.to_string());
        let mut record = DesktopAuditRecord {
            schema_version: RECORD_SCHEMA.to_string(),
            ledger_id: ledger_id.to_string(),
            sequence: records.len() as u64 + 1,
            previous_digest,
            payload: payload.clone(),
            record_digest: String::new(),
        };
        record.record_digest = record_digest(&record)?;

        let parent = path
            .parent()
            .ok_or_else(|| "desktop audit ledger has no parent directory".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
        let mut bytes = serde_json::to_vec(&record)
            .map_err(|error| format!("failed to encode desktop audit record: {error}"))?;
        bytes.push(b'\n');
        file.write_all(&bytes)
            .map_err(|error| format!("failed to append {}: {error}", path.display()))?;
        file.flush()
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("failed to sync {}: {error}", path.display()))?;
        Ok(status_from_records(
            path,
            records.len() + 1,
            Some(record.record_digest),
        ))
    })
}

fn load_or_migrate_records(
    path: &Path,
    ledger_id: &str,
) -> Result<Vec<DesktopAuditRecord>, String> {
    recover_interrupted_migration(path, ledger_id)?;
    match read_verified_records(path, ledger_id) {
        Ok(records) => {
            clear_migration_sidecars(path)?;
            Ok(records)
        }
        Err(verification_error) => {
            let payloads = read_legacy_payloads(path).map_err(|legacy_error| {
                format!(
                    "{verification_error}; legacy desktop audit migration rejected: {legacy_error}"
                )
            })?;
            migrate_legacy_records(path, ledger_id, &payloads)
        }
    }
}

fn recover_interrupted_migration(path: &Path, ledger_id: &str) -> Result<(), String> {
    let next = migration_sidecar(path, ".migration.next");
    let previous = migration_sidecar(path, ".migration.previous");
    if !path.exists() && previous.exists() {
        fs::rename(&previous, path).map_err(|error| {
            format!(
                "failed to restore interrupted audit migration {}: {error}",
                path.display()
            )
        })?;
    } else if !path.exists() && next.exists() && read_verified_records(&next, ledger_id).is_ok() {
        fs::rename(&next, path).map_err(|error| {
            format!(
                "failed to activate interrupted audit migration {}: {error}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn read_legacy_payloads(path: &Path) -> Result<Vec<Value>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut payloads = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let payload: Value = serde_json::from_str(line).map_err(|error| {
            format!("legacy audit record {} is invalid JSON: {error}", index + 1)
        })?;
        let object = payload
            .as_object()
            .ok_or_else(|| format!("legacy audit record {} is not an object", index + 1))?;
        if object.contains_key("record_digest")
            || object.contains_key("previous_digest")
            || object.contains_key("sequence")
            || object.contains_key("ledger_id")
            || object.get("schema_version").and_then(Value::as_str) == Some(RECORD_SCHEMA)
        {
            return Err(format!(
                "record {} resembles a chain record and cannot be re-signed",
                index + 1
            ));
        }
        payloads.push(payload);
    }
    if payloads.is_empty() {
        return Err("legacy audit ledger contains no records".to_string());
    }
    Ok(payloads)
}

fn migrate_legacy_records(
    path: &Path,
    ledger_id: &str,
    payloads: &[Value],
) -> Result<Vec<DesktopAuditRecord>, String> {
    let records = build_record_chain(ledger_id, payloads)?;
    let next = migration_sidecar(path, ".migration.next");
    let previous = migration_sidecar(path, ".migration.previous");
    remove_if_exists(&next)?;
    remove_if_exists(&previous)?;

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&next)
        .map_err(|error| format!("failed to create {}: {error}", next.display()))?;
    for record in &records {
        let mut bytes = serde_json::to_vec(record)
            .map_err(|error| format!("failed to encode migrated audit record: {error}"))?;
        bytes.push(b'\n');
        file.write_all(&bytes)
            .map_err(|error| format!("failed to write {}: {error}", next.display()))?;
    }
    file.flush()
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("failed to sync {}: {error}", next.display()))?;

    fs::rename(path, &previous)
        .map_err(|error| format!("failed to rotate {}: {error}", path.display()))?;
    if let Err(error) = fs::rename(&next, path) {
        let _ = fs::rename(&previous, path);
        return Err(format!(
            "failed to activate migrated audit ledger {}: {error}",
            path.display()
        ));
    }
    remove_if_exists(&previous)?;
    read_verified_records(path, ledger_id)
}

fn build_record_chain(
    ledger_id: &str,
    payloads: &[Value],
) -> Result<Vec<DesktopAuditRecord>, String> {
    let mut previous_digest = GENESIS_DIGEST.to_string();
    let mut records = Vec::with_capacity(payloads.len());
    for (index, payload) in payloads.iter().enumerate() {
        let mut record = DesktopAuditRecord {
            schema_version: RECORD_SCHEMA.to_string(),
            ledger_id: ledger_id.to_string(),
            sequence: index as u64 + 1,
            previous_digest,
            payload: payload.clone(),
            record_digest: String::new(),
        };
        record.record_digest = record_digest(&record)?;
        previous_digest = record.record_digest.clone();
        records.push(record);
    }
    Ok(records)
}

fn clear_migration_sidecars(path: &Path) -> Result<(), String> {
    remove_if_exists(&migration_sidecar(path, ".migration.next"))?;
    remove_if_exists(&migration_sidecar(path, ".migration.previous"))
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", path.display())),
    }
}

fn migration_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn status_at(path: &Path, ledger_id: &str) -> Result<DesktopAuditLedgerStatus, String> {
    let records = read_verified_records(path, ledger_id)?;
    Ok(status_from_records(
        path,
        records.len(),
        records.last().map(|record| record.record_digest.clone()),
    ))
}

fn status_from_records(
    path: &Path,
    record_count: usize,
    head_digest: Option<String>,
) -> DesktopAuditLedgerStatus {
    DesktopAuditLedgerStatus {
        schema_version: STATUS_SCHEMA.to_string(),
        ledger_schema: RECORD_SCHEMA.to_string(),
        status: "verified".to_string(),
        digest_algorithm: "sha256".to_string(),
        record_count,
        head_digest,
        ledger_path: path.display().to_string(),
    }
}

fn read_verified_records(path: &Path, ledger_id: &str) -> Result<Vec<DesktopAuditRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut records = Vec::new();
    let mut expected_previous = GENESIS_DIGEST.to_string();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            return Err(format!(
                "desktop audit ledger contains an empty record at line {}",
                index + 1
            ));
        }
        let record: DesktopAuditRecord = serde_json::from_str(line).map_err(|error| {
            format!(
                "desktop audit ledger record {} is invalid JSON: {error}",
                index + 1
            )
        })?;
        verify_record(&record, ledger_id, index as u64 + 1, &expected_previous)?;
        expected_previous = record.record_digest.clone();
        records.push(record);
    }
    Ok(records)
}

fn verify_record(
    record: &DesktopAuditRecord,
    ledger_id: &str,
    sequence: u64,
    previous_digest: &str,
) -> Result<(), String> {
    if record.schema_version != RECORD_SCHEMA
        || record.ledger_id != ledger_id
        || record.sequence != sequence
        || record.previous_digest != previous_digest
    {
        return Err(format!(
            "desktop audit ledger chain mismatch at sequence {sequence}"
        ));
    }
    let actual = record_digest(record)?;
    if record.record_digest != actual {
        return Err(format!(
            "desktop audit ledger digest mismatch at sequence {sequence}"
        ));
    }
    Ok(())
}

fn record_digest(record: &DesktopAuditRecord) -> Result<String, String> {
    let material = DigestMaterial {
        schema_version: &record.schema_version,
        ledger_id: &record.ledger_id,
        sequence: record.sequence,
        previous_digest: &record.previous_digest,
        payload: &record.payload,
    };
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("failed to encode desktop audit digest material: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn with_ledger_lock<T>(run: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let _guard = LEDGER_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "desktop audit ledger lock is poisoned".to_string())?;
    run()
}

fn validate_file_name(file_name: &str) -> Result<(), String> {
    if file_name.is_empty()
        || file_name.len() > 128
        || !file_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("desktop audit ledger file name is invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{append_record_at, migration_sidecar, prepare_at, status_at};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn append_and_reload_preserve_hash_chain() {
        let root = fixture_root("append-reload");
        let path = root.join("installer.jsonl");
        append_record_at(&path, "installer.jsonl", &json!({"action": "prepare"})).unwrap();
        let status = append_record_at(
            &path,
            "installer.jsonl",
            &json!({"action": "apply", "status": "ok"}),
        )
        .unwrap();
        assert_eq!(status.record_count, 2);
        assert_eq!(status.head_digest.as_deref().map(str::len), Some(64));
        assert_eq!(status_at(&path, "installer.jsonl").unwrap(), status);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tampered_record_is_rejected_before_append() {
        let root = fixture_root("tamper");
        let path = root.join("installer.jsonl");
        append_record_at(&path, "installer.jsonl", &json!({"action": "prepare"})).unwrap();
        let mut record: Value =
            serde_json::from_str(fs::read_to_string(&path).unwrap().trim()).unwrap();
        record["payload"]["action"] = json!("tampered");
        fs::write(
            &path,
            format!("{}\n", serde_json::to_string(&record).unwrap()),
        )
        .unwrap();
        let error =
            append_record_at(&path, "installer.jsonl", &json!({"action": "apply"})).unwrap_err();
        assert!(error.contains("digest mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ledger_file_names_cannot_escape_the_preferences_sandbox() {
        assert!(super::validate_file_name("installer-guarded-mutations.jsonl").is_ok());
        assert!(super::validate_file_name("../outside.jsonl").is_err());
        assert!(super::validate_file_name("nested/audit.jsonl").is_err());
    }

    #[test]
    fn legacy_json_lines_migrate_without_residue() {
        let root = fixture_root("legacy-migration");
        let path = root.join("installer.jsonl");
        fs::write(
            &path,
            "{\"action\":\"prepare\"}\n{\"action\":\"apply\",\"status\":\"ok\"}\n",
        )
        .unwrap();

        let status = prepare_at(&path, "installer.jsonl").unwrap();
        assert_eq!(status.record_count, 2);
        let appended = append_record_at(
            &path,
            "installer.jsonl",
            &json!({"action": "verify", "status": "ok"}),
        )
        .unwrap();
        assert_eq!(appended.record_count, 3);
        assert_eq!(status_at(&path, "installer.jsonl").unwrap(), appended);
        assert!(!migration_sidecar(&path, ".migration.next").exists());
        assert!(!migration_sidecar(&path, ".migration.previous").exists());

        let interrupted = root.join("interrupted.jsonl");
        let previous = migration_sidecar(&interrupted, ".migration.previous");
        fs::write(&previous, "{\"action\":\"prepare\"}\n").unwrap();
        let recovered = prepare_at(&interrupted, "interrupted.jsonl").unwrap();
        assert_eq!(recovered.record_count, 1);
        assert!(interrupted.exists());
        assert!(!previous.exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn fixture_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-desktop-audit-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
