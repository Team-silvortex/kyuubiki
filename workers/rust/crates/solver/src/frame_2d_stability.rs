use crate::frame_2d::solve_frame_2d;
use crate::frame_2d_math::{
    frame_dof_map, frame_local_geometric_stiffness, frame_local_stiffness, frame_transform,
    transform_frame_stiffness,
};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
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
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let dof_count = request.frame.nodes.len() * 3;
    let mut elastic = SparseMatrix::new(dof_count);
    let mut geometric = SparseMatrix::new(dof_count);
    let mut reference_force = vec![0.0; dof_count];
    let mut element_preloads = Vec::with_capacity(request.frame.elements.len());

    for (index, node) in request.frame.nodes.iter().enumerate() {
        reference_force[index * 3] = node.load_x;
        reference_force[index * 3 + 1] = node.load_y;
        reference_force[index * 3 + 2] = node.moment_z;
        checkpoint_chunk(
            SolverStage::ConstraintMap,
            index + 1,
            request.frame.nodes.len(),
        )?;
    }
    for (index, element) in request.frame.elements.iter().enumerate() {
        let node_i = &request.frame.nodes[element.node_i];
        let node_j = &request.frame.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = dx.hypot(dy);
        let transform = frame_transform(dx / length, dy / length);
        let local_elastic = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let static_element = &static_result.elements[index];
        let signed_axial_force =
            0.5 * static_element.axial_force_i - 0.5 * static_element.axial_force_j;
        let displacement_i = &static_result.nodes[element.node_i];
        let displacement_j = &static_result.nodes[element.node_j];
        let displacement_scale = displacement_i
            .ux
            .hypot(displacement_i.uy)
            .max(displacement_j.ux.hypot(displacement_j.uy));
        // Local recovery resolution scales with stiffness and displacement, not
        // the units or loads on unrelated members. Keep the raw signed force.
        let force_resolution =
            (64.0 * f64::EPSILON * local_elastic[0][0].abs()) * displacement_scale;
        if !signed_axial_force.is_finite() || !force_resolution.is_finite() {
            return Err(format!(
                "buckling frame 2d element {index} axial preload or its resolution is non-finite"
            ));
        }
        let reference_compressive_force = if signed_axial_force > force_resolution {
            signed_axial_force
        } else {
            0.0
        };
        // The diagnostic must describe the same compression used to assemble Kg.
        let active = reference_compressive_force > 0.0;
        let local_geometric = frame_local_geometric_stiffness(reference_compressive_force, length);
        if local_elastic
            .iter()
            .chain(&local_geometric)
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(format!(
                "buckling frame 2d element {index} stiffness exceeds the finite numeric range"
            ));
        }
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
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.frame.elements.len(),
        )?;
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
