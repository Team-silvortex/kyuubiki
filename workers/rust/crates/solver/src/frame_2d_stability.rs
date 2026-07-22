use crate::frame_2d::solve_frame_2d;
use crate::frame_2d_math::{
    frame_dof_map, frame_local_geometric_stiffness, frame_local_stiffness, frame_transform,
    transform_frame_stiffness,
};
use crate::linear_algebra::{SparseMatrix, add_at};
use kyuubiki_protocol::{
    BucklingFrame2dElementPreloadResult, SolveBucklingFrame2dRequest, SolveFrame2dResult,
};

pub(crate) struct Frame2dStabilitySystem {
    pub static_result: SolveFrame2dResult,
    pub elastic: SparseMatrix,
    pub geometric: SparseMatrix,
    pub reference_force: Vec<f64>,
    pub constrained_dofs: Vec<usize>,
    pub element_preloads: Vec<BucklingFrame2dElementPreloadResult>,
}

pub(crate) fn assemble_frame_2d_stability(
    request: &SolveBucklingFrame2dRequest,
) -> Result<Frame2dStabilitySystem, String> {
    let static_result = solve_frame_2d(&request.frame)?;
    let dof_count = request.frame.nodes.len() * 3;
    let mut elastic = SparseMatrix::new(dof_count);
    let mut geometric = SparseMatrix::new(dof_count);
    let mut reference_force = vec![0.0; dof_count];
    let mut element_preloads = Vec::with_capacity(request.frame.elements.len());

    for (index, node) in request.frame.nodes.iter().enumerate() {
        reference_force[index * 3] = node.load_x;
        reference_force[index * 3 + 1] = node.load_y;
        reference_force[index * 3 + 2] = node.moment_z;
    }
    for (index, element) in request.frame.elements.iter().enumerate() {
        let node_i = &request.frame.nodes[element.node_i];
        let node_j = &request.frame.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let transform = frame_transform(dx / length, dy / length);
        let local_elastic = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let static_element = &static_result.elements[index];
        let signed_axial_force =
            0.5 * (static_element.axial_force_i - static_element.axial_force_j);
        let reference_compressive_force = signed_axial_force.max(0.0);
        let active = reference_compressive_force > 1.0e-12;
        let local_geometric = frame_local_geometric_stiffness(reference_compressive_force, length);
        let map = frame_dof_map(element.node_i, element.node_j);
        assemble(
            &mut elastic,
            &transform_frame_stiffness(&local_elastic, &transform),
            &map,
        );
        assemble(
            &mut geometric,
            &transform_frame_stiffness(&local_geometric, &transform),
            &map,
        );
        element_preloads.push(BucklingFrame2dElementPreloadResult {
            index,
            id: element.id.clone(),
            signed_axial_force,
            reference_compressive_force,
            active_in_geometric_stiffness: active,
        });
    }
    if !element_preloads
        .iter()
        .any(|preload| preload.active_in_geometric_stiffness)
    {
        return Err("buckling frame 2d reference load produces no compressive member force".into());
    }

    Ok(Frame2dStabilitySystem {
        static_result,
        elastic,
        geometric,
        reference_force,
        constrained_dofs: constrained_dofs(request),
        element_preloads,
    })
}

fn assemble(global: &mut SparseMatrix, element: &[[f64; 6]; 6], map: &[usize; 6]) {
    for row in 0..6 {
        for column in 0..6 {
            add_at(global, map[row], map[column], element[row][column]);
        }
    }
}

fn constrained_dofs(request: &SolveBucklingFrame2dRequest) -> Vec<usize> {
    request
        .frame
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 3),
                node.fix_y.then_some(index * 3 + 1),
                node.fix_rz.then_some(index * 3 + 2),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}
