use kyuubiki_engine::run_workflow_graph_with_options;
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowGraphRunOptions, WorkflowGraphRunRequest,
    WorkflowGraphRunResult,
};
use kyuubiki_solver::solver_control::{
    SolverControl, SolverStage, with_solver_control, with_solver_observer,
};
use serde_json::{Value, json};

fn request(skip: bool, last: bool, missing: bool) -> WorkflowGraphRunRequest {
    let port = |id| json!({"id":id,"artifact_type":"artifact/json"});
    let edge = |from, to| {
        json!({"id":format!("{from}-{to}"),
        "from":{"node":from,"port":"payload"},"to":{"node":to,"port":"payload"},
        "artifact_type":"artifact/json"})
    };
    let mut value = json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"workflow.cancel-boundaries",
            "name":"Cancellation boundaries","version":"1.0.0",
            "entry_nodes":["input","independent"],"output_nodes":["raw","output","independent_output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"diagnose","kind":"extract","operator_id":"extract.stokes_flow_result_diagnostics",
                 "config":{"on_error":if skip {"skip"} else {"fail"}},
                 "inputs":[port("payload")],"outputs":[port("payload")]},
                {"id":"independent","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"raw","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"pass","kind":"transform","operator_id":"transform.first_available",
                 "inputs":[port("payload")],"outputs":[port("payload")]},
                {"id":"output","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"independent_output","kind":"output","inputs":[port("payload")],"outputs":[]}
            ],"edges":[edge("input","diagnose"),edge("input","raw"),edge("diagnose","pass"),
                edge("pass","output"),edge("independent","independent_output")]
        },"input_artifacts":{
            "input":{
                "nodes":(0..129).map(|id| json!({"id":format!("n-{id}"),"velocity_magnitude":2.0,"pressure":-3.0})).collect::<Vec<_>>(),
                "elements":[{"id":"e","divergence_error":0.01,"reynolds_number":0.5,"viscous_dissipation":0.25}]
            },"independent":{"value":7}
        }
    });
    if last {
        value["graph"]["nodes"].as_array_mut().unwrap().truncate(2);
        value["graph"]["nodes"][1]["outputs"] = json!([]);
        value["graph"]["edges"].as_array_mut().unwrap().truncate(1);
        value["graph"]["entry_nodes"] = json!(["input"]);
        value["graph"]["output_nodes"] = json!([]);
    }
    if last || missing {
        value["input_artifacts"]
            .as_object_mut()
            .unwrap()
            .remove("independent");
    }
    serde_json::from_value(value).unwrap()
}

fn options(artifact_projection: WorkflowArtifactProjection) -> WorkflowGraphRunOptions {
    WorkflowGraphRunOptions {
        artifact_projection,
    }
}

fn inner_error(result: Result<WorkflowGraphRunResult, String>) -> String {
    match result {
        Err(error) => error,
        Ok(run) => panic!(
            "cancelled workflow returned success: completed={:?}, failed={:?}",
            run.completed_nodes, run.failed_nodes
        ),
    }
}

fn capture(
    request: WorkflowGraphRunRequest,
    projection: WorkflowArtifactProjection,
) -> (String, Result<WorkflowGraphRunResult, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let mut inner = None;
    let outer = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResultDiagnostics && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            inner = Some(run_workflow_graph_with_options(
                request,
                options(projection),
            ));
            Ok::<(), String>(())
        },
    )
    .unwrap_err();
    assert!(control.was_interrupted());
    assert!(
        outer.contains("cancelled at result_diagnostics after 64 steps"),
        "{outer}"
    );
    (
        outer,
        inner.expect("the active outer scope entered the workflow"),
    )
}

#[test]
fn workflow_cancellation_boundaries_skip_policy_cannot_return_an_inner_success() {
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        for nested in [false, true] {
            let mut input = request(true, false, false);
            if nested {
                input.graph.nodes[1].config = Some(json!({"recovery":{"on_error":"skip"}}));
            }
            let (outer, inner) = capture(input, projection);
            assert_eq!(inner_error(inner), outer);
            let fresh = SolverControl::default();
            let run = with_solver_control(&fresh, || {
                run_workflow_graph_with_options(request(false, false, false), options(projection))
            })
            .unwrap();
            assert!(run.failed_nodes.is_empty() && run.skipped_nodes.is_empty());
            if projection != WorkflowArtifactProjection::None {
                assert_eq!(run.artifacts["output.payload"]["cfd_velocity_max"], 2.0);
                assert_eq!(
                    run.artifacts["independent_output.payload"],
                    json!({"value":7})
                );
            }
            assert!(!fresh.was_interrupted());
        }
    }
}

#[test]
fn workflow_cancellation_boundaries_stop_before_a_queued_independent_node() {
    let input = request(true, false, true);
    let (outer, inner) = capture(input.clone(), WorkflowArtifactProjection::All);
    assert_eq!(inner_error(inner), outer);
    let fresh = SolverControl::default();
    let error = with_solver_control(&fresh, || {
        run_workflow_graph_with_options(input, options(WorkflowArtifactProjection::All))
    })
    .unwrap_err();
    assert!(
        error.contains("missing workflow input artifact for node independent"),
        "{error}"
    );
    assert!(!fresh.was_interrupted());
}

#[test]
fn workflow_cancellation_boundaries_final_recoverable_node_cannot_complete_the_graph() {
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        let (outer, inner) = capture(request(true, true, false), projection);
        assert_eq!(inner_error(inner), outer);
    }
}

#[test]
fn workflow_cancellation_boundaries_pending_cancel_stops_a_pure_forwarding_graph() {
    for projection in [
        WorkflowArtifactProjection::All,
        WorkflowArtifactProjection::Outputs,
        WorkflowArtifactProjection::None,
    ] {
        let mut input = request(true, false, false);
        input
            .graph
            .nodes
            .retain(|node| node.id == "input" || node.id == "raw");
        input.graph.edges.retain(|edge| edge.to.node == "raw");
        input.graph.entry_nodes = vec!["input".into()];
        input.graph.output_nodes = vec!["raw".into()];
        input.input_artifacts.remove("independent");
        let control = SolverControl::default();
        let mut inner = None;
        let outer = with_solver_control(&control, || {
            control.request_cancel();
            inner = Some(run_workflow_graph_with_options(input, options(projection)));
            Ok::<(), String>(())
        })
        .unwrap_err();
        assert_eq!(inner_error(inner.unwrap()), outer);
        assert!(outer.contains("cancelled before numerical execution"));
        assert!(control.last_checkpoint().is_none());
    }
}

#[test]
fn workflow_cancellation_boundaries_ordinary_errors_keep_recovery_and_unobserved_error_identity() {
    let mut input = request(true, false, false);
    input.graph.nodes[1].id = "cancelled-task".into();
    for edge in &mut input.graph.edges {
        if edge.from.node == "diagnose" {
            edge.from.node = "cancelled-task".into();
        }
        if edge.to.node == "diagnose" {
            edge.to.node = "cancelled-task".into();
        }
    }
    input.input_artifacts.get_mut("input").unwrap()["nodes"][0]["pressure"] = Value::Null;
    let fresh = SolverControl::default();
    let result = with_solver_control(&fresh, || {
        run_workflow_graph_with_options(input.clone(), options(WorkflowArtifactProjection::All))
    })
    .unwrap();
    assert_eq!(result.failed_nodes, ["cancelled-task"]);
    assert_eq!(
        result.artifacts["independent_output.payload"],
        json!({"value":7})
    );
    assert!(!result.artifacts.contains_key("cancelled-task.payload"));
    assert!(!fresh.was_interrupted());
    input.graph.nodes[1].config = Some(json!({"on_error":"fail"}));
    let control = SolverControl::default();
    let error = with_solver_control(&control, || {
        let error =
            run_workflow_graph_with_options(input, options(WorkflowArtifactProjection::All))
                .unwrap_err();
        control.request_cancel();
        Err::<(), String>(error)
    })
    .unwrap_err();
    assert!(
        error.contains("pressure") && error.contains("cancelled-task"),
        "{error}"
    );
    assert!(!error.contains("solver cancelled"));
    assert!(!control.was_interrupted());
}
