use super::{compare_versions, parse_artifacts, parse_rules, select_channel};
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

const UPDATE_CATALOG_JSON_FUZZ_CASES: usize = 384;
const UPDATE_CATALOG_BYTE_FUZZ_CASES: usize = 256;

#[test]
fn installer_update_catalog_fuzz_smoke_does_not_panic_on_mutated_json() {
    let mut rng = FuzzRng::new(0x55_50_43_41_54_4a_53_4f);
    for case_index in 0..UPDATE_CATALOG_JSON_FUZZ_CASES {
        let value = fuzz_catalog_json(&mut rng, case_index);
        let channel = maybe_channel_name(&mut rng);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            exercise_update_catalog_boundary(&value, channel.as_deref());
        }));
        assert!(
            outcome.is_ok(),
            "installer update catalog JSON fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn installer_update_catalog_fuzz_smoke_does_not_panic_on_byte_ingress() {
    let mut rng = FuzzRng::new(0x55_50_43_41_54_42_59_54);
    for case_index in 0..UPDATE_CATALOG_BYTE_FUZZ_CASES {
        let bytes = fuzz_catalog_bytes(&mut rng, case_index);
        let channel = maybe_channel_name(&mut rng);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                exercise_update_catalog_boundary(&value, channel.as_deref());
            }
        }));
        assert!(
            outcome.is_ok(),
            "installer update catalog byte fuzz case {case_index} panicked"
        );
    }
}

fn exercise_update_catalog_boundary(catalog: &Value, requested: Option<&str>) {
    if let Ok(channel) = select_channel(catalog, requested) {
        let _ = parse_rules(channel.get("visible_rules"));
        let _ = parse_artifacts(Path::new("."), channel.get("desktop_artifacts"));
        let current = channel
            .get("current_version")
            .and_then(Value::as_str)
            .unwrap_or("1.17.8");
        let target = channel
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("1.17.8");
        let _ = compare_versions(current, target);
    }
}

fn fuzz_catalog_json(rng: &mut FuzzRng, case_index: usize) -> Value {
    if rng.one_in(4) {
        return fuzz_json(rng, 0, 7);
    }

    let mut value = catalog_fixture(rng, case_index);
    mutate_json(rng, &mut value, 0);
    value
}

fn catalog_fixture(rng: &mut FuzzRng, case_index: usize) -> Value {
    json!({
        "schema_version": "kyuubiki.update-catalog/v1",
        "default_channel": if rng.one_in(8) { "missing" } else { "stable" },
        "channels": [
            channel_fixture("stable", "1.17.8", case_index),
            channel_fixture("preview", "1.17.0", case_index + 1)
        ]
    })
}

fn channel_fixture(id: &str, version: &str, case_index: usize) -> Value {
    json!({
        "id": id,
        "tag": format!("{id}-tag"),
        "version": version,
        "current_version": "1.17.8",
        "summary": format!("fuzz channel {case_index}"),
        "aliases": [id, format!("{id}-alias")],
        "visible_rules": [
            {
                "label": "download source",
                "value": "configured",
                "description": "Fuzz fixture source rule."
            }
        ],
        "desktop_artifacts": [
            {
                "product": "hub",
                "platform": "macos",
                "kind": "app",
                "path": format!("dist/fuzz/hub-{case_index}.tar.gz")
            }
        ]
    })
}

fn maybe_channel_name(rng: &mut FuzzRng) -> Option<String> {
    match rng.usize(5) {
        0 => None,
        1 => Some("stable".to_string()),
        2 => Some("preview-tag".to_string()),
        3 => Some("stable-alias".to_string()),
        _ => Some(fuzz_text(rng, 96)),
    }
}

fn fuzz_catalog_bytes(rng: &mut FuzzRng, case_index: usize) -> Vec<u8> {
    if rng.one_in(4) {
        return fuzz_raw_bytes(rng, 4096);
    }

    let value = fuzz_catalog_json(rng, case_index);
    let mut bytes = serde_json::to_vec(&value).expect("fuzz JSON value should serialize");
    mutate_bytes(rng, &mut bytes);
    bytes
}

fn mutate_json(rng: &mut FuzzRng, value: &mut Value, depth: usize) {
    if depth > 5 {
        return;
    }

    match value {
        Value::Object(object) => {
            if rng.one_in(6) {
                object.remove(pick_catalog_field(rng));
            }
            if rng.one_in(5) {
                object.insert(fuzz_text(rng, 96), fuzz_json(rng, 0, 4));
            }
            for item in object.values_mut() {
                if rng.bool() {
                    mutate_json(rng, item, depth + 1);
                }
            }
        }
        Value::Array(items) => {
            if rng.one_in(5) {
                items.push(fuzz_json(rng, 0, 4));
            }
            for item in items {
                if rng.bool() {
                    mutate_json(rng, item, depth + 1);
                }
            }
        }
        Value::String(text) => {
            if rng.one_in(5) {
                *text = fuzz_text(rng, 512);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            if rng.one_in(8) {
                *value = fuzz_json(rng, 0, 3);
            }
        }
    }
}

fn pick_catalog_field(rng: &mut FuzzRng) -> &'static str {
    const FIELDS: &[&str] = &[
        "schema_version",
        "default_channel",
        "channels",
        "id",
        "tag",
        "version",
        "aliases",
        "visible_rules",
        "desktop_artifacts",
    ];
    FIELDS[rng.usize(FIELDS.len())]
}

fn fuzz_json(rng: &mut FuzzRng, depth: usize, max_depth: usize) -> Value {
    if depth >= max_depth {
        return Value::String(fuzz_text(rng, 64));
    }
    match rng.usize(6) {
        0 => Value::Null,
        1 => Value::Bool(rng.bool()),
        2 => Value::Number(serde_json::Number::from(rng.usize(10_000) as u64)),
        3 => {
            let max_len = 256 + rng.usize(1024);
            Value::String(fuzz_text(rng, max_len))
        }
        4 => Value::Array(
            (0..rng.usize(6))
                .map(|_| fuzz_json(rng, depth + 1, max_depth))
                .collect(),
        ),
        _ => {
            let mut object = serde_json::Map::new();
            for index in 0..rng.usize(6) {
                let key = if rng.one_in(8) {
                    fuzz_text(rng, 256)
                } else {
                    format!("key_{depth}_{index}")
                };
                object.insert(key, fuzz_json(rng, depth + 1, max_depth));
            }
            Value::Object(object)
        }
    }
}

fn mutate_bytes(rng: &mut FuzzRng, bytes: &mut Vec<u8>) {
    if rng.one_in(5) {
        bytes.truncate(rng.usize(bytes.len().saturating_add(1)));
    }
    if rng.one_in(5) {
        bytes.extend(fuzz_raw_bytes(rng, 256));
    }
    for byte in bytes.iter_mut() {
        if rng.one_in(64) {
            *byte = rng.byte();
        }
    }
}

fn fuzz_raw_bytes(rng: &mut FuzzRng, max_len: usize) -> Vec<u8> {
    let len = rng.usize(max_len.saturating_add(1));
    (0..len).map(|_| rng.byte()).collect()
}

fn fuzz_text(rng: &mut FuzzRng, max_len: usize) -> String {
    let len = rng.usize(max_len.saturating_add(1));
    (0..len)
        .map(|_| match rng.usize(8) {
            0 => '\0',
            1 => ' ',
            2 => ':',
            3 => '/',
            4 => '.',
            5 => '-',
            6 => '_',
            _ => char::from_u32(0x4e00 + rng.usize(128) as u32).unwrap_or('x'),
        })
        .collect()
}

struct FuzzRng {
    state: u64,
}

impl FuzzRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            (self.next() as usize) % upper
        }
    }

    fn bool(&mut self) -> bool {
        self.next() & 1 == 0
    }

    fn one_in(&mut self, divisor: usize) -> bool {
        self.usize(divisor) == 0
    }

    fn byte(&mut self) -> u8 {
        (self.next() & 0xff) as u8
    }
}
