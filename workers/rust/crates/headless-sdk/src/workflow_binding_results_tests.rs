use super::*;
use crate::workflow_bindings::resolve_step_payload;
use crate::{
    HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome,
    HeadlessRisk, execute_batch_with_executor,
};
use serde_json::json;

pub(super) fn batch(payloads: Vec<Value>) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-07T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "binding-lifetime".into(),
        template_id: None,
        warnings: vec![],
        steps: payloads
            .into_iter()
            .enumerate()
            .map(|(offset, payload)| HeadlessExecutionBatchStep {
                index: offset + 1,
                action: "service_health".into(),
                risk: HeadlessRisk::Normal,
                payload,
            })
            .collect(),
    }
}

fn string_pointer(value: &Value) -> *const u8 {
    value.as_str().unwrap().as_ptr()
}

fn reference_resolve(value: &Value, results: &HashMap<usize, Value>) -> Result<Value, String> {
    match value {
        Value::String(text) => match parse_binding(text) {
            Some((step, output)) => results
                .get(&step)
                .and_then(Value::as_object)
                .and_then(|fields| fields.get(output))
                .cloned()
                .ok_or_else(|| format!("binding source step {step} has no output {output}")),
            None => Ok(value.clone()),
        },
        Value::Array(items) => items
            .iter()
            .map(|item| reference_resolve(item, results))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| Ok((key.clone(), reference_resolve(value, results)?)))
            .collect::<Result<_, String>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
}

#[test]
fn unreferenced_results_and_unused_raw_mirrors_are_not_retained() {
    let document = batch(vec![json!({}), json!({"id":"{{steps.1.result.status}}"})]);
    let mut store = BindingResults::new(&document);
    store.insert(1, json!({"status":"ok","result":{"nodes":(0..4096).collect::<Vec<_>>()},"raw":{"unused":"x".repeat(100_000)}}));
    assert_eq!(
        store.values[&1]
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["status"]
    );
    assert_eq!(store.take(1, "status").unwrap(), "ok");
    assert!(store.values.is_empty());
    assert!(store.remaining.is_empty());
    store.insert(2, json!({"result":[1,2,3]}));
    assert!(store.values.is_empty());
}

#[test]
fn last_string_use_moves_the_original_allocation_into_the_payload() {
    let document = batch(vec![
        json!({}),
        json!({"data":"{{steps.1.result.solver_endpoints}}"}),
    ]);
    let mut store = BindingResults::new(&document);
    let result = json!({"solver_endpoints":"x".repeat(1024 * 1024)});
    let pointer = string_pointer(&result["solver_endpoints"]);
    store.insert(1, result);
    let resolved = resolve_step_payload(&document.steps[1], &mut store).unwrap();
    assert_eq!(string_pointer(&resolved["data"]), pointer);
    assert_eq!(resolved["data"].as_str().unwrap().len(), 1024 * 1024);
    assert!(store.values.is_empty());
    assert!(store.remaining.is_empty());
}

#[test]
fn fanout_clones_nonfinal_uses_but_moves_the_last_array() {
    let document = batch(vec![
        json!({}),
        json!({"data":"{{steps.1.result.solver_endpoints}}"}),
        json!({"data":"{{steps.01.result.solver_endpoints}}"}),
    ]);
    let mut store = BindingResults::new(&document);
    let result = json!({"solver_endpoints":(0..4096).collect::<Vec<_>>()});
    let pointer = result["solver_endpoints"].as_array().unwrap().as_ptr();
    store.insert(1, result);
    let mut first = resolve_step_payload(&document.steps[1], &mut store)
        .unwrap()
        .into_owned();
    assert_ne!(first["data"].as_array().unwrap().as_ptr(), pointer);
    first["data"][0] = json!(9999);
    assert_eq!(store.values[&1]["solver_endpoints"][0], 0);
    let last = resolve_step_payload(&document.steps[2], &mut store).unwrap();
    assert_eq!(last["data"].as_array().unwrap().as_ptr(), pointer);
    assert_eq!(last["data"][0], 0);
    assert!(store.values.is_empty());
}

#[test]
fn repeated_nested_bindings_in_one_payload_count_every_occurrence() {
    let document = batch(vec![
        json!({}),
        json!({"a":" {{ steps.1.result.status }} ", "b":["{{steps.1.result.status}}",{"c":"{{steps.1.result.status}}"}]}),
    ]);
    let mut store = BindingResults::new(&document);
    store.insert(1, json!({"status":{"value":17}}));
    assert_eq!(store.remaining[&1]["status"], 3);
    let payload = resolve_step_payload(&document.steps[1], &mut store).unwrap();
    assert_eq!(payload["a"], payload["b"][0]);
    assert_eq!(payload["a"], payload["b"][1]["c"]);
    assert!(store.values.is_empty());
    assert!(store.remaining.is_empty());
}

#[test]
fn output_lifetimes_are_independent_even_in_the_same_source_result() {
    let document = batch(vec![
        json!({}),
        json!({"data":"{{steps.1.result.solver_endpoints}}"}),
        json!({"state":"{{steps.1.result.status}}"}),
    ]);
    let mut store = BindingResults::new(&document);
    store.insert(
        1,
        json!({"status":"ok","solver_endpoints":(0..4096).collect::<Vec<_>>()}),
    );
    let first = resolve_step_payload(&document.steps[1], &mut store).unwrap();
    assert_eq!(first["data"].as_array().unwrap().len(), 4096);
    assert_eq!(store.values[&1].len(), 1);
    assert!(!store.values[&1].contains_key("solver_endpoints"));
    assert_eq!(store.take(1, "status").unwrap(), "ok");
    assert!(store.values.is_empty());
}

#[test]
fn optional_null_is_a_present_output_and_missing_values_still_fail() {
    let document = batch(vec![
        json!({}),
        json!({"data":"{{steps.1.result.service}}"}),
    ]);
    for result in [
        json!({"service":null}),
        json!({}),
        json!(null),
        json!([]),
        json!("bad"),
    ] {
        let expected = reference_resolve(
            &document.steps[1].payload,
            &HashMap::from([(1, result.clone())]),
        );
        let mut store = BindingResults::new(&document);
        store.insert(1, result);
        let actual =
            resolve_step_payload(&document.steps[1], &mut store).map(|value| value.into_owned());
        assert_eq!(actual, expected);
    }
}

#[test]
fn inserted_result_templates_remain_opaque_and_do_not_extend_the_plan() {
    let document = batch(vec![
        json!({}),
        json!({"data":"{{steps.1.result.solver_endpoints}}"}),
    ]);
    let mut store = BindingResults::new(&document);
    let value =
        json!({"literal":"{{steps.999.result.secret}}","array":["{{steps.1.result.status}}"]});
    store.insert(1, json!({"solver_endpoints":value}));
    let payload = resolve_step_payload(&document.steps[1], &mut store).unwrap();
    assert_eq!(payload["data"], value);
    assert!(store.remaining.is_empty());
}

#[test]
fn literal_interpolation_and_object_keys_do_not_create_hidden_dependencies() {
    let document = batch(vec![
        json!({}),
        json!({"{{steps.1.result.status}}":"plain", "text":"prefix {{steps.1.result.status}} suffix"}),
    ]);
    let mut store = BindingResults::new(&document);
    assert!(store.remaining.is_empty());
    store.insert(1, json!({"status":"unused"}));
    let resolved = resolve_step_payload(&document.steps[1], &mut store).unwrap();
    assert_eq!(*resolved, document.steps[1].payload);
    assert!(store.values.is_empty());
}

#[test]
fn preview_retention_copies_only_referenced_outputs_and_preserves_report_values() {
    let document = batch(vec![json!({}), json!({"data":"{{steps.1.result.status}}"})]);
    let mut store = BindingResults::new(&document);
    let preview = json!({"status":"x".repeat(4096),"raw":{"nodes":(0..4096).collect::<Vec<_>>()}});
    let pointer = string_pointer(&preview["status"]);
    store.insert_preview(1, &preview);
    assert_eq!(store.values[&1].len(), 1);
    assert_ne!(string_pointer(&store.values[&1]["status"]), pointer);
    assert_eq!(store.take(1, "status").unwrap(), preview["status"]);
    assert_eq!(string_pointer(&preview["status"]), pointer);
    assert_eq!(preview["raw"]["nodes"].as_array().unwrap().len(), 4096);
}

#[test]
fn twenty_layer_chain_keeps_only_the_live_frontier_without_array_copies() {
    let mut payloads = vec![json!({})];
    for index in 1..=20 {
        payloads.push(json!({"data":format!("{{{{steps.{index}.result.solver_endpoints}}}}")}));
    }
    let document = batch(payloads);
    let mut store = BindingResults::new(&document);
    let mut data = json!((0..4096).collect::<Vec<_>>());
    let pointer = data.as_array().unwrap().as_ptr();
    for step in &document.steps {
        if step.index > 1 {
            let mut payload = resolve_step_payload(step, &mut store).unwrap().into_owned();
            data = payload.as_object_mut().unwrap().remove("data").unwrap();
            assert_eq!(data.as_array().unwrap().as_ptr(), pointer);
            assert!(store.values.is_empty());
        }
        store.insert(
            step.index,
            Value::Object(Map::from_iter([("solver_endpoints".into(), data)])),
        );
        assert!(store.values.len() <= 1);
        data = Value::Null;
    }
    assert!(store.values.is_empty());
    assert!(store.remaining.is_empty());
}

#[test]
fn graph_matrix_matches_the_previous_whole_value_resolver() {
    for depth in [2, 8, 20] {
        for fanout in [1, 2, 4] {
            let mut payloads = vec![json!({})];
            for index in 1..depth {
                let values = (0..fanout).map(|offset| {
                    let source = 1 + (index + offset) % index;
                    json!({"array":[format!("{{{{steps.{source}.result.solver_endpoints}}}}")],
                        "optional":format!("{{{{steps.{source}.result.service}}}}"), "literal":"a {{steps.1.result.status}} b"})
                }).collect::<Vec<_>>();
                payloads.push(json!({"values":values}));
            }
            let document = batch(payloads);
            let mut store = BindingResults::new(&document);
            let mut reference = HashMap::new();
            for step in &document.steps {
                assert_eq!(
                    resolve_step_payload(step, &mut store).unwrap().into_owned(),
                    reference_resolve(&step.payload, &reference).unwrap()
                );
                let result = json!({"solver_endpoints":{"nodes":(0..256).collect::<Vec<_>>(), "literal":"{{steps.999.result.status}}"}, "service":null, "unreferenced":"x".repeat(1000)});
                reference.insert(step.index, result.clone());
                store.insert(step.index, result);
            }
            assert!(store.values.is_empty());
            assert!(store.remaining.is_empty());
        }
    }
}

struct MoveProbe {
    first: Option<Value>,
    received: Vec<(usize, usize)>,
}

impl HeadlessExecutor for MoveProbe {
    fn name(&self) -> &'static str {
        "service"
    }
    fn execute_step(
        &mut self,
        _action: &str,
        index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        if index > 1 {
            let value = payload["data"].as_array().unwrap();
            self.received.push((value.as_ptr() as usize, value.len()));
        }
        Ok(HeadlessExecutorOutcome {
            status: "executed".into(),
            result: self.first.take().unwrap_or_else(|| json!({"status":"ok"})),
        })
    }
}

#[test]
fn public_batch_dispatch_moves_last_use_but_keeps_compacted_reports_unchanged() {
    let document = batch(vec![
        json!({}),
        json!({"data":"{{steps.1.result.solver_endpoints}}"}),
    ]);
    let source = json!({"solver_endpoints":(0..4096).collect::<Vec<_>>(), "raw":{"values":(0..4096).collect::<Vec<_>>()}});
    let pointer = source["solver_endpoints"].as_array().unwrap().as_ptr() as usize;
    let preview = crate::run::compact_report_value(&source);
    let mut executor = MoveProbe {
        first: Some(source),
        received: vec![],
    };
    let report = execute_batch_with_executor(&document, &mut executor, false, false);
    assert_eq!(report.status, "ok");
    assert_eq!(report.executed_step_count, 2);
    assert_eq!(executor.received, [(pointer, 4096)]);
    assert_eq!(report.steps[0].result_preview, preview);
    assert_eq!(report.steps[1].payload["data"]["item_count"], 4096);
}
