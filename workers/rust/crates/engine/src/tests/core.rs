use crate::{
    EngineSolveRequest, built_in_operator_descriptors, chunk_result, describe_built_in_operator,
    is_supported_workflow_operator, solve, supported_workflow_operator_ids,
};
use kyuubiki_protocol::{
    AnalysisResult, OperatorKind, ResultChunkKind, ResultChunkRequest, SolveBarRequest,
    SolveTruss2dRequest, TrussElementInput, TrussNodeInput,
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
fn exposes_verified_built_in_operator_descriptors() {
    let descriptors = built_in_operator_descriptors();
    assert!(descriptors.len() >= 4);
    assert!(
        descriptors
            .iter()
            .any(|descriptor| descriptor.id == "solve.frame_3d")
    );

    let descriptor = describe_built_in_operator("solve.thermal_frame_3d").expect("descriptor");
    assert_eq!(descriptor.kind, OperatorKind::Solver);
    assert_eq!(descriptor.family, "thermal_frame_3d");
    assert!(
        descriptor
            .capability_tags
            .iter()
            .any(|tag| tag == "verified")
    );
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
