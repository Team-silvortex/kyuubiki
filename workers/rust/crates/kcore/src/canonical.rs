use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).expect("JSON string serialization"),
        Value::Array(values) => {
            let values = values.iter().map(json).collect::<Vec<_>>().join(",");
            format!("[{values}]")
        }
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let fields = keys
                .into_iter()
                .map(|key| {
                    let encoded_key = serde_json::to_string(key).expect("JSON key serialization");
                    format!("{encoded_key}:{}", json(&values[key]))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
    }
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::json;
    use serde_json::json as value;

    #[test]
    fn canonical_json_sorts_nested_keys() {
        assert_eq!(
            json(&value!({"z": {"b": 2, "a": 1}, "a": [2, 1]})),
            r#"{"a":[2,1],"z":{"a":1,"b":2}}"#
        );
    }
}
