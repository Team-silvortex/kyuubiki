use crate::{migration::ProjectMigrationLimits, paths};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub(crate) fn source_path(input: &str) -> Result<PathBuf, String> {
    let path = Path::new(input);
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect project input: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("migration requires a regular .json or .kyuubiki file, not a directory or symbolic link".into());
    }
    if !paths::has_extension(path, "json") && !paths::has_extension(path, "kyuubiki") {
        return Err("migration input must end with .json or .kyuubiki".into());
    }
    path.canonicalize().map_err(|error| error.to_string())
}

pub(crate) fn hash_file(path: &Path, limit: u64) -> Result<(String, u64), String> {
    let mut input = File::open(path).map_err(|error| error.to_string())?;
    hash_reader(&mut input, limit)
}

fn hash_reader(input: &mut impl Read, limit: u64) -> Result<(String, u64), String> {
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|error| format!("cannot read project data: {error}"))?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .filter(|total| *total <= limit)
            .ok_or("project data exceeds configured byte limit")?;
        hash.update(&buffer[..count]);
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}

pub(crate) fn copy_source(source: &Path, target: &Path, limit: u64) -> Result<(), String> {
    let mut input = File::open(source).map_err(|error| error.to_string())?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options.open(target).map_err(|error| error.to_string())?;
    let count = std::io::copy(
        &mut Read::by_ref(&mut input).take(limit.saturating_add(1)),
        &mut output,
    )
    .map_err(|error| error.to_string())?;
    if count > limit {
        return Err("source exceeds configured byte limit".into());
    }
    output.flush().map_err(|error| error.to_string())
}

pub(crate) fn read_source(path: &Path, limits: &ProjectMigrationLimits) -> Result<Value, String> {
    if limits.max_manifest_bytes == 0 || limits.max_entries == 0 {
        return Err("migration limits must be positive".into());
    }
    if paths::has_extension(path, "kyuubiki") {
        let mut archive = open_archive(path, limits)?;
        inventory(&mut archive, limits)?;
        read_json_entry(&mut archive, "project.json", limits.max_manifest_bytes)
    } else {
        let file = File::open(path).map_err(|error| error.to_string())?;
        read_json(file, limits.max_manifest_bytes)
    }
}

pub(crate) fn open_archive(
    path: &Path,
    limits: &ProjectMigrationLimits,
) -> Result<ZipArchive<File>, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let declared = crate::migration_zip::check_directory(&mut file, limits)?;
    let archive =
        ZipArchive::new(file).map_err(|error| format!("invalid project archive: {error}"))?;
    if archive.len() != declared {
        return Err("ZIP reader merged ambiguous physical entries".into());
    }
    Ok(archive)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EntryDigest {
    pub sha256: String,
    pub bytes: u64,
    pub directory: bool,
}

pub(crate) fn inventory(
    archive: &mut ZipArchive<File>,
    limits: &ProjectMigrationLimits,
) -> Result<BTreeMap<String, EntryDigest>, String> {
    if archive.len() > limits.max_entries {
        return Err("archive exceeds configured entry limit".into());
    }
    let mut names = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut inventory = BTreeMap::new();
    let mut remaining = limits.max_expanded_bytes;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("cannot read archive entry: {error}"))?;
        let name = entry.name().to_string();
        if name == "project.json" && entry.size() > limits.max_manifest_bytes {
            return Err("project.json exceeds configured manifest byte limit".into());
        }
        safe_name(&name)?;
        let key = name.trim_end_matches('/').to_lowercase();
        if !names.insert(key.clone()) {
            return Err(format!("duplicate or case-colliding archive entry: {name}"));
        }
        if let Some(mode) = entry.unix_mode() {
            let kind = mode & 0o170000;
            if kind != 0 && kind != 0o100000 && kind != 0o040000 {
                return Err(format!("archive contains a link or special file: {name}"));
            }
        }
        let directory = entry.is_dir();
        if !directory {
            files.insert(key);
        }
        if entry.size() > remaining {
            return Err("archive exceeds configured expanded byte limit".into());
        }
        let (sha256, bytes) = hash_reader(&mut entry, remaining)?;
        if bytes != entry.size() || (directory && bytes != 0) {
            return Err(format!("archive entry size mismatch: {name}"));
        }
        remaining -= bytes;
        inventory.insert(
            name,
            EntryDigest {
                sha256,
                bytes,
                directory,
            },
        );
    }
    for name in &names {
        let mut parent = name.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            if files.contains(prefix) {
                return Err(format!("archive file/directory collision: {name}"));
            }
            parent = prefix;
        }
    }
    Ok(inventory)
}

pub(crate) fn safe_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 1024
        || name.contains(['\\', ':'])
        || name.chars().any(char::is_control)
    {
        return Err(format!("unsafe or non-portable project entry: {name:?}"));
    }
    for part in name.trim_end_matches('/').split('/') {
        let device = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        let reserved = matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (device.len() == 4
                && (device.starts_with("COM") || device.starts_with("LPT"))
                && matches!(device.as_bytes()[3], b'1'..=b'9'));
        if matches!(part, "" | "." | "..")
            || part.ends_with(['.', ' '])
            || reserved
            || part.contains(['<', '>', '"', '|', '?', '*'])
        {
            return Err(format!("unsafe or non-portable project entry: {name:?}"));
        }
    }
    Ok(())
}

pub(crate) fn read_json_entry(
    archive: &mut ZipArchive<File>,
    name: &str,
    limit: u64,
) -> Result<Value, String> {
    let entry = archive
        .by_name(name)
        .map_err(|error| format!("cannot read {name}: {error}"))?;
    read_json(entry, limit).map_err(|error| format!("invalid {name}: {error}"))
}

pub(crate) fn read_json(reader: impl Read, limit: u64) -> Result<Value, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("JSON exceeds configured manifest byte limit".into());
    }
    serde_json::from_slice::<UniqueValue>(&bytes)
        .map(|value| value.0)
        .map_err(|error| format!("invalid or ambiguous JSON: {error}"))
}

// Standard JSON decoding silently keeps the last duplicate key. An upgrade must not
// silently choose between conflicting scientific records or version declarations.
struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = access.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
                    }
                    values.insert(key, access.next_value::<UniqueValue>()?.0);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
