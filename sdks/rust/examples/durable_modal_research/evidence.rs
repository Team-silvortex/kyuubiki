use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::{
    fmt, fs,
    io::{self, Read},
    path::Path,
};

// This four-mode example has a bounded local evidence policy, not an SDK-wide limit.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
pub struct Document {
    pub bytes: Vec<u8>,
    pub value: Value,
}

fn invalid(name: &str, message: impl fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("evidence {name}: {message}"),
    )
}

fn contextual(name: &str, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("evidence {name}: {error}"))
}

pub fn read(root: &Path, name: &str) -> io::Result<Document> {
    read_optional(root, name)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("evidence {name}: required file is missing"),
        )
    })
}

/// Only an absent leaf is optional. Dangling links and invalid containers are errors.
pub fn read_optional(root: &Path, name: &str) -> io::Result<Option<Document>> {
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '\0'))
    {
        return Err(invalid("filename", "expected one fixed local filename"));
    }
    if !fs::symlink_metadata(root)
        .map_err(|e| contextual(name, e))?
        .is_dir()
    {
        return Err(invalid(name, "container must be a non-symlink directory"));
    }
    let path = root.join(name);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(contextual(name, error)),
    };
    if !metadata.is_file() {
        return Err(invalid(name, "expected a regular non-symlink file"));
    }
    if metadata.len() > MAX_BYTES {
        return Err(invalid(name, "exceeds the 8 MiB file limit"));
    }
    let file = fs::File::open(path).map_err(|e| contextual(name, e))?;
    let opened = file.metadata().map_err(|e| contextual(name, e))?;
    if !opened.is_file() || opened.len() > MAX_BYTES {
        return Err(invalid(name, "opened file is not regular or exceeds 8 MiB"));
    }
    // Do not reserve from metadata, and keep the read bounded even if the file grows.
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| contextual(name, e))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(invalid(name, "exceeds the 8 MiB file limit"));
    }
    let mut decoder = serde_json::Deserializer::from_slice(&bytes);
    let value = UniqueValue::deserialize(&mut decoder)
        .map_err(|e| invalid(name, e))?
        .0;
    decoder.end().map_err(|e| invalid(name, e))?;
    Ok(Some(Document { bytes, value }))
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        decoder.deserialize_any(UniqueVisitor)
    }
}

struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON with unique object keys")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(|number| UniqueValue(Value::Number(number)))
            .ok_or_else(|| E::custom("nonfinite JSON number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.visit_string(value.into())
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueValue>()? {
            values.push(value.0);
        }
        Ok(UniqueValue(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            values.insert(key, object.next_value::<UniqueValue>()?.0);
        }
        Ok(UniqueValue(Value::Object(values)))
    }
}
