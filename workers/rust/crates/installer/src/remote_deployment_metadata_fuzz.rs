use super::RemoteDeploymentDryRunReport;
use crate::{RemoteDeploymentJournal, RemoteDeploymentPlan};
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

const REMOTE_DEPLOYMENT_JSON_FUZZ_CASES: usize = 384;
const REMOTE_DEPLOYMENT_BYTE_FUZZ_CASES: usize = 256;

#[test]
fn remote_deployment_metadata_fuzz_smoke_does_not_panic_on_mutated_json() {
    let mut rng = FuzzRng::new(0x52_44_50_4c_4a_53_4f_4e);
    for case_index in 0..REMOTE_DEPLOYMENT_JSON_FUZZ_CASES {
        let value = fuzz_metadata_json(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            exercise_remote_deployment_boundary(value);
        }));
        assert!(
            outcome.is_ok(),
            "remote deployment metadata JSON fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn remote_deployment_metadata_fuzz_smoke_does_not_panic_on_byte_ingress() {
    let mut rng = FuzzRng::new(0x52_44_50_4c_42_59_54_45);
    for case_index in 0..REMOTE_DEPLOYMENT_BYTE_FUZZ_CASES {
        let bytes = fuzz_metadata_bytes(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                exercise_remote_deployment_boundary(value);
            }
        }));
        assert!(
            outcome.is_ok(),
            "remote deployment metadata byte fuzz case {case_index} panicked"
        );
    }
}

fn exercise_remote_deployment_boundary(value: Value) {
    if let Ok(plan) = serde_json::from_value::<RemoteDeploymentPlan>(value.clone()) {
        let _ = serde_json::to_value(&plan);
        let _ = serde_json::to_string(&plan);
        let _ = plan.render();
    }
    if let Ok(journal) = serde_json::from_value::<RemoteDeploymentJournal>(value.clone()) {
        let _ = serde_json::to_value(&journal);
        let _ = serde_json::to_string(&journal);
        let _ = journal.render();
    }
    if let Ok(report) = serde_json::from_value::<RemoteDeploymentDryRunReport>(value) {
        let _ = serde_json::to_value(&report);
        let _ = serde_json::to_string(&report);
        let _ = report.render();
    }
}

fn fuzz_metadata_json(rng: &mut FuzzRng, case_index: usize) -> Value {
    if rng.one_in(4) {
        return fuzz_json(rng, 0, 7);
    }

    let mut value = match rng.usize(3) {
        0 => plan_fixture(case_index),
        1 => journal_fixture(case_index),
        _ => dry_run_fixture(case_index),
    };
    mutate_json(rng, &mut value, 0);
    value
}

fn plan_fixture(case_index: usize) -> Value {
    json!({
        "schema_version": "kyuubiki.remote-deployment-plan/v1",
        "plan_id": format!("remote-agent-lab-pilot-{case_index}"),
        "target_profile": "policy-bounded SSH remote agent",
        "steps": [
            step_fixture("policy-check", "preflight", "remote.policy"),
            step_fixture("sync-artifacts", "delivery", "remote.artifacts"),
            step_fixture("health-check", "verify", "remote.health")
        ]
    })
}

fn step_fixture(id: &str, phase: &str, idempotency_key: &str) -> Value {
    json!({
        "id": id,
        "title": format!("Step {id}"),
        "phase": phase,
        "idempotency_key": idempotency_key,
        "rollback_hint": "stop before remote mutation when possible",
        "failure_class": phase
    })
}

fn journal_fixture(case_index: usize) -> Value {
    json!({
        "schema_version": "kyuubiki.remote-deployment-journal/v1",
        "plan_id": format!("remote-agent-lab-pilot-{case_index}"),
        "target_ref": format!("lab-agent-{case_index}"),
        "records": [
            record_fixture("policy-check", "preflight", "pending"),
            record_fixture("sync-artifacts", "delivery", "pending")
        ]
    })
}

fn record_fixture(step_id: &str, phase: &str, status: &str) -> Value {
    json!({
        "step_id": step_id,
        "phase": phase,
        "status": status,
        "idempotency_key": format!("lab-agent:{step_id}"),
        "failure_class": phase,
        "local_record_path": format!(".kyuubiki/remote-journal/plan/{step_id}.jsonl"),
        "remote_record_path": format!(".kyuubiki/remote-journal/lab/{step_id}.jsonl")
    })
}

fn dry_run_fixture(case_index: usize) -> Value {
    json!({
        "schema_version": "kyuubiki.remote-deployment-dry-run/v1",
        "status": "blocked",
        "plan": plan_fixture(case_index),
        "journal": journal_fixture(case_index),
        "artifact_manifest": null,
        "blockers": ["fixture only"],
        "warnings": ["does not open SSH"],
        "next_actions": ["review metadata"]
    })
}

fn fuzz_metadata_bytes(rng: &mut FuzzRng, case_index: usize) -> Vec<u8> {
    if rng.one_in(4) {
        return fuzz_raw_bytes(rng, 4096);
    }

    let value = fuzz_metadata_json(rng, case_index);
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
                object.remove(pick_metadata_field(rng));
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

fn pick_metadata_field(rng: &mut FuzzRng) -> &'static str {
    const FIELDS: &[&str] = &[
        "schema_version",
        "status",
        "plan",
        "journal",
        "artifact_manifest",
        "blockers",
        "warnings",
        "next_actions",
        "plan_id",
        "target_profile",
        "steps",
        "target_ref",
        "records",
        "step_id",
        "phase",
        "idempotency_key",
        "failure_class",
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
