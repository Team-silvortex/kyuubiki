use crate::{
    built_in_operator_descriptors, chunk_result, describe_built_in_operator,
    is_supported_workflow_operator, solve, supported_workflow_operator_ids,
    workflow_solve_executor::run_solve_operator, EngineSolveRequest,
};
use kyuubiki_protocol::{
    AnalysisResult, OperatorKind, ResultChunkKind, ResultChunkRequest, SolidTetra3dElementInput,
    SolidTetra3dNodeInput, SolveBarRequest, SolveSolidTetra3dRequest,
    SolveStokesFlowPlaneQuad2dRequest, SolveTruss2dRequest, StokesFlowPlaneNodeInput,
    StokesFlowPlaneQuadElementInput, TrussElementInput, TrussNodeInput,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn solves_through_engine_facade() {
    let result = solve(EngineSolveRequest::Bar1d(SolveBarRequest {
        length: 1.0,
        area: 0.01,
        youngs_modulus: 210.0e9,
        elements: 1,
        tip_force: 1000.0,
    }))
    .expect("bar should solve");

    assert!(matches!(result, AnalysisResult::Bar1d(_)));
}

#[test]
fn runs_stokes_flow_through_workflow_solve_executor() {
    let payload = serde_json::to_value(SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            stokes_node("n0", 0.0, 0.0, true, true, 0.0, 0.0),
            stokes_node("n1", 1.0, 0.0, false, true, 2.0, 0.0),
            stokes_node("n2", 1.0, 1.0, false, false, 2.0, 0.5),
            stokes_node("n3", 0.0, 1.0, true, true, 0.0, 0.0),
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "sf0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            viscosity: 2.0,
            density: 1.0,
        }],
    })
    .expect("payload should encode");

    let result = run_solve_operator("solve.stokes_flow_quad_2d", payload)
        .expect("stokes flow workflow solve should run");

    assert_eq!(result["elements"][0]["id"], "sf0");
    assert!(result["max_velocity"].as_f64().unwrap() > 0.0);
    assert!(result["max_reynolds_number"].as_f64().unwrap() > 0.0);
}

#[test]
fn chunks_result_items() {
    let result = solve(EngineSolveRequest::Truss2d(SolveTruss2dRequest {
        nodes: vec![
            TrussNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            TrussNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            TrussNodeInput {
                id: "n2".to_string(),
                x: 0.5,
                y: 0.75,
                fix_x: false,
                fix_y: false,
                load_x: 0.0,
                load_y: -1000.0,
            },
        ],
        elements: vec![
            TrussElementInput {
                id: "e0".to_string(),
                node_i: 0,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "e1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "e2".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
        ],
    }))
    .expect("truss should solve");

    let chunk = chunk_result(
        &result,
        &ResultChunkRequest {
            kind: ResultChunkKind::Nodes,
            offset: 1,
            limit: 1,
        },
    )
    .expect("chunk should build");

    assert_eq!(chunk.total, 3);
    assert_eq!(chunk.returned, 1);
    assert_eq!(chunk.offset, 1);
}

#[test]
fn chunks_solid_tetra_nodes_and_elements() {
    let result = solve(EngineSolveRequest::SolidTetra3d(SolveSolidTetra3dRequest {
        nodes: vec![
            solid_node("n0", 0.0, 0.0, 0.0, true),
            solid_node("n1", 1.0, 0.0, 0.0, true),
            solid_node("n2", 0.0, 1.0, 0.0, true),
            solid_node("tip", 0.0, 0.0, 1.0, false),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "tet0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    }))
    .expect("solid tetra should solve");

    let nodes = chunk_result(
        &result,
        &ResultChunkRequest {
            kind: ResultChunkKind::Nodes,
            offset: 3,
            limit: 1,
        },
    )
    .expect("node chunk should build");
    let elements = chunk_result(
        &result,
        &ResultChunkRequest {
            kind: ResultChunkKind::Elements,
            offset: 0,
            limit: 1,
        },
    )
    .expect("element chunk should build");

    assert_eq!(nodes.total, 4);
    assert_eq!(nodes.items[0]["id"], "tip");
    assert_eq!(elements.total, 1);
    assert_eq!(elements.items[0]["id"], "tet0");
}

#[test]
fn exposes_verified_built_in_operator_descriptors() {
    let descriptors = built_in_operator_descriptors();
    assert!(descriptors.len() >= 4);
    assert!(descriptors
        .iter()
        .any(|descriptor| descriptor.id == "solve.frame_3d"));

    let descriptor = describe_built_in_operator("solve.thermal_frame_3d").expect("descriptor");
    assert_eq!(descriptor.kind, OperatorKind::Solver);
    assert_eq!(descriptor.family, "thermal_frame_3d");
    assert!(descriptor
        .capability_tags
        .iter()
        .any(|tag| tag == "verified"));
    assert!(descriptors.iter().any(|descriptor| {
        descriptor.id == "bridge.electrostatic_field_to_heat_quad_2d"
            && descriptor.kind == OperatorKind::WorkflowBridge
    }));
}

#[test]
fn built_in_workflow_descriptors_are_headless_supported() {
    let unsupported = built_in_operator_descriptors()
        .into_iter()
        .filter(|descriptor| !is_supported_workflow_operator(&descriptor.id))
        .map(|descriptor| descriptor.id)
        .collect::<Vec<_>>();

    assert!(
        unsupported.is_empty(),
        "built-in workflow descriptors missing headless support: {}",
        unsupported.join(", ")
    );
}

#[test]
fn frontend_workflow_templates_only_use_headless_supported_operators() {
    let templates_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../../apps/frontend/src/components/workbench/workflow/workbench-workflow-node-templates.ts",
    );
    let templates_source =
        fs::read_to_string(&templates_path).expect("workflow template source should be readable");

    let operator_ids = templates_source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("operatorId: "))
        .filter_map(|value| value.strip_prefix('"'))
        .filter_map(|value| value.split('"').next())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();

    let supported = supported_workflow_operator_ids()
        .into_iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();

    let unsupported = operator_ids
        .difference(&supported)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        unsupported.is_empty(),
        "frontend workflow templates reference unsupported headless operators: {}",
        unsupported.join(", ")
    );
}

#[test]
fn supported_headless_workflow_operators_have_built_in_descriptors() {
    let descriptor_ids = built_in_operator_descriptors()
        .into_iter()
        .map(|descriptor| descriptor.id)
        .collect::<BTreeSet<_>>();

    let undocumented = supported_workflow_operator_ids()
        .into_iter()
        .filter(|operator_id| !descriptor_ids.contains(*operator_id))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    assert!(
        undocumented.is_empty(),
        "supported headless workflow operators missing built-in descriptors: {}",
        undocumented.join(", ")
    );
}

#[test]
fn diagnostics_template_matches_rust_workflow_report_chain() {
    let template_source = diagnostics_template_sources();

    for required in [
        "\"id\" => \"workflow.diagnostics-bundle-guard-report-markdown\"",
        "\"operator_id\" => \"transform.compose_diagnostics_bundle\"",
        "\"operator_id\" => \"transform.evaluate_diagnostics_bundle_guard\"",
        "\"operator_id\" => \"transform.compose_diagnostics_report_payload\"",
        "\"operator_id\" => \"export.diagnostics_bundle_markdown\"",
        "\"dataset_value\" => \"diagnostics_bundle\"",
        "\"dataset_value\" => \"guard_result\"",
        "\"dataset_value\" => \"report_payload\"",
        "\"dataset_value\" => \"markdown_report\"",
    ] {
        assert!(
            template_source.contains(required),
            "diagnostics template drifted, missing required fragment: {required}"
        );
    }
}

#[test]
fn peak_diagnostics_template_matches_rust_workflow_report_chain() {
    let template_source = diagnostics_template_sources();

    for required in [
        "\"id\" => \"workflow.peak-diagnostics-bundle-report-markdown\"",
        "\"operator_id\" => \"transform.compose_diagnostics_bundle\"",
        "\"operator_id\" => \"transform.evaluate_diagnostics_bundle_guard\"",
        "\"operator_id\" => \"transform.compose_diagnostics_report_payload\"",
        "\"operator_id\" => \"export.diagnostics_bundle_markdown\"",
        "\"field\" => \"electrostatic_field_peak_magnitude\"",
        "\"field\" => \"thermal_flux_peak_magnitude\"",
        "\"field\" => \"thermo_peak_stress\"",
        "\"dataset_value\" => \"diagnostics_bundle\"",
        "\"dataset_value\" => \"guard_result\"",
        "\"dataset_value\" => \"report_payload\"",
        "\"dataset_value\" => \"markdown_report\"",
    ] {
        assert!(
            template_source.contains(required),
            "peak diagnostics template drifted, missing required fragment: {required}"
        );
    }
}

#[test]
fn coupled_diagnostics_template_matches_rust_workflow_report_chain() {
    let template_source = diagnostics_template_sources();

    for required in [
        "\"id\" => \"workflow.electrostatic-heat-thermo-diagnostics-markdown\"",
        "\"solve.electrostatic_plane_quad_2d\"",
        "\"bridge.electrostatic_field_to_heat_quad_2d\"",
        "\"solve.heat_plane_quad_2d\"",
        "\"bridge.temperature_field_to_thermo_quad_2d\"",
        "\"solve.thermal_plane_quad_2d\"",
        "\"extract.electrostatic_result_diagnostics\"",
        "\"extract.thermal_result_diagnostics\"",
        "\"extract.thermo_result_diagnostics\"",
        "\"operator_id\" => \"transform.compose_diagnostics_bundle\"",
        "\"operator_id\" => \"transform.evaluate_diagnostics_bundle_guard\"",
        "\"operator_id\" => \"transform.compose_diagnostics_report_payload\"",
        "\"operator_id\" => \"export.diagnostics_bundle_markdown\"",
        "\"dataset_value\" => \"electrostatic_diagnostics\"",
        "\"dataset_value\" => \"thermal_diagnostics\"",
        "\"dataset_value\" => \"thermo_diagnostics\"",
        "\"dataset_value\" => \"diagnostics_bundle\"",
        "\"dataset_value\" => \"guard_result\"",
        "\"dataset_value\" => \"report_payload\"",
        "\"dataset_value\" => \"markdown_report\"",
        "\"field\" => \"thermo_stress_peak\"",
    ] {
        assert!(
            template_source.contains(required),
            "coupled diagnostics template drifted, missing required fragment: {required}"
        );
    }
}

fn diagnostics_template_sources() -> String {
    [
        "workflow_template_diagnostics_entries.ex",
        "workflow_template_diagnostics_graphs.ex",
        "workflow_template_diagnostics_graph_nodes.ex",
    ]
    .iter()
    .map(|file_name| {
        let template_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../apps/web/lib/kyuubiki_web")
            .join(file_name);

        fs::read_to_string(&template_path).unwrap_or_else(|error| {
            panic!("diagnostics template source should be readable: {error}")
        })
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn solid_node(id: &str, x: f64, y: f64, z: f64, fixed: bool) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: if fixed { 0.0 } else { -1000.0 },
    }
}

fn stokes_node(
    id: &str,
    x: f64,
    y: f64,
    fix_velocity_x: bool,
    fix_velocity_y: bool,
    body_force_x: f64,
    body_force_y: f64,
) -> StokesFlowPlaneNodeInput {
    StokesFlowPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_velocity_x,
        velocity_x: 0.0,
        fix_velocity_y,
        velocity_y: 0.0,
        fix_pressure: id == "n0",
        pressure: if id == "n0" { 1.0 } else { 0.0 },
        body_force_x,
        body_force_y,
    }
}
