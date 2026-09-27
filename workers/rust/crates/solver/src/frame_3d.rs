use std::borrow::Cow;

use crate::frame_3d_element::{FrameElement, finite_fields, norm3, total_energy, validate_matrix};
use crate::frame_3d_math::frame3d_rotation_with_local_y;
use crate::linear_algebra::{
    SparseMatrix, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::{max_results, restore_solution, try_collect_results};
use kyuubiki_protocol::{
    Frame3dElementResult, Frame3dNodeResult, SolveFrame3dRequest, SolveFrame3dResult,
};

pub fn solve_frame_3d(request: &SolveFrame3dRequest) -> Result<SolveFrame3dResult, String> {
    solve_frame_3d_internal(Cow::Borrowed(request), SpdSolveOptions::default())
}

pub fn solve_frame_3d_owned(request: SolveFrame3dRequest) -> Result<SolveFrame3dResult, String> {
    solve_frame_3d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_frame_3d_with_options(
    request: &SolveFrame3dRequest,
    options: SpdSolveOptions,
) -> Result<SolveFrame3dResult, String> {
    solve_frame_3d_internal(Cow::Borrowed(request), options)
}

fn solve_frame_3d_internal(
    request: Cow<'_, SolveFrame3dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveFrame3dResult, String> {
    validate_frame_3d_request(&request)?;
    checkpoint(SolverStage::LinearPrepare, 0)?;
    let dof_count = request.nodes.len() * 6;
    let mut stiffness = SparseMatrix::new(dof_count);
    let mut forces = vec![0.0; dof_count];
    for (index, node) in request.nodes.iter().enumerate() {
        forces[index * 6..index * 6 + 6].copy_from_slice(&[
            node.load_x,
            node.load_y,
            node.load_z,
            node.moment_x,
            node.moment_y,
            node.moment_z,
        ]);
        checkpoint_chunk(SolverStage::LinearPrepare, index + 1, request.nodes.len())?;
    }
    checkpoint(SolverStage::ElementAssembly, 0)?;
    for (index, element) in request.elements.iter().enumerate() {
        FrameElement::mechanical(&request, element)?.assemble(&mut stiffness, &mut forces)?;
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }
    validate_matrix(&stiffness, dof_count)?;
    let constrained = constrained_frame_3d_dofs(&request);
    let (reduced, force, free) = reduce_sparse_system(&stiffness, &forces, &constrained)?;
    let solution = solve_spd_system_profile_with_options(&reduced, &force, options)?.solution;
    let displacements = restore_solution(dof_count, &[], &free, &solution)?;

    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let [ux, uy, uz, rx, ry, rz] =
                std::array::from_fn(|dof| displacements[index * 6 + dof]);
            let displacement_magnitude = norm3([ux, uy, uz]);
            let rotation_magnitude = norm3([rx, ry, rz]);
            finite_fields(
                &node.id,
                "displacements",
                &[
                    ux,
                    uy,
                    uz,
                    rx,
                    ry,
                    rz,
                    displacement_magnitude,
                    rotation_magnitude,
                ],
            )?;
            Ok(Frame3dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                ux,
                uy,
                uz,
                rx,
                ry,
                rz,
                displacement_magnitude,
                rotation_magnitude,
            })
        }),
    )?;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, e)| {
            let kernel = FrameElement::mechanical(&request, e)?;
            let r = kernel.recover(&displacements)?;
            let f = r.forces;
            Ok(Frame3dElementResult {
                index,
                id: e.id.clone(),
                node_i: e.node_i,
                node_j: e.node_j,
                length: kernel.length,
                axial_force_i: f[0],
                shear_force_y_i: f[1],
                shear_force_z_i: f[2],
                torsion_i: f[3],
                moment_y_i: f[4],
                moment_z_i: f[5],
                axial_force_j: f[6],
                shear_force_y_j: f[7],
                shear_force_z_j: f[8],
                torsion_j: f[9],
                moment_y_j: f[10],
                moment_z_j: f[11],
                axial_stress: r.axial_stress,
                max_bending_stress: r.bending_stress,
                max_combined_stress: r.combined_stress,
                strain_energy: r.energy,
            })
        }),
    )?;
    Ok(SolveFrame3dResult {
        input: request.into_owned(),
        max_displacement: max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
            n.displacement_magnitude
        })?,
        max_rotation: max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
            n.rotation_magnitude
        })?,
        max_moment: max_results(SolverStage::ResultElementSummary, &elements, |e| {
            e.moment_y_i
                .abs()
                .max(e.moment_z_i.abs())
                .max(e.moment_y_j.abs())
                .max(e.moment_z_j.abs())
        })?,
        max_stress: max_results(SolverStage::ResultElementSummary, &elements, |e| {
            e.max_combined_stress
        })?,
        total_strain_energy: total_energy(elements.iter().map(|e| e.strain_energy))?,
        nodes,
        elements,
    })
}

pub(super) fn validate_frame_3d_request(request: &SolveFrame3dRequest) -> Result<(), String> {
    checkpoint(SolverStage::ElementPrecompute, 0)?;
    if request.nodes.len() < 2 {
        return Err("3d frame must define at least two nodes".into());
    }
    if request.elements.is_empty() {
        return Err("3d frame must define at least one element".into());
    }
    let mut constrained = 0;
    let count = request.nodes.len() + request.elements.len();
    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() || !node.z.is_finite() {
            return Err(format!("3d frame node {index} has invalid coordinates"));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.load_z.is_finite() {
            return Err(format!("3d frame node {index} has invalid load"));
        }
        if !node.moment_x.is_finite() || !node.moment_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("3d frame node {index} has invalid moment"));
        }
        constrained += [
            node.fix_x,
            node.fix_y,
            node.fix_z,
            node.fix_rx,
            node.fix_ry,
            node.fix_rz,
        ]
        .into_iter()
        .filter(|v| *v)
        .count();
        checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, count)?;
    }
    if constrained < 6 {
        return Err("3d frame must restrain at least six degrees of freedom".into());
    }
    for (index, e) in request.elements.iter().enumerate() {
        if e.node_i >= request.nodes.len() || e.node_j >= request.nodes.len() {
            return Err("3d frame element references an out-of-range node".into());
        }
        if e.node_i == e.node_j {
            return Err("3d frame element must connect two distinct nodes".into());
        }
        for (name, value) in [
            ("area", e.area),
            ("youngs_modulus", e.youngs_modulus),
            ("shear_modulus", e.shear_modulus),
            ("torsion_constant", e.torsion_constant),
            ("moment_of_inertia_y", e.moment_of_inertia_y),
            ("moment_of_inertia_z", e.moment_of_inertia_z),
            ("section_modulus_y", e.section_modulus_y),
            ("section_modulus_z", e.section_modulus_z),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("3d frame element {name} must be positive"));
            }
        }
        let a = &request.nodes[e.node_i];
        let b = &request.nodes[e.node_j];
        let delta = [b.x - a.x, b.y - a.y, b.z - a.z];
        frame3d_rotation_with_local_y(delta[0], delta[1], delta[2], norm3(delta), e.local_y_axis)?;
        checkpoint_chunk(
            SolverStage::ElementPrecompute,
            request.nodes.len() + index + 1,
            count,
        )?;
    }
    Ok(())
}

pub(super) fn constrained_frame_3d_dofs(request: &SolveFrame3dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x,
                node.fix_y,
                node.fix_z,
                node.fix_rx,
                node.fix_ry,
                node.fix_rz,
            ]
            .into_iter()
            .enumerate()
            .filter_map(move |(dof, fixed)| fixed.then_some(index * 6 + dof))
        })
        .collect()
}
