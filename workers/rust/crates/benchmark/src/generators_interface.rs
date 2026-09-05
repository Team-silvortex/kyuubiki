use kyuubiki_protocol::{
    CohesiveInterface2dDisplacementStepInput, CohesiveInterface2dElementInput,
    CohesiveInterface2dMaterialInput, CohesiveInterface2dNodeInput,
    CohesiveInterface3dMaterialInput, CohesiveInterfaceMesh2dElementInput,
    CohesiveInterfaceMesh2dMaterialInput, CohesiveInterfaceMesh2dNodeInput,
    CohesiveInterfaceMesh3dElementInput, CohesiveInterfaceMesh3dMaterialInput,
    CohesiveInterfaceMesh3dNodeInput, SolveCohesiveInterface1dRequest,
    SolveCohesiveInterface2dRequest, SolveCohesiveInterfaceMesh2dRequest,
    SolveCohesiveInterfaceMesh3dRequest,
};

const MAX_HISTORY_STEPS: usize = 4_096;
const MAX_MESH_2D_ELEMENTS: usize = 96;
const MAX_MESH_3D_ELEMENTS: usize = 80;

pub(crate) fn generate_cohesive_interface_1d_case(scale: usize) -> SolveCohesiveInterface1dRequest {
    let history_steps = bounded_history_steps(scale);
    SolveCohesiveInterface1dRequest {
        id: "benchmark-interface-1d".to_string(),
        initial_stiffness: 1_000.0,
        compression_stiffness: 2_000.0,
        peak_traction: 10.0,
        failure_separation: 0.05,
        separation_history: (0..history_steps)
            .map(|step| cyclic_separation(step, history_steps, 0.04))
            .collect(),
    }
}

pub(crate) fn generate_cohesive_interface_2d_case(scale: usize) -> SolveCohesiveInterface2dRequest {
    let history_steps = bounded_history_steps(scale);
    SolveCohesiveInterface2dRequest {
        nodes: [[0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [1.0, 0.0]]
            .into_iter()
            .enumerate()
            .map(|(index, point)| CohesiveInterface2dNodeInput {
                id: format!("ci{index}"),
                x: point[0],
                y: point[1],
            })
            .collect(),
        element: CohesiveInterface2dElementInput {
            id: "benchmark-interface-2d".to_string(),
            lower_i: 0,
            lower_j: 1,
            upper_i: 2,
            upper_j: 3,
            thickness: 1.0,
        },
        material: cohesive_history_material_2d(),
        displacement_history: (0..history_steps)
            .map(|step| {
                let normal = cyclic_separation(step, history_steps, 0.04);
                let shear = cyclic_separation(step, history_steps, 0.025);
                CohesiveInterface2dDisplacementStepInput {
                    nodal_displacements: vec![
                        [0.0, 0.0],
                        [0.0, 0.0],
                        [shear, normal],
                        [shear, normal],
                    ],
                }
            })
            .collect(),
    }
}

pub(crate) fn generate_cohesive_interface_mesh_2d_case(
    scale: usize,
) -> SolveCohesiveInterfaceMesh2dRequest {
    let segment_count = (scale / 4).clamp(1, MAX_MESH_2D_ELEMENTS);
    let mut nodes = Vec::with_capacity(segment_count * 4);
    let mut elements = Vec::with_capacity(segment_count);
    for segment in 0..segment_count {
        let x = segment as f64 * 1.25;
        let first = nodes.len();
        nodes.extend([
            mesh_2d_node(format!("lower-{segment}-i"), x, true, [0.0, 0.0]),
            mesh_2d_node(format!("lower-{segment}-j"), x + 1.0, true, [0.0, 0.0]),
            mesh_2d_node(format!("upper-{segment}-i"), x, false, [0.25, 0.5]),
            mesh_2d_node(format!("upper-{segment}-j"), x + 1.0, false, [0.25, 0.5]),
        ]);
        elements.push(CohesiveInterfaceMesh2dElementInput {
            id: format!("interface-{segment}"),
            lower_i: first,
            lower_j: first + 1,
            upper_i: first + 2,
            upper_j: first + 3,
            thickness: 1.0,
            material_id: "adhesive".to_string(),
        });
    }

    SolveCohesiveInterfaceMesh2dRequest {
        id: format!("benchmark-interface-mesh-2d-{segment_count}"),
        nodes,
        materials: vec![CohesiveInterfaceMesh2dMaterialInput {
            id: "adhesive".to_string(),
            properties: cohesive_mesh_material_2d(),
        }],
        elements,
        connector_springs: vec![],
        host_trusses: vec![],
        host_plane_triangles: vec![],
        host_plane_quads: vec![],
        host_frames: vec![],
        load_steps: Some(1),
        control_history: None,
        max_iterations: Some(8),
        tolerance: Some(1.0e-11),
    }
}

pub(crate) fn generate_cohesive_interface_mesh_3d_case(
    scale: usize,
) -> SolveCohesiveInterfaceMesh3dRequest {
    let element_count = (scale / 6).clamp(1, MAX_MESH_3D_ELEMENTS);
    let mut nodes = Vec::with_capacity(element_count * 6);
    let mut elements = Vec::with_capacity(element_count);
    for element in 0..element_count {
        let origin = element as f64 * 2.0;
        let base = nodes.len();
        for (suffix, x, y, upper) in [
            ("lower-a", origin, 0.0, false),
            ("lower-b", origin + 1.0, 0.0, false),
            ("lower-c", origin, 1.0, false),
            ("upper-a", origin, 0.0, true),
            ("upper-b", origin + 1.0, 0.0, true),
            ("upper-c", origin, 1.0, true),
        ] {
            nodes.push(mesh_3d_node(element, suffix, x, y, upper));
        }
        elements.push(CohesiveInterfaceMesh3dElementInput {
            id: format!("interface-{element}"),
            lower_a: base,
            lower_b: base + 1,
            lower_c: base + 2,
            upper_a: base + 3,
            upper_b: base + 4,
            upper_c: base + 5,
            material_id: "adhesive".to_string(),
        });
    }

    SolveCohesiveInterfaceMesh3dRequest {
        id: format!("benchmark-interface-mesh-3d-{element_count}"),
        nodes,
        materials: vec![CohesiveInterfaceMesh3dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface3dMaterialInput {
                normal_initial_stiffness: 1_000.0,
                normal_compression_stiffness: 2_000.0,
                normal_peak_traction: 100.0,
                normal_failure_separation: 1.0,
                shear_initial_stiffness: 500.0,
                shear_peak_traction: 50.0,
                shear_failure_separation: 1.0,
            },
        }],
        elements,
        host_tetrahedra: vec![],
        load_steps: Some(1),
        control_history: None,
        max_iterations: Some(8),
        tolerance: Some(1.0e-11),
    }
}

fn bounded_history_steps(scale: usize) -> usize {
    scale.clamp(4, MAX_HISTORY_STEPS)
}

fn cyclic_separation(step: usize, steps: usize, peak: f64) -> f64 {
    let phase = step as f64 / steps.saturating_sub(1).max(1) as f64;
    let cycle = 1.0 - (2.0 * phase - 1.0).abs();
    peak * cycle
}

fn cohesive_history_material_2d() -> CohesiveInterface2dMaterialInput {
    CohesiveInterface2dMaterialInput {
        normal_initial_stiffness: 1_000.0,
        normal_compression_stiffness: 2_000.0,
        normal_peak_traction: 10.0,
        normal_failure_separation: 0.05,
        shear_initial_stiffness: 800.0,
        shear_peak_traction: 8.0,
        shear_failure_separation: 0.05,
    }
}

fn cohesive_mesh_material_2d() -> CohesiveInterface2dMaterialInput {
    CohesiveInterface2dMaterialInput {
        normal_initial_stiffness: 1_000.0,
        normal_compression_stiffness: 2_000.0,
        normal_peak_traction: 100.0,
        normal_failure_separation: 0.5,
        shear_initial_stiffness: 800.0,
        shear_peak_traction: 80.0,
        shear_failure_separation: 0.5,
    }
}

fn mesh_2d_node(
    id: String,
    x: f64,
    fixed: bool,
    load: [f64; 2],
) -> CohesiveInterfaceMesh2dNodeInput {
    CohesiveInterfaceMesh2dNodeInput {
        id,
        x,
        y: 0.0,
        fixed: [fixed, fixed],
        prescribed_displacement: None,
        load,
        fixed_rotation: false,
        prescribed_rotation: None,
        moment_z: 0.0,
    }
}

fn mesh_3d_node(
    element: usize,
    suffix: &str,
    x: f64,
    y: f64,
    upper: bool,
) -> CohesiveInterfaceMesh3dNodeInput {
    CohesiveInterfaceMesh3dNodeInput {
        id: format!("{element}-{suffix}"),
        x,
        y,
        z: 0.0,
        fixed: if upper {
            [true, true, false]
        } else {
            [true; 3]
        },
        prescribed_displacement: None,
        load: if upper {
            [0.0, 0.0, 1_000.0 * 0.001 * 0.5 / 3.0]
        } else {
            [0.0; 3]
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_HISTORY_STEPS, MAX_MESH_2D_ELEMENTS, MAX_MESH_3D_ELEMENTS,
        generate_cohesive_interface_1d_case, generate_cohesive_interface_2d_case,
        generate_cohesive_interface_mesh_2d_case, generate_cohesive_interface_mesh_3d_case,
    };

    #[test]
    fn interface_generators_respect_solver_scale_contracts() {
        let one_d = generate_cohesive_interface_1d_case(1_000_000);
        let two_d = generate_cohesive_interface_2d_case(1_000_000);
        let mesh_2d = generate_cohesive_interface_mesh_2d_case(1_000_000);
        let mesh_3d = generate_cohesive_interface_mesh_3d_case(1_000_000);

        assert_eq!(one_d.separation_history.len(), MAX_HISTORY_STEPS);
        assert_eq!(two_d.displacement_history.len(), MAX_HISTORY_STEPS);
        assert_eq!(mesh_2d.elements.len(), MAX_MESH_2D_ELEMENTS);
        assert_eq!(mesh_2d.nodes.len(), MAX_MESH_2D_ELEMENTS * 4);
        assert_eq!(mesh_3d.elements.len(), MAX_MESH_3D_ELEMENTS);
        assert_eq!(mesh_3d.nodes.len(), MAX_MESH_3D_ELEMENTS * 6);
    }
}
