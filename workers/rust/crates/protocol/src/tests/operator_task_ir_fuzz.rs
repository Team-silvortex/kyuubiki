use super::prelude::*;
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

const TASK_IR_FUZZ_CASES: usize = 384;
const TASK_IR_BYTE_FUZZ_CASES: usize = 256;

#[test]
fn operator_task_ir_fuzz_smoke_does_not_panic_on_mutated_json() {
    let mut rng = FuzzRng::new(0x54_41_53_4b_49_52_4a_53);
    for case_index in 0..TASK_IR_FUZZ_CASES {
        let value = fuzz_task_ir_json(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| exercise_task_ir_boundary(&value)));
        assert!(
            outcome.is_ok(),
            "operator Task IR JSON fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn operator_task_ir_fuzz_smoke_does_not_panic_on_json_byte_ingress() {
    let mut rng = FuzzRng::new(0x54_41_53_4b_49_52_42_59);
    for case_index in 0..TASK_IR_BYTE_FUZZ_CASES {
        let bytes = fuzz_task_ir_json_bytes(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                exercise_task_ir_boundary(&value);
            }
        }));
        assert!(
            outcome.is_ok(),
            "operator Task IR byte fuzz case {case_index} panicked"
        );
    }
}

fn exercise_task_ir_boundary(value: &Value) {
    let _ = compute_operator_task_digest(value);
    let _ = verify_operator_task_digest(value);
    let _ = summarize_operator_task_execution_checked(value);
}

fn fuzz_task_ir_json(rng: &mut FuzzRng, case_index: usize) -> Value {
    if rng.one_in(4) {
        return fuzz_json(rng, 0, 7);
    }

    let mut value = task_ir_fixture(case_index);
    mutate_task_ir_json(rng, &mut value, 0);
    if rng.one_in(2) {
        refresh_task_digest(&mut value);
    }
    value
}

fn task_ir_fixture(case_index: usize) -> Value {
    let operator_id = format!("transform.fixture.{case_index}");
    let mut value = json!({
        "schema_version": OPERATOR_TASK_IR_SCHEMA,
        "task_id": format!("task-{case_index}"),
        "operator": {
            "id": operator_id,
            "family": "fixture",
            "kind": "transform",
            "execution": {
                "package_ref": "orchestra://operator-package/transform.fixture"
            }
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "task-ir-fuzz-smoke",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "x": case_index,
            "values": [1, 2, 3]
        },
        "config": {
            "alpha": true
        },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": operator_id,
            "program_family": "fixture",
            "program_kind": "transform",
            "package_ref": "orchestra://operator-package/transform.fixture",
            "package_version": "library-managed",
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": operator_id,
                "operator_kind": "transform"
            }
        },
        "dataset_contract": null,
        "orchestration_context": {
            "orch_id": "fixture-orch"
        },
        "runtime_hints": {
            "operator_kind": "transform",
            "package_ref": "orchestra://operator-package/transform.fixture",
            "package_version": "library-managed",
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "cache_scope": "job",
            "agent_fetchable": true
        },
        "integrity": {}
    });
    refresh_task_digest(&mut value);
    value
}

fn refresh_task_digest(value: &mut Value) {
    let Ok(digest) = compute_operator_task_digest(value) else {
        return;
    };
    if !value.get("integrity").is_some_and(Value::is_object) {
        value["integrity"] = json!({});
    }
    value["integrity"]["task_digest"] = Value::String(digest);
}

fn fuzz_task_ir_json_bytes(rng: &mut FuzzRng, case_index: usize) -> Vec<u8> {
    if rng.one_in(4) {
        return fuzz_raw_bytes(rng, 4096);
    }

    let value = fuzz_task_ir_json(rng, case_index);
    let mut bytes = serde_json::to_vec(&value).expect("fuzz JSON value should serialize");
    mutate_bytes(rng, &mut bytes);
    bytes
}

fn mutate_task_ir_json(rng: &mut FuzzRng, value: &mut Value, depth: usize) {
    if depth > 5 {
        return;
    }

    match value {
        Value::Object(object) => {
            if rng.one_in(6) {
                object.remove(pick_task_ir_field(rng));
            }
            if rng.one_in(5) {
                object.insert(fuzz_text(rng, 96), fuzz_json(rng, 0, 4));
            }
            for item in object.values_mut() {
                if rng.bool() {
                    mutate_task_ir_json(rng, item, depth + 1);
                }
            }
        }
        Value::Array(items) => {
            if rng.one_in(5) {
                items.push(fuzz_json(rng, 0, 4));
            }
            for item in items {
                if rng.bool() {
                    mutate_task_ir_json(rng, item, depth + 1);
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

fn pick_task_ir_field(rng: &mut FuzzRng) -> &'static str {
    const FIELDS: &[&str] = &[
        "schema_version",
        "task_id",
        "operator",
        "execution_program",
        "runtime_hints",
        "integrity",
        "config",
        "input_artifact",
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
