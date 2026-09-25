use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNodeRunStatus};
use serde_json::{Value, json};

const EXTRACTORS: [&str; 2] = ["extract.field_statistics", "extract.field_hotspots"];

fn payload() -> Value {
    json!({"nodes":[{"id":"n0","v":1.0},{"id":"n1","v":2.0},{"id":"n2","v":3.0}]})
}

fn request(extractor: &str, input: Value, config: Value) -> Value {
    let artifact = "artifact/result_summary";
    let port = |id| json!({"id":id,"artifact_type":artifact});
    let edge = |from: &str, output: &str, to: &str, input: &str| {
        json!({
        "id":format!("{from}-{to}"),"artifact_type":artifact,
        "from":{"node":from,"port":output},"to":{"node":to,"port":input}})
    };
    let metric = if extractor == EXTRACTORS[0] {
        "v_max"
    } else {
        "v_hotspot_max"
    };
    json!({"graph":{
        "schema_version":"kyuubiki.workflow-graph/v1","id":"field-bundle-integrity",
        "name":"Checked fields to diagnostic report","version":"1.0.0",
        "entry_nodes":["input","extra","independent"],
        "output_nodes":["raw","raw_extra","decision","independent_output"],
        "nodes":[
            {"id":"input","kind":"input","inputs":[],"outputs":[port("payload")]},
            {"id":"extra","kind":"input","inputs":[],"outputs":[port("payload")]},
            {"id":"field","kind":"extract","operator_id":extractor,"config":config,
                "inputs":[port("payload")],"outputs":[port("summary")]},
            {"id":"bundle","kind":"transform","operator_id":"transform.compose_diagnostics_bundle",
                "config":{"include_non_diagnostics":true},"inputs":[port("field"),port("extra")],"outputs":[port("summary")]},
            {"id":"guard","kind":"transform","operator_id":"transform.evaluate_diagnostics_bundle_guard",
                "config":{"rules":[{"source":"field","field":metric,"comparison":"gt","threshold":100.0,"severity":"block"}]},
                "inputs":[port("bundle")],"outputs":[port("summary")]},
            {"id":"report","kind":"transform","operator_id":"transform.compose_diagnostics_report_payload",
                "inputs":[port("bundle"),port("guard")],"outputs":[port("summary")]},
            {"id":"export","kind":"export","operator_id":"export.summary_json",
                "inputs":[port("payload")],"outputs":[port("summary")]},
            {"id":"decision","kind":"output","inputs":[port("summary")],"outputs":[]},
            {"id":"raw","kind":"output","inputs":[port("payload")],"outputs":[]},
            {"id":"raw_extra","kind":"output","inputs":[port("payload")],"outputs":[]},
            {"id":"independent","kind":"input","inputs":[],"outputs":[port("value")]},
            {"id":"independent_output","kind":"output","inputs":[port("value")],"outputs":[]}
        ],
        "edges":[edge("input","payload","field","payload"),edge("input","payload","raw","payload"),
            edge("field","summary","bundle","field"),edge("extra","payload","bundle","extra"),
            edge("extra","payload","raw_extra","payload"),edge("bundle","summary","guard","bundle"),
            edge("bundle","summary","report","bundle"),edge("guard","summary","report","guard"),
            edge("report","summary","export","payload"),edge("export","summary","decision","summary"),
            edge("independent","value","independent_output","value")]
    },"input_artifacts":{"input":input,"extra":{
        "diagnostic_contract":"kyuubiki.workflow_diagnostics/v1","diagnostic_domain":"thermal",
        "diagnostic_node_count":2,"diagnostic_element_count":1,"thermal_temperature_max":10.0
    },"independent":{"value":7}}})
}

fn base(extractor: &str) -> Value {
    request(
        extractor,
        payload(),
        json!({"source":"nodes","field":"v","output_prefix":"v","percentiles":[50,90],"percentile":50}),
    )
}

fn node<'a>(request: &'a mut Value, name: &str) -> &'a mut Value {
    request["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == name)
        .unwrap()
}

fn run(mut request: Value, recover: bool) -> Result<WorkflowGraphRunResult, String> {
    if recover {
        for id in ["field", "bundle", "guard"] {
            node(&mut request, id)["config"]["on_error"] = json!("skip");
        }
    }
    let request: WorkflowGraphRunRequest = serde_json::from_value(request).unwrap();
    run_workflow_graph(request)
}

fn failures(extractor: &str) -> Vec<(Value, &'static str, &'static str)> {
    let mut cases = Vec::new();
    let mut req = base(extractor);
    req["input_artifacts"]["input"]["nodes"][1]["v"] = Value::Null;
    cases.push((req, "field", "payload.nodes[1].v"));
    let mut req = base(extractor);
    req["input_artifacts"]["input"]["converged"] = json!(false);
    cases.push((req, "field", "payload.converged"));
    let mut req = base(extractor);
    node(&mut req, "field")["config"]["field"] = json!("missing");
    cases.push((req, "field", "payload.nodes[0].missing"));
    let mut req = base(extractor);
    req["input_artifacts"]["extra"]["converged"] = json!(false);
    cases.push((req, "bundle", "payload.extra.converged"));
    let mut req = base(extractor);
    req["input_artifacts"]["extra"]["diagnostic_node_count"] = json!(-1);
    cases.push((req, "bundle", "payload.extra.diagnostic_node_count"));
    let mut req = base(extractor);
    node(&mut req, "guard")["config"]["rules"][0]["field"] = json!("missing");
    cases.push((req, "guard", "payload.bundle_payloads.field.missing"));
    let mut req = base(extractor);
    node(&mut req, "guard")["config"]["rules"][0]["severity"] = json!("fatal");
    cases.push((req, "guard", "config.rules[0].severity"));
    let mut req = base(extractor);
    node(&mut req, "bundle")["config"]["include_payloads"] = json!(false);
    cases.push((req, "guard", "bundle_payloads"));
    cases
}

fn exported(run: &WorkflowGraphRunResult) -> Value {
    assert!(run.failed_nodes.is_empty());
    assert!(run.skipped_nodes.is_empty());
    serde_json::from_str(
        run.artifacts["decision.summary"]["content"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn field_bundle_chains_fail_before_exporting_invalid_evidence() {
    for extractor in EXTRACTORS {
        for (req, failed, path) in failures(extractor) {
            let error = run(req, false).unwrap_err();
            assert!(
                error.contains(&format!("workflow node {failed} failed")),
                "{error}"
            );
            assert!(error.contains(path), "{error}");
            assert!(!error.contains("panicked"), "{error}");
        }
    }
}

#[test]
fn reporting_recovery_preserves_raw_inputs_without_a_false_pass_document() {
    for extractor in EXTRACTORS {
        for (req, failed, path) in failures(extractor) {
            let original = req["input_artifacts"].clone();
            let result = run(req, true).unwrap();
            assert_eq!(result.failed_nodes, vec![failed]);
            assert_eq!(result.artifacts["raw.payload"], original["input"]);
            assert_eq!(result.artifacts["raw_extra.payload"], original["extra"]);
            assert_eq!(
                result.artifacts["independent_output.value"],
                json!({"value":7})
            );
            for id in ["report", "export", "decision"] {
                assert!(result.skipped_nodes.iter().any(|node| node == id));
                assert!(!result.artifacts.contains_key(&format!("{id}.summary")));
            }
            let trace = result
                .node_runs
                .iter()
                .find(|node| node.node_id == failed)
                .unwrap();
            assert_eq!(trace.status, WorkflowNodeRunStatus::Failed);
            assert!(trace.error_message.as_ref().unwrap().contains(path));
        }
    }
}

#[test]
fn corrected_field_and_bundle_evidence_replays_to_a_complete_report() {
    for extractor in EXTRACTORS {
        for (req, _, _) in failures(extractor) {
            assert_eq!(run(req, true).unwrap().failed_nodes.len(), 1);
            let report = exported(&run(base(extractor), false).unwrap());
            assert_eq!(report["report_guard_status"], "pass");
            assert_eq!(report["guard_payload"]["guard_checked_rule_count"], 1);
            assert_eq!(report["bundle_source_count"], 2);
            assert_eq!(report["bundle_total_node_count"], Value::Null);
        }
    }
}

#[test]
fn a_real_guard_violation_remains_a_blocked_report_not_an_execution_error() {
    for extractor in EXTRACTORS {
        let mut req = base(extractor);
        node(&mut req, "guard")["config"]["rules"][0]["threshold"] = json!(2.0);
        let report = exported(&run(req, false).unwrap());
        assert_eq!(report["report_guard_status"], "block");
        assert_eq!(report["guard_payload"]["guard_passed"], false);
        assert_eq!(report["guard_payload"]["guard_trigger_count"], 1);
    }
}

#[test]
fn real_heat_and_electrostatic_results_complete_statistics_and_hotspot_report_chains() {
    for heat in [true, false] {
        let (operator, field, fix, source, material) = if heat {
            (
                "solve.heat_plane_quad_2d",
                "temperature",
                "fix_temperature",
                "heat_load",
                "conductivity",
            )
        } else {
            (
                "solve.electrostatic_plane_quad_2d",
                "potential",
                "fix_potential",
                "charge_density",
                "permittivity",
            )
        };
        let nodes = [(0.0,0.0),(1.0,0.0),(1.0,1.0),(0.0,1.0)].into_iter().enumerate().map(|(i,(x,y))|
            json!({"id":format!("n{i}"),"x":x,"y":y,field:10.0*(i+1) as f64,fix:true,source:0.0})).collect::<Vec<_>>();
        let raw = run_solve_operator(
            operator,
            json!({"nodes":nodes,"elements":[{
            "id":"e0","node_i":0,"node_j":1,"node_k":2,"node_l":3,"thickness":0.1,material:1.0}]}),
        )
        .unwrap();
        for extractor in EXTRACTORS {
            let config = json!({"source":"nodes","field":field,"output_prefix":"v","percentiles":[50,90],"percentile":50});
            let req = request(extractor, raw.clone(), config.clone());
            let result = run(req, false).unwrap();
            let report = exported(&result);
            assert_eq!(report["report_guard_status"], "pass");
            let metrics = &result.artifacts["field.summary"];
            if extractor == EXTRACTORS[0] {
                assert_eq!(metrics["v_count"], 4);
                assert_eq!(metrics["v_mean"], 25.0);
                assert_eq!(metrics["v_p50"], 25.0);
                assert_eq!(metrics["v_p90"], 37.0);
            } else {
                assert_eq!(metrics["v_threshold"], 25.0);
                assert_eq!(metrics["v_hotspot_count"], 2);
                assert_eq!(metrics["v_hotspot_fraction"], 0.5);
                assert_eq!(metrics["v_hotspot_ids"], json!(["n3", "n2"]));
            }
            let mut corrupt = raw.clone();
            corrupt["nodes"][3][field] = Value::Null;
            assert_eq!(
                run(request(extractor, corrupt, config.clone()), true)
                    .unwrap()
                    .failed_nodes,
                vec!["field"]
            );
            assert_eq!(
                exported(&run(request(extractor, raw.clone(), config), false).unwrap())["report_guard_status"],
                "pass"
            );
        }
    }
}
