use std::borrow::Cow;

use crate::frame_2d_element::{FrameElement, finite_fields};
use crate::frame_2d_system::solve_system;
use crate::frame_2d_validation::{validate_frame_2d_request, validate_thermal_frame_2d_request};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};
use crate::solver_control::SolverStage;
use crate::solver_postprocess::{fold_results, max_results, try_collect_results};
use kyuubiki_protocol::{
    Frame2dElementResult, Frame2dNodeResult, SolveFrame2dRequest, SolveFrame2dResult,
    SolveThermalFrame2dRequest, SolveThermalFrame2dResult, ThermalFrame2dElementResult,
    ThermalFrame2dNodeResult,
};

const THERMAL_FRAME_SGS_NODE_THRESHOLD: usize = 90_000;

pub fn solve_frame_2d(request: &SolveFrame2dRequest) -> Result<SolveFrame2dResult, String> {
    solve_frame_2d_internal(Cow::Borrowed(request), SpdSolveOptions::default())
}

pub fn solve_frame_2d_owned(request: SolveFrame2dRequest) -> Result<SolveFrame2dResult, String> {
    solve_frame_2d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_frame_2d_with_options(
    request: &SolveFrame2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveFrame2dResult, String> {
    solve_frame_2d_internal(Cow::Borrowed(request), options)
}

fn solve_frame_2d_internal(
    request: Cow<'_, SolveFrame2dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveFrame2dResult, String> {
    validate_frame_2d_request(request.as_ref())?;
    let displacements = solve_system(
        request.nodes.iter().map(|n| {
            (
                [n.load_x, n.load_y, n.moment_z],
                [n.fix_x, n.fix_y, n.fix_rz],
            )
        }),
        request
            .elements
            .iter()
            .map(|e| FrameElement::mechanical(&request, e)),
        options,
    )?;
    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let [ux, uy, rz] = std::array::from_fn(|dof| displacements[index * 3 + dof]);
            let displacement_magnitude = ux.hypot(uy);
            finite_fields(
                &node.id,
                "displacements",
                &[ux, uy, rz, displacement_magnitude],
            )?;
            Ok(Frame2dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                rz,
                displacement_magnitude,
            })
        }),
    )?;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, e)| {
            let kernel = FrameElement::mechanical(&request, e)?;
            let r = kernel.recover(&displacements)?;
            Ok(Frame2dElementResult {
                index,
                id: e.id.clone(),
                node_i: e.node_i,
                node_j: e.node_j,
                length: kernel.length,
                axial_force_i: r.forces[0],
                shear_force_i: r.forces[1],
                moment_i: r.forces[2],
                axial_force_j: r.forces[3],
                shear_force_j: r.forces[4],
                moment_j: r.forces[5],
                axial_stress: r.axial_stress,
                max_bending_stress: r.bending_stress,
                max_combined_stress: r.combined_stress,
                strain_energy: r.energy,
            })
        }),
    )?;
    let max_displacement = max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
        n.displacement_magnitude
    })?;
    let max_rotation = max_results(SolverStage::ResultNodeSummary, &nodes, |n| n.rz.abs())?;
    let max_moment = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.moment_i.abs().max(e.moment_j.abs())
    })?;
    let max_stress = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.max_combined_stress
    })?;
    let total_strain_energy = total_energy(elements.iter().map(|e| e.strain_energy))?;
    Ok(SolveFrame2dResult {
        input: request.into_owned(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        total_strain_energy,
    })
}

pub fn solve_thermal_frame_2d(
    request: &SolveThermalFrame2dRequest,
) -> Result<SolveThermalFrame2dResult, String> {
    solve_thermal_frame_2d_internal(
        Cow::Borrowed(request),
        default_thermal_frame_options(request.nodes.len()),
    )
}

pub fn solve_thermal_frame_2d_owned(
    request: SolveThermalFrame2dRequest,
) -> Result<SolveThermalFrame2dResult, String> {
    let options = default_thermal_frame_options(request.nodes.len());
    solve_thermal_frame_2d_internal(Cow::Owned(request), options)
}

pub fn solve_thermal_frame_2d_with_options(
    request: &SolveThermalFrame2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveThermalFrame2dResult, String> {
    solve_thermal_frame_2d_internal(Cow::Borrowed(request), options)
}

fn solve_thermal_frame_2d_internal(
    request: Cow<'_, SolveThermalFrame2dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveThermalFrame2dResult, String> {
    validate_thermal_frame_2d_request(request.as_ref())?;
    let displacements = solve_system(
        request.nodes.iter().map(|n| {
            (
                [n.load_x, n.load_y, n.moment_z],
                [n.fix_x, n.fix_y, n.fix_rz],
            )
        }),
        request
            .elements
            .iter()
            .map(|e| FrameElement::thermal(&request, e)),
        options,
    )?;
    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let [ux, uy, rz] = std::array::from_fn(|dof| displacements[index * 3 + dof]);
            let displacement_magnitude = ux.hypot(uy);
            finite_fields(
                &node.id,
                "displacements",
                &[ux, uy, rz, displacement_magnitude],
            )?;
            Ok(ThermalFrame2dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                rz,
                displacement_magnitude,
                temperature_delta: node.temperature_delta,
            })
        }),
    )?;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, e)| {
            let kernel = FrameElement::thermal(&request, e)?;
            let r = kernel.recover(&displacements)?;
            Ok(ThermalFrame2dElementResult {
                index,
                id: e.id.clone(),
                node_i: e.node_i,
                node_j: e.node_j,
                length: kernel.length,
                average_temperature_delta: kernel.average_temperature,
                thermal_strain: kernel.thermal_strain,
                mechanical_strain: r.mechanical_strain,
                total_strain: r.total_strain,
                temperature_gradient_y: e.temperature_gradient_y,
                thermal_curvature: kernel.thermal_curvature,
                axial_force_i: r.forces[0],
                shear_force_i: r.forces[1],
                moment_i: r.forces[2],
                axial_force_j: r.forces[3],
                shear_force_j: r.forces[4],
                moment_j: r.forces[5],
                axial_stress: r.axial_stress,
                max_bending_stress: r.bending_stress,
                max_combined_stress: r.combined_stress,
                strain_energy: r.energy,
            })
        }),
    )?;
    let max_displacement = max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
        n.displacement_magnitude
    })?;
    let max_rotation = max_results(SolverStage::ResultNodeSummary, &nodes, |n| n.rz.abs())?;
    let max_moment = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.moment_i.abs().max(e.moment_j.abs())
    })?;
    let max_stress = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.max_combined_stress
    })?;
    let max_axial_force = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.axial_force_i.abs().max(e.axial_force_j.abs())
    })?;
    let max_temperature_delta = max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
        n.temperature_delta.abs()
    })?;
    let max_temperature_gradient =
        max_results(SolverStage::ResultElementSummary, &elements, |e| {
            e.temperature_gradient_y.abs()
        })?;
    let total_strain_energy = total_energy(elements.iter().map(|e| e.strain_energy))?;
    Ok(SolveThermalFrame2dResult {
        input: request.into_owned(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        max_temperature_gradient,
        total_strain_energy,
    })
}

fn total_energy(values: impl ExactSizeIterator<Item = f64>) -> Result<f64, String> {
    let total = fold_results(SolverStage::ResultTotals, values, 0.0, |sum, value| {
        sum + value
    })?;
    finite_fields("summary", "total strain energy", &[total])?;
    Ok(total)
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
