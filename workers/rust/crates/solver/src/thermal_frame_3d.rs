use std::borrow::Cow;

use crate::frame_3d_element::{
    FrameElement, elastic_energy, finite_fields, norm3, total_energy, validate_matrix,
};
use crate::frame_3d_math::normalized_direction;
use crate::linear_algebra::{SparseMatrix, add_at, solve_spd_system_profile_with_options};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::{max_results, try_collect_results};
use crate::thermal_frame_3d_constraints::ThermalFrame3dConstraintSystem;
use crate::thermal_frame_3d_validation::validate_request;
use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, SolveThermalFrame3dResult,
    ThermalFrame3dDirectionalRotationalSpringResult, ThermalFrame3dDirectionalSpringResult,
    ThermalFrame3dElementResult, ThermalFrame3dNodeResult,
};

const THERMAL_FRAME_SGS_NODE_THRESHOLD: usize = 90_000;

pub fn solve_thermal_frame_3d(
    request: &SolveThermalFrame3dRequest,
) -> Result<SolveThermalFrame3dResult, String> {
    solve_thermal_frame_3d_internal(
        Cow::Borrowed(request),
        default_thermal_frame_options(request.nodes.len()),
    )
}

pub fn solve_thermal_frame_3d_owned(
    request: SolveThermalFrame3dRequest,
) -> Result<SolveThermalFrame3dResult, String> {
    let options = default_thermal_frame_options(request.nodes.len());
    solve_thermal_frame_3d_internal(Cow::Owned(request), options)
}

pub fn solve_thermal_frame_3d_with_options(
    request: &SolveThermalFrame3dRequest,
    options: SpdSolveOptions,
) -> Result<SolveThermalFrame3dResult, String> {
    solve_thermal_frame_3d_internal(Cow::Borrowed(request), options)
}

fn solve_thermal_frame_3d_internal(
    request: Cow<'_, SolveThermalFrame3dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveThermalFrame3dResult, String> {
    validate_request(&request)?;
    let constraints = ThermalFrame3dConstraintSystem::build(&request)?;
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
    let count = request.elements.len()
        + request.directional_springs.len()
        + request.directional_rotational_springs.len();
    let mut completed = 0;
    for spring in &request.directional_springs {
        assemble_spring(
            &mut stiffness,
            spring.node * 6,
            spring.direction,
            spring.stiffness,
        )?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementAssembly, completed, count)?;
    }
    for spring in &request.directional_rotational_springs {
        assemble_spring(
            &mut stiffness,
            spring.node * 6 + 3,
            spring.direction,
            spring.stiffness,
        )?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementAssembly, completed, count)?;
    }
    for e in &request.elements {
        FrameElement::thermal(&request, e)?.assemble(&mut stiffness, &mut forces)?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementAssembly, completed, count)?;
    }
    // Constrained rows must be valid too: projection must not hide bad assembly.
    validate_matrix(&stiffness, dof_count)?;
    let (reduced, force) = constraints.project(&stiffness, &forces)?;
    let solution = if force.is_empty() {
        Vec::new()
    } else {
        solve_spd_system_profile_with_options(&reduced, &force, options)?.solution
    };
    let displacements = constraints.restore(&solution)?;

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
            Ok(ThermalFrame3dNodeResult {
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
                temperature_delta: node.temperature_delta,
            })
        }),
    )?;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, e)| {
            let kernel = FrameElement::thermal(&request, e)?;
            let r = kernel.recover(&displacements)?;
            let f = r.forces;
            Ok(ThermalFrame3dElementResult {
                index,
                id: e.id.clone(),
                node_i: e.node_i,
                node_j: e.node_j,
                length: kernel.length,
                average_temperature_delta: kernel.temperature,
                thermal_strain: kernel.thermal_strain,
                mechanical_strain: r.mechanical_strain,
                total_strain: r.total_strain,
                temperature_gradient_y: e.temperature_gradient_y,
                temperature_gradient_z: e.temperature_gradient_z,
                thermal_curvature_y: kernel.thermal_curvatures[0],
                thermal_curvature_z: kernel.thermal_curvatures[1],
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
    let directional_springs = try_collect_results(
        SolverStage::ResultElements,
        request
            .directional_springs
            .iter()
            .enumerate()
            .map(|(index, s)| {
                let (direction, displacement, reaction_force, strain_energy) =
                    spring_state(&s.id, s.node * 6, s.direction, s.stiffness, &displacements)?;
                Ok(ThermalFrame3dDirectionalSpringResult {
                    index,
                    id: s.id.clone(),
                    node: s.node,
                    direction,
                    displacement,
                    reaction_force,
                    stiffness: s.stiffness,
                    strain_energy,
                })
            }),
    )?;
    let directional_rotational_springs = try_collect_results(
        SolverStage::ResultElements,
        request
            .directional_rotational_springs
            .iter()
            .enumerate()
            .map(|(index, s)| {
                let (direction, rotation, reaction_moment, strain_energy) = spring_state(
                    &s.id,
                    s.node * 6 + 3,
                    s.direction,
                    s.stiffness,
                    &displacements,
                )?;
                Ok(ThermalFrame3dDirectionalRotationalSpringResult {
                    index,
                    id: s.id.clone(),
                    node: s.node,
                    direction,
                    rotation,
                    reaction_moment,
                    stiffness: s.stiffness,
                    strain_energy,
                })
            }),
    )?;
    let (directional_constraints, directional_rotational_constraints) =
        constraints.build_results(&request, &stiffness, &forces, &displacements)?;
    let total_strain_energy = total_energy(elements.iter().map(|e| e.strain_energy))?
        + total_energy(directional_springs.iter().map(|s| s.strain_energy))?
        + total_energy(
            directional_rotational_springs
                .iter()
                .map(|s| s.strain_energy),
        )?;
    finite_fields("summary", "total strain energy", &[total_strain_energy])?;
    Ok(SolveThermalFrame3dResult {
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
        max_axial_force: max_results(SolverStage::ResultElementSummary, &elements, |e| {
            e.axial_force_i.abs().max(e.axial_force_j.abs())
        })?,
        max_temperature_delta: max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
            n.temperature_delta.abs()
        })?,
        max_temperature_gradient: max_results(SolverStage::ResultElementSummary, &elements, |e| {
            e.temperature_gradient_y
                .abs()
                .max(e.temperature_gradient_z.abs())
        })?,
        total_strain_energy,
        nodes,
        elements,
        directional_springs,
        directional_rotational_springs,
        directional_constraints,
        directional_rotational_constraints,
    })
}

fn assemble_spring(
    matrix: &mut SparseMatrix,
    offset: usize,
    raw_direction: [f64; 3],
    stiffness: f64,
) -> Result<(), String> {
    let direction = normalized_direction(raw_direction)?;
    for row in 0..3 {
        for column in 0..3 {
            add_at(
                matrix,
                offset + row,
                offset + column,
                stiffness * direction[row] * direction[column],
            );
        }
    }
    Ok(())
}

fn spring_state(
    id: &str,
    offset: usize,
    raw_direction: [f64; 3],
    stiffness: f64,
    displacements: &[f64],
) -> Result<([f64; 3], f64, f64, f64), String> {
    let direction = normalized_direction(raw_direction)?;
    let displacement = (0..3)
        .map(|i| direction[i] * displacements[offset + i])
        .sum::<f64>();
    let reaction = -stiffness * displacement;
    finite_fields(id, "spring response", &[displacement, reaction])?;
    let energy = elastic_energy(id, displacement, stiffness, 1.0)?;
    Ok((direction, displacement, reaction, energy))
}

fn default_thermal_frame_options(node_count: usize) -> SpdSolveOptions {
    SpdSolveOptions {
        preconditioner: if node_count >= THERMAL_FRAME_SGS_NODE_THRESHOLD {
            SpdPreconditioner::SymmetricGaussSeidel
        } else {
            SpdPreconditioner::Jacobi
        },
        progress_interval: None,
    }
}
