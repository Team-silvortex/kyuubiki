use super::{RemoteSshFixturePlan, RemoteSshFixtureReport, contains_secret_token};
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

const REMOTE_SSH_FIXTURE_JSON_FUZZ_CASES: usize = 384;
const REMOTE_SSH_FIXTURE_BYTE_FUZZ_CASES: usize = 256;

#[test]
fn remote_ssh_fixture_fuzz_smoke_does_not_panic_on_mutated_json() {
    let mut rng = FuzzRng::new(0x53_53_48_46_4a_53_4f_4e);
    for case_index in 0..REMOTE_SSH_FIXTURE_JSON_FUZZ_CASES {
        let value = fuzz_fixture_json(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            exercise_fixture_boundary(value);
        }));
        assert!(
            outcome.is_ok(),
            "remote SSH fixture JSON fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn remote_ssh_fixture_fuzz_smoke_does_not_panic_on_byte_ingress() {
    let mut rng = FuzzRng::new(0x53_53_48_46_42_59_54_45);
    for case_index in 0..REMOTE_SSH_FIXTURE_BYTE_FUZZ_CASES {
        let bytes = fuzz_fixture_bytes(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                exercise_fixture_boundary(value);
            }
        }));
        assert!(
            outcome.is_ok(),
            "remote SSH fixture byte fuzz case {case_index} panicked"
        );
    }
}

fn exercise_fixture_boundary(value: Value) {
    if let Ok(report) = serde_json::from_value::<RemoteSshFixtureReport>(value.clone()) {
        let _ = serde_json::to_value(&report);
        let _ = serde_json::to_string(&report);
        let rendered = report.render();
        let _ = contains_secret_token(&rendered);
    }
    if let Ok(plan) = serde_json::from_value::<RemoteSshFixturePlan>(value) {
        let _ = serde_json::to_value(&plan);
        let _ = serde_json::to_string(&plan);
        let rendered = plan.render();
        let _ = contains_secret_token(&rendered);
    }
}

fn fuzz_fixture_json(rng: &mut FuzzRng, case_index: usize) -> Value {
    if rng.one_in(4) {
        return fuzz_json(rng, 0, 7);
    }

    let mut value = if rng.bool() {
        report_fixture(case_index)
    } else {
        plan_fixture(case_index)
    };
    mutate_json(rng, &mut value, 0);
    value
}

fn report_fixture(case_index: usize) -> Value {
    json!({
        "schema_version": "kyuubiki.remote-ssh-fixture/v1",
        "fixture_target": format!("kyuubiki-fixture@fixture-{case_index}.local"),
        "commands": [
            {
                "id": "probe",
                "program": "ssh",
                "args": [
                    "-o",
                    "StrictHostKeyChecking=accept-new",
                    "-p",
                    "2222",
                    format!("kyuubiki-fixture@fixture-{case_index}.local"),
                    "printf kyuubiki-remote-ok"
                ],
                "mutates_remote": false
            }
        ],
        "checks": [
            {
                "label": "no-network-opened",
                "ok": true,
                "detail": "fixture report is pure metadata"
            }
        ]
    })
}

fn plan_fixture(case_index: usize) -> Value {
    json!({
        "schema_version": "kyuubiki.remote-ssh-fixture-plan/v1",
        "compose_file": "tests/integration/remote-ssh-fixture/compose.yaml",
        "dockerfile": "tests/integration/remote-ssh-fixture/Dockerfile",
        "run_script": "scripts/run-remote-ssh-fixture.sh",
        "runtime_dir": "tests/integration/remote-ssh-fixture/runtime",
        "bind_address": "127.0.0.1:2222",
        "ssh_user": "kyuubiki-fixture",
        "remote_workspace": format!("/tmp/kyuubiki-fixture-{case_index}"),
        "manual_only": true,
        "ignored_runtime_paths": [
            "tests/integration/remote-ssh-fixture/runtime/client_key",
            "tests/integration/remote-ssh-fixture/runtime/known_hosts"
        ]
    })
}

fn fuzz_fixture_bytes(rng: &mut FuzzRng, case_index: usize) -> Vec<u8> {
    if rng.one_in(4) {
        return fuzz_raw_bytes(rng, 4096);
    }

    let value = fuzz_fixture_json(rng, case_index);
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
                object.remove(pick_fixture_field(rng));
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

fn pick_fixture_field(rng: &mut FuzzRng) -> &'static str {
    const FIELDS: &[&str] = &[
        "schema_version",
        "fixture_target",
        "commands",
        "checks",
        "compose_file",
        "dockerfile",
        "run_script",
        "runtime_dir",
        "bind_address",
        "ssh_user",
        "remote_workspace",
        "manual_only",
        "ignored_runtime_paths",
        "id",
        "program",
        "args",
        "mutates_remote",
        "label",
        "ok",
        "detail",
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
