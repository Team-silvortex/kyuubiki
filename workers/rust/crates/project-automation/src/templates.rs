use serde_json::{Map, Value};

pub(crate) fn resolve(value: Value, payload: &Value, state: &Value) -> Value {
    match value {
        Value::String(text) => resolve_string(&text, payload, state),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| resolve(value, payload, state))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, resolve(value, payload, state)))
                .collect::<Map<_, _>>(),
        ),
        other => other,
    }
}

fn resolve_string(text: &str, payload: &Value, state: &Value) -> Value {
    if let Some((source, key)) = exact_binding(text) {
        return lookup(source, key, payload, state)
            .cloned()
            .unwrap_or(Value::Null);
    }
    let mut output = String::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("{{") {
        output.push_str(&remaining[..start]);
        let tail = &remaining[start + 2..];
        let Some(end) = tail.find("}}") else {
            output.push_str(&remaining[start..]);
            return Value::String(output);
        };
        let expression = tail[..end].trim();
        if let Some((source, key)) = expression.split_once('.') {
            if matches!(source.trim(), "payload" | "state") && valid_key(key.trim()) {
                if let Some(value) = lookup(source.trim(), key.trim(), payload, state) {
                    output.push_str(&inline_value(value));
                }
            } else {
                output.push_str(&remaining[start..start + 4 + end]);
            }
        } else {
            output.push_str(&remaining[start..start + 4 + end]);
        }
        remaining = &tail[end + 2..];
    }
    output.push_str(remaining);
    Value::String(output)
}

fn exact_binding(text: &str) -> Option<(&str, &str)> {
    let expression = text.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let (source, key) = expression.split_once('.')?;
    let source = source.trim();
    let key = key.trim();
    (matches!(source, "payload" | "state") && valid_key(key)).then_some((source, key))
}

fn valid_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn lookup<'a>(source: &str, key: &str, payload: &'a Value, state: &'a Value) -> Option<&'a Value> {
    if source == "payload" {
        payload.get(key)
    } else {
        state.get(key)
    }
}

fn inline_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}
