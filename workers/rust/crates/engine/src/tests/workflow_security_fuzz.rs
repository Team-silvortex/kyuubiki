use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

const FUZZ_CASES: usize = 384;
const JSON_DECODE_FUZZ_CASES: usize = 256;
const JSON_BYTE_DECODE_FUZZ_CASES: usize = 256;

#[test]
fn workflow_security_fuzz_smoke_does_not_panic_on_mutated_graphs() {
    let mut rng = FuzzRng::new(0x4b_59_55_55_42_49_4b_49);
    for case_index in 0..FUZZ_CASES {
        let request = fuzz_request(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| run_workflow_graph(request)));
        assert!(
            outcome.is_ok(),
            "workflow security fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn workflow_security_fuzz_smoke_does_not_panic_on_json_decode_boundary() {
    let mut rng = FuzzRng::new(0x57_4f_52_4b_46_4c_4f_57);
    for case_index in 0..JSON_DECODE_FUZZ_CASES {
        let value = fuzz_request_json(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(request) = serde_json::from_value::<WorkflowGraphRunRequest>(value) {
                let _ = run_workflow_graph(request);
            }
        }));
        assert!(
            outcome.is_ok(),
            "workflow JSON decode fuzz case {case_index} panicked"
        );
    }
}

#[test]
fn workflow_security_fuzz_smoke_does_not_panic_on_json_byte_decode_boundary() {
    let mut rng = FuzzRng::new(0x4a_53_4f_4e_42_59_54_45);
    for case_index in 0..JSON_BYTE_DECODE_FUZZ_CASES {
        let bytes = fuzz_request_json_bytes(&mut rng, case_index);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(request) = serde_json::from_slice::<WorkflowGraphRunRequest>(&bytes) {
                let _ = run_workflow_graph(request);
            }
        }));
        assert!(
            outcome.is_ok(),
            "workflow JSON byte decode fuzz case {case_index} panicked"
        );
    }
}

fn fuzz_request(rng: &mut FuzzRng, case_index: usize) -> WorkflowGraphRunRequest {
    let node_count = 1 + rng.usize(18);
    let nodes = (0..node_count)
        .map(|index| fuzz_node(rng, index, case_index))
        .collect::<Vec<_>>();
    let edges = fuzz_edges(rng, &nodes);
    let entry_node_count = rng.usize(4);
    let output_node_count = rng.usize(4);
    let graph_id_ascii_safe = rng.bool();
    let description_len = 256 + rng.usize(1024);
    let entry_nodes = fuzz_node_refs(rng, &nodes, entry_node_count);
    let output_nodes = fuzz_node_refs(rng, &nodes, output_node_count);
    let input_artifacts = fuzz_input_artifacts(rng, &nodes);

    WorkflowGraphRunRequest {
        graph: WorkflowGraph {
            schema_version: if rng.bool() {
                "kyuubiki.workflow-graph/v1".to_string()
            } else {
                fuzz_text(rng, 48, true)
            },
            id: fuzz_text(rng, 96, graph_id_ascii_safe),
            name: fuzz_text(rng, 64, true),
            version: fuzz_text(rng, 24, true),
            description: maybe_fuzz_text(rng, description_len, true),
            dataset_contract: None,
            entry_nodes,
            output_nodes,
            defaults: WorkflowDefaults::default(),
            nodes,
            edges,
        },
        input_artifacts,
    }
}

fn fuzz_node(rng: &mut FuzzRng, index: usize, case_index: usize) -> WorkflowNode {
    let kind = match rng.usize(7) {
        0 => WorkflowNodeKind::Input,
        1 => WorkflowNodeKind::Solve,
        2 => WorkflowNodeKind::Transform,
        3 => WorkflowNodeKind::Extract,
        4 => WorkflowNodeKind::Export,
        5 => WorkflowNodeKind::Condition,
        _ => WorkflowNodeKind::Output,
    };
    let operator_id = match kind {
        WorkflowNodeKind::Solve => Some(if rng.bool() {
            "solve.bar_1d".to_string()
        } else {
            fuzz_operator(rng)
        }),
        WorkflowNodeKind::Transform | WorkflowNodeKind::Extract | WorkflowNodeKind::Export => {
            Some(fuzz_operator(rng))
        }
        WorkflowNodeKind::Input | WorkflowNodeKind::Condition | WorkflowNodeKind::Output => {
            rng.bool().then(|| fuzz_operator(rng))
        }
    };
    let config = rng.bool().then(|| fuzz_json(rng, 0, 6));
    let input_port_count = rng.usize(5);
    let output_port_count = rng.usize(5);

    WorkflowNode {
        id: if rng.one_in(9) {
            format!("node.{case_index}.{index}")
        } else {
            format!("node_{case_index}_{index}_{}", rng.usize(16))
        },
        kind,
        operator_id,
        name: maybe_fuzz_text(rng, 80, true),
        description: maybe_fuzz_text(rng, 320, true),
        config,
        cache_policy: None,
        inputs: fuzz_ports(rng, "input", input_port_count),
        outputs: fuzz_ports(rng, "output", output_port_count),
    }
}

fn fuzz_edges(rng: &mut FuzzRng, nodes: &[WorkflowNode]) -> Vec<WorkflowEdge> {
    let edge_count = rng.usize(nodes.len().saturating_mul(2).max(1));
    (0..edge_count)
        .map(|index| {
            let from = pick_node(rng, nodes);
            let to = pick_node(rng, nodes);
            let from_port = pick_port_id(rng, &from.outputs);
            let to_port = pick_port_id(rng, &to.inputs);
            WorkflowEdge {
                id: if rng.one_in(7) {
                    fuzz_text_with_random_charset(rng, 150)
                } else {
                    format!("edge_{index}_{}", rng.usize(128))
                },
                from: WorkflowNodePortRef {
                    node: from.id.clone(),
                    port: from_port,
                },
                to: WorkflowNodePortRef {
                    node: to.id.clone(),
                    port: to_port,
                },
                artifact_type: fuzz_artifact_type(rng),
                dataset_value: maybe_fuzz_text_with_random_charset(rng, 96),
            }
        })
        .collect()
}

fn fuzz_input_artifacts(
    rng: &mut FuzzRng,
    nodes: &[WorkflowNode],
) -> BTreeMap<String, serde_json::Value> {
    let mut artifacts = BTreeMap::new();
    for _ in 0..rng.usize(5) {
        let node = pick_node(rng, nodes);
        artifacts.insert(node.id.clone(), fuzz_json(rng, 0, 5));
    }
    artifacts
}

fn fuzz_request_json(rng: &mut FuzzRng, case_index: usize) -> Value {
    if rng.one_in(4) {
        return fuzz_json(rng, 0, 7);
    }

    let mut value = serde_json::to_value(fuzz_request(rng, case_index))
        .expect("typed fuzz request should serialize");
    mutate_json_request(rng, &mut value, 0);
    value
}

fn fuzz_request_json_bytes(rng: &mut FuzzRng, case_index: usize) -> Vec<u8> {
    if rng.one_in(4) {
        return fuzz_raw_bytes(rng, 4096);
    }

    let value = fuzz_request_json(rng, case_index);
    let mut bytes = serde_json::to_vec(&value).expect("fuzz JSON value should serialize");
    mutate_bytes(rng, &mut bytes);
    bytes
}

fn mutate_json_request(rng: &mut FuzzRng, value: &mut Value, depth: usize) {
    if depth > 5 {
        return;
    }
    match value {
        Value::Object(object) => {
            if rng.one_in(5) {
                let key = fuzz_text_with_random_charset(rng, 80);
                object.insert(key, fuzz_json(rng, 0, 4));
            }
            if rng.one_in(7) {
                object.remove("graph");
            }
            if rng.one_in(7) {
                object.insert("input_artifacts".to_string(), fuzz_json(rng, 0, 5));
            }
            for item in object.values_mut() {
                if rng.bool() {
                    mutate_json_request(rng, item, depth + 1);
                }
            }
        }
        Value::Array(items) => {
            if rng.one_in(5) {
                items.push(fuzz_json(rng, 0, 4));
            }
            for item in items {
                if rng.bool() {
                    mutate_json_request(rng, item, depth + 1);
                }
            }
        }
        Value::String(text) => {
            if rng.one_in(6) {
                *text = fuzz_text_with_random_charset(rng, 512);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            if rng.one_in(8) {
                *value = fuzz_json(rng, 0, 3);
            }
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

fn fuzz_node_refs(rng: &mut FuzzRng, nodes: &[WorkflowNode], count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            if rng.one_in(5) {
                fuzz_text_with_random_charset(rng, 80)
            } else {
                pick_node(rng, nodes).id.clone()
            }
        })
        .collect()
}

fn fuzz_ports(rng: &mut FuzzRng, prefix: &str, count: usize) -> Vec<WorkflowPort> {
    (0..count)
        .map(|index| WorkflowPort {
            id: if rng.one_in(8) {
                fuzz_text_with_random_charset(rng, 150)
            } else {
                format!("{prefix}_{index}")
            },
            artifact_type: fuzz_artifact_type(rng),
            name: maybe_fuzz_text(rng, 80, true),
            required: rng.bool().then(|| rng.bool()),
            cardinality: maybe_fuzz_text_with_random_charset(rng, 24),
            dataset_value: maybe_fuzz_text_with_random_charset(rng, 96),
        })
        .collect()
}

fn fuzz_json(rng: &mut FuzzRng, depth: usize, max_depth: usize) -> Value {
    if depth >= max_depth {
        return Value::String(fuzz_text(rng, 64, true));
    }
    match rng.usize(6) {
        0 => Value::Null,
        1 => Value::Bool(rng.bool()),
        2 => Value::Number(serde_json::Number::from(rng.usize(10_000) as u64)),
        3 => {
            let max_len = 256 + rng.usize(2048);
            let ascii_safe = !rng.one_in(16);
            Value::String(fuzz_text(rng, max_len, ascii_safe))
        }
        4 => Value::Array(
            (0..rng.usize(6))
                .map(|_| fuzz_json(rng, depth + 1, max_depth))
                .collect(),
        ),
        _ => {
            let mut object = serde_json::Map::new();
            for index in 0..rng.usize(6) {
                object.insert(
                    if rng.one_in(12) {
                        fuzz_text_with_random_charset(rng, 320)
                    } else {
                        format!("key_{depth}_{index}")
                    },
                    fuzz_json(rng, depth + 1, max_depth),
                );
            }
            Value::Object(object)
        }
    }
}

fn pick_node<'a>(rng: &mut FuzzRng, nodes: &'a [WorkflowNode]) -> &'a WorkflowNode {
    &nodes[rng.usize(nodes.len())]
}

fn pick_port_id(rng: &mut FuzzRng, ports: &[WorkflowPort]) -> String {
    if ports.is_empty() || rng.one_in(4) {
        fuzz_text_with_random_charset(rng, 48)
    } else {
        ports[rng.usize(ports.len())].id.clone()
    }
}

fn fuzz_operator(rng: &mut FuzzRng) -> String {
    match rng.usize(5) {
        0 => "solve.bar_1d".to_string(),
        1 => "export.alert_markdown".to_string(),
        2 => "summary.normalize".to_string(),
        3 => "solve.not_real".to_string(),
        _ => fuzz_text_with_random_charset(rng, 96),
    }
}

fn fuzz_artifact_type(rng: &mut FuzzRng) -> String {
    match rng.usize(6) {
        0 => "study_model/bar_1d".to_string(),
        1 => "result/bar_1d".to_string(),
        2 => "summary/generic".to_string(),
        3 => "report/markdown".to_string(),
        4 => "bad artifact type".to_string(),
        _ => fuzz_text_with_random_charset(rng, 192),
    }
}

fn maybe_fuzz_text(rng: &mut FuzzRng, max_len: usize, ascii_safe: bool) -> Option<String> {
    rng.bool().then(|| fuzz_text(rng, max_len, ascii_safe))
}

fn maybe_fuzz_text_with_random_charset(rng: &mut FuzzRng, max_len: usize) -> Option<String> {
    rng.bool()
        .then(|| fuzz_text_with_random_charset(rng, max_len))
}

fn fuzz_text_with_random_charset(rng: &mut FuzzRng, max_len: usize) -> String {
    let ascii_safe = rng.bool();
    fuzz_text(rng, max_len, ascii_safe)
}

fn fuzz_text(rng: &mut FuzzRng, max_len: usize, ascii_safe: bool) -> String {
    let len = rng.usize(max_len.saturating_add(1));
    (0..len)
        .map(|_| {
            if ascii_safe {
                let alphabet =
                    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-./";
                alphabet[rng.usize(alphabet.len())] as char
            } else {
                match rng.usize(6) {
                    0 => '\0',
                    1 => ' ',
                    2 => ':',
                    3 => '/',
                    4 => '.',
                    _ => char::from_u32(0x4e00 + rng.usize(128) as u32).unwrap_or('x'),
                }
            }
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
