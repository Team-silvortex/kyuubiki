use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use kyuubiki_protocol::WorkflowGraphRunRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

#[path = "support/mechanical_constraints.rs"]
mod fixtures;

fn check(kind: &str) {
    let operator = format!("solve.{kind}");
    let input = fixtures::model(kind);
    let decoded = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let baseline =
        run_solve_operator(&operator, decoded).unwrap_or_else(|error| panic!("{kind}: {error}"));
    for completed in [0, 2 * fixtures::dof_count(kind, &input)] {
        for workflow in [false, true] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::ConstraintValidate
                        && point.completed_steps == completed as u64
                    {
                        cancel.request_cancel();
                    }
                },
                || {
                    if workflow {
                        run_workflow_graph(graph(kind, input.clone())).map(|_| ())
                    } else {
                        run_solve_operator(&operator, input.clone()).map(|_| ())
                    }
                },
            );
            let error = result.unwrap_err();
            assert!(error.contains("constraint_validate"), "{kind}: {error}");
            assert!(control.was_interrupted(), "{kind}");
        }
    }
    let replay = run_solve_operator(&operator, input.clone()).unwrap();
    assert_eq!(replay, baseline, "{kind}");
    let run = run_workflow_graph(graph(kind, input.clone())).unwrap();
    assert!(run.failed_nodes.is_empty(), "{kind}");
    assert_eq!(run.artifacts["output.result"], baseline, "{kind}");
    let mut fixed = input;
    fixtures::fix_all(&mut fixed);
    let all_fixed = run_solve_operator(&operator, fixed).unwrap();
    let mut checked = 0;
    for node in all_fixed["nodes"].as_array().unwrap() {
        for field in ["ux", "uy", "uz", "rx", "ry", "rz"] {
            if let Some(value) = node[field].as_f64() {
                assert_eq!(value, 0.0, "{kind}: {field}");
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "{kind} has no checked displacement fields");
}

fn graph(kind: &str, input: Value) -> WorkflowGraphRunRequest {
    let model_type = format!("study_model/{kind}");
    let result_type = format!("result/{kind}");
    serde_json::from_value(json!({
        "graph": {
            "schema_version":"kyuubiki.workflow-graph/v1", "id":format!("zero-constraint-{kind}"),
            "name":"Mechanical original equation admission", "version":"1.0.0",
            "entry_nodes":["input"], "output_nodes":["output"],
            "nodes":[
                {"id":"input", "kind":"input", "inputs":[], "outputs":[{"id":"model", "artifact_type":model_type}]},
                {"id":"solve", "kind":"solve", "operator_id":format!("solve.{kind}"),
                 "inputs":[{"id":"model", "artifact_type":model_type}], "outputs":[{"id":"result", "artifact_type":result_type}]},
                {"id":"output", "kind":"output", "outputs":[], "inputs":[{"id":"result", "artifact_type":result_type}]}
            ],
            "edges":[
                {"id":"in", "from":{"node":"input", "port":"model"}, "to":{"node":"solve", "port":"model"}, "artifact_type":model_type},
                {"id":"out", "from":{"node":"solve", "port":"result"}, "to":{"node":"output", "port":"result"}, "artifact_type":result_type}
            ]
        }, "input_artifacts":{"input":input}
    })).unwrap()
}

macro_rules! admission_cases {
    ($($test:ident => $kind:literal),+ $(,)?) => { $(#[test] fn $test() { check($kind); })+ };
}

admission_cases! {
    homogeneous_spring_1d => "spring_1d",
    homogeneous_spring_2d => "spring_2d",
    homogeneous_spring_3d => "spring_3d",
    homogeneous_thermal_bar_1d_non_path => "thermal_bar_1d",
    homogeneous_torsion_1d => "torsion_1d",
    homogeneous_truss_2d => "truss_2d",
    homogeneous_truss_3d => "truss_3d",
    homogeneous_thermal_truss_2d => "thermal_truss_2d",
    homogeneous_thermal_truss_3d => "thermal_truss_3d",
    homogeneous_beam_1d => "beam_1d",
    homogeneous_thermal_beam_1d => "thermal_beam_1d",
    homogeneous_frame_2d => "frame_2d",
    homogeneous_thermal_frame_2d => "thermal_frame_2d",
    homogeneous_frame_3d => "frame_3d",
    homogeneous_solid_tetra_3d => "solid_tetra_3d",
    homogeneous_plane_triangle_2d => "plane_triangle_2d",
    homogeneous_plane_quad_2d => "plane_quad_2d",
    homogeneous_thermal_plane_triangle_2d => "thermal_plane_triangle_2d",
    homogeneous_thermal_plane_quad_2d => "thermal_plane_quad_2d",
}
