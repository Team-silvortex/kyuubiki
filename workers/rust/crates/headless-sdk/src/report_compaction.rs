use serde_json::{Map, Value};
use std::borrow::Cow;

const MAX_REPORT_ARRAY_ITEMS: usize = 128;
const REPORT_ARRAY_SAMPLE_ITEMS: usize = 3;
const MAX_REPORT_STRING_BYTES: usize = 4_096;

pub(crate) fn compact_report_payload(value: Cow<'_, Value>) -> Value {
    match value {
        Cow::Borrowed(value) => compact_report_value(value),
        Cow::Owned(value) => compact_owned_report_value(value),
    }
}

pub(crate) fn compact_report_value(value: &Value) -> Value {
    match value {
        Value::Array(items) if items.len() > MAX_REPORT_ARRAY_ITEMS => array_summary(
            items.len(),
            items
                .iter()
                .take(REPORT_ARRAY_SAMPLE_ITEMS)
                .map(compact_report_value)
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(compact_report_value).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), compact_report_value(value)))
                .collect(),
        ),
        Value::String(text) if text.len() > MAX_REPORT_STRING_BYTES => string_summary(text),
        _ => value.clone(),
    }
}

pub(crate) fn compact_owned_report_value(value: Value) -> Value {
    match value {
        Value::Array(items) if items.len() > MAX_REPORT_ARRAY_ITEMS => {
            let count = items.len();
            // A fresh tiny vector prevents iterator collection from retaining the source capacity.
            let mut sample = Vec::with_capacity(REPORT_ARRAY_SAMPLE_ITEMS);
            for item in items.into_iter().take(REPORT_ARRAY_SAMPLE_ITEMS) {
                sample.push(compact_owned_report_value(item));
            }
            array_summary(count, sample)
        }
        Value::Array(mut items) => {
            for value in &mut items {
                *value = compact_owned_report_value(std::mem::take(value));
            }
            if items.capacity() > MAX_REPORT_ARRAY_ITEMS {
                items.shrink_to_fit();
            }
            Value::Array(items)
        }
        Value::Object(mut fields) => {
            for value in fields.values_mut() {
                *value = compact_owned_report_value(std::mem::take(value));
            }
            Value::Object(fields)
        }
        Value::String(text) if text.len() > MAX_REPORT_STRING_BYTES => string_summary(&text),
        Value::String(mut text) => {
            if text.capacity() > MAX_REPORT_STRING_BYTES {
                text.shrink_to_fit();
            }
            Value::String(text)
        }
        value => value,
    }
}

fn array_summary(count: usize, sample: Vec<Value>) -> Value {
    Value::Object(Map::from_iter([
        (
            "$kyuubiki_report_summary".into(),
            Value::String("array".into()),
        ),
        ("item_count".into(), Value::from(count as u64)),
        ("sample".into(), Value::Array(sample)),
        (
            "omitted_item_count".into(),
            Value::from((count - REPORT_ARRAY_SAMPLE_ITEMS) as u64),
        ),
    ]))
}

fn string_summary(text: &str) -> Value {
    Value::Object(Map::from_iter([
        (
            "$kyuubiki_report_summary".into(),
            Value::String("string".into()),
        ),
        ("byte_count".into(), Value::from(text.len() as u64)),
        (
            "prefix".into(),
            Value::String(text.chars().take(256).collect()),
        ),
    ]))
}

#[cfg(test)]
#[path = "report_compaction_tests.rs"]
mod tests;
