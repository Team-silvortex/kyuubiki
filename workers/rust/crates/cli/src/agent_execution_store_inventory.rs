use super::{MAX_ENTRIES, validate_durable_execution_id};
use std::{collections::BTreeMap, fs, path::Path};

/// Finalized record accounting, not content validation or protection against
/// concurrent filesystem mutation. Only startup and successful commits update it.
#[derive(Default)]
pub(super) struct Inventory {
    records: BTreeMap<String, u64>,
    pub blocked: bool,
}

impl Inventory {
    pub fn remember(&mut self, name: String, bytes: u64) {
        self.records.insert(name, bytes);
    }

    pub fn check(&mut self, root: &Path) -> Result<(), String> {
        if self.blocked || self.matches(root).is_err() {
            self.blocked = true;
            return Err("execution store inventory changed or could not be verified; stop Agent, inspect original records and restart; no replay authorized".into());
        }
        Ok(())
    }

    fn matches(&self, root: &Path) -> Result<(), String> {
        let mut finalized = 0;
        for (count, entry) in fs::read_dir(root).map_err(|e| e.to_string())?.enumerate() {
            if count > 4 * MAX_ENTRIES {
                return Err("execution store file count exceeds limit".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name = name.to_str().ok_or("invalid execution store filename")?;
            if name == "owner.lock" {
                continue; // The locked owner is checked separately by Store.
            }
            let (id, suffix) = name.split_once('.').ok_or("foreign store entry")?;
            validate_durable_execution_id(id)?;
            if matches!(suffix, "admission.next" | "receipt.next") {
                // Pending files never count as committed evidence. Leave them to
                // exclusive publication/write-failure fencing and restart cleanup.
                continue;
            }
            if !matches!(suffix, "admission" | "receipt") {
                return Err("foreign store entry".into());
            }
            let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || self.records.get(name) != Some(&metadata.len())
            {
                return Err("finalized record inventory mismatch".into());
            }
            finalized += 1;
        }
        if finalized != self.records.len() {
            return Err("finalized record missing".into());
        }
        Ok(())
    }
}
