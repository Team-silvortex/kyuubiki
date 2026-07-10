use crate::{
    OPERATOR_PACKAGE_MANIFEST_FILE, OPERATOR_PACKAGE_SCHEMA_VERSION, OPERATOR_SDK_API_VERSION,
    OperatorPackageManifest, read_operator_package_manifest,
};
use serde_json::{Value, json};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MANIFEST_JSON_FUZZ_CASES: usize = 384;
const MANIFEST_BYTE_FUZZ_CASES: usize = 256;

#[test]
fn operator_package_manifest_fuzz_smoke_does_not_panic_on_mutated_json() {
    let root = temp_dir("operator-manifest-json-fuzz");
    let manifest_path = root.join(OPERATOR_PACKAGE_MANIFEST_FILE);
    let mut rng = FuzzRng::new(0x4f_50_4d_41_4e_4a_53_4f);

    for case_index in 0..MANIFEST_JSON_FUZZ_CASES {
        let value = fuzz_manifest_json(&mut rng, case_index);
        let bytes = serde_json::to_vec(&value).expect("fuzz JSON value should serialize");
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            fs::write(&manifest_path, bytes).expect("write fuzz manifest");
            let _ = read_operator_package_manifest(&manifest_path);
            let _ = serde_json::from_value::<OperatorPackageManifest>(value);
        }));
        assert!(
            outcome.is_ok(),
            "operator package manifest JSON fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn operator_package_manifest_fuzz_smoke_does_not_panic_on_byte_ingress() {
    let root = temp_dir("operator-manifest-byte-fuzz");
    let manifest_path = root.join(OPERATOR_PACKAGE_MANIFEST_FILE);
    let mut rng = FuzzRng::new(0x4f_50_4d_41_4e_42_59_54);

    for case_index in 0..MANIFEST_BYTE_FUZZ_CASES {
        let bytes = fuzz_manifest_bytes(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            fs::write(&manifest_path, bytes).expect("write fuzz manifest");
            let _ = read_operator_package_manifest(&manifest_path);
        }));
        assert!(
            outcome.is_ok(),
            "operator package manifest byte fuzz case {case_index} panicked"
        );
    }
}

fn fuzz_manifest_json(rng: &mut FuzzRng, case_index: usize) -> Value {
    if rng.one_in(4) {
        return fuzz_json(rng, 0, 7);
    }

    let mut value = manifest_fixture(rng, case_index);
    mutate_json(rng, &mut value, 0);
    value
}

fn manifest_fixture(rng: &mut FuzzRng, case_index: usize) -> Value {
    json!({
        "schema_version": if rng.one_in(8) {
            fuzz_text(rng, 64)
        } else {
            OPERATOR_PACKAGE_SCHEMA_VERSION.to_string()
        },
        "sdk_api_version": if rng.one_in(8) {
            fuzz_text(rng, 64)
        } else {
            OPERATOR_SDK_API_VERSION.to_string()
        },
        "package_id": format!("operator.fuzz.{case_index}"),
        "package_version": "0.1.0",
        "minimum_host_version": "1.16.0",
        "validation_status": pick_validation_status(rng),
        "validation_notes": "Deterministic manifest fuzz-smoke fixture.",
        "runtime": pick_runtime(rng),
        "entrypoint": "target/debug/liboperator_fuzz",
        "operators": [
            {
                "operator_id": format!("transform.fuzz.{case_index}"),
                "kind": "transform",
                "entry_symbol": "register_operator"
            }
        ]
    })
}

fn pick_validation_status(rng: &mut FuzzRng) -> &'static str {
    const STATUSES: &[&str] = &["verified", "partial", "experimental", "bad_status"];
    STATUSES[rng.usize(STATUSES.len())]
}

fn pick_runtime(rng: &mut FuzzRng) -> &'static str {
    const RUNTIMES: &[&str] = &["rust_crate", "wasm_component", "python_wheel", ""];
    RUNTIMES[rng.usize(RUNTIMES.len())]
}

fn fuzz_manifest_bytes(rng: &mut FuzzRng, case_index: usize) -> Vec<u8> {
    if rng.one_in(4) {
        return fuzz_raw_bytes(rng, 4096);
    }

    let value = fuzz_manifest_json(rng, case_index);
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
                object.remove(pick_manifest_field(rng));
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

fn pick_manifest_field(rng: &mut FuzzRng) -> &'static str {
    const FIELDS: &[&str] = &[
        "schema_version",
        "sdk_api_version",
        "package_id",
        "package_version",
        "minimum_host_version",
        "validation_status",
        "validation_notes",
        "runtime",
        "entrypoint",
        "operators",
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

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kyuubiki-{label}-{unique}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
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
