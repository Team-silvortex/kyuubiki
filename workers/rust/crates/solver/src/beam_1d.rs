use crate::beam_1d_element::{BeamElement, finite_fields};
use crate::beam_1d_system::solve_system;
use crate::beam_1d_validation::{validate_beam_1d_request, validate_thermal_beam_1d_request};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::solver_control::SolverStage;
use crate::solver_postprocess::{fold_results, max_results, try_collect_results};
use kyuubiki_protocol::{
    Beam1dElementResult, Beam1dNodeResult, SolveBeam1dRequest, SolveBeam1dResult,
    SolveThermalBeam1dRequest, SolveThermalBeam1dResult, ThermalBeam1dElementResult,
    ThermalBeam1dNodeResult,
};
use std::borrow::Cow;

pub fn solve_beam_1d(request: &SolveBeam1dRequest) -> Result<SolveBeam1dResult, String> {
    solve_beam_1d_internal(Cow::Borrowed(request), SpdSolveOptions::default())
}

pub fn solve_beam_1d_owned(request: SolveBeam1dRequest) -> Result<SolveBeam1dResult, String> {
    solve_beam_1d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_beam_1d_with_options(
    request: &SolveBeam1dRequest,
    options: SpdSolveOptions,
) -> Result<SolveBeam1dResult, String> {
    solve_beam_1d_internal(Cow::Borrowed(request), options)
}

fn solve_beam_1d_internal(
    request: Cow<'_, SolveBeam1dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveBeam1dResult, String> {
    validate_beam_1d_request(request.as_ref())?;
    let displacements = solve_system(
        request
            .nodes
            .iter()
            .map(|n| (n.load_y, n.moment_z, n.fix_y, n.fix_rz)),
        request
            .elements
            .iter()
            .map(|e| BeamElement::mechanical(&request, e)),
        options,
    )?;
    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let uy = displacements[index * 2];
            let rz = displacements[index * 2 + 1];
            finite_fields(&node.id, "displacements", &[uy, rz])?;
            Ok(Beam1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                uy,
                rz,
                displacement_magnitude: uy.abs(),
            })
        }),
    )?;
    let mut max_moment = 0.0_f64;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, element)| {
            let kernel = BeamElement::mechanical(&request, element)?;
            let recovered = kernel.recover(&displacements, element.section_modulus)?;
            max_moment = max_moment.max(recovered.max_moment);
            Ok(Beam1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length: kernel.length,
                shear_force_i: recovered.forces[0],
                moment_i: recovered.forces[1],
                shear_force_j: recovered.forces[2],
                moment_j: recovered.forces[3],
                max_bending_stress: recovered.max_stress,
                strain_energy: recovered.energy,
            })
        }),
    )?;
    let max_displacement = max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
        n.displacement_magnitude
    })?;
    let max_rotation = max_results(SolverStage::ResultNodeSummary, &nodes, |n| n.rz.abs())?;
    let max_stress = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.max_bending_stress
    })?;
    let total_strain_energy = total_energy(elements.iter().map(|e| e.strain_energy))?;
    Ok(SolveBeam1dResult {
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

pub fn solve_thermal_beam_1d(
    request: &SolveThermalBeam1dRequest,
) -> Result<SolveThermalBeam1dResult, String> {
    solve_thermal_beam_1d_internal(Cow::Borrowed(request), SpdSolveOptions::default())
}

pub fn solve_thermal_beam_1d_owned(
    request: SolveThermalBeam1dRequest,
) -> Result<SolveThermalBeam1dResult, String> {
    solve_thermal_beam_1d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_thermal_beam_1d_with_options(
    request: &SolveThermalBeam1dRequest,
    options: SpdSolveOptions,
) -> Result<SolveThermalBeam1dResult, String> {
    solve_thermal_beam_1d_internal(Cow::Borrowed(request), options)
}

fn solve_thermal_beam_1d_internal(
    request: Cow<'_, SolveThermalBeam1dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveThermalBeam1dResult, String> {
    validate_thermal_beam_1d_request(request.as_ref())?;
    let displacements = solve_system(
        request
            .nodes
            .iter()
            .map(|n| (n.load_y, n.moment_z, n.fix_y, n.fix_rz)),
        request
            .elements
            .iter()
            .map(|e| BeamElement::thermal(&request, e)),
        options,
    )?;
    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let uy = displacements[index * 2];
            let rz = displacements[index * 2 + 1];
            finite_fields(&node.id, "displacements", &[uy, rz])?;
            Ok(ThermalBeam1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                uy,
                rz,
                displacement_magnitude: uy.abs(),
            })
        }),
    )?;
    let mut max_moment = 0.0_f64;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, element)| {
            let kernel = BeamElement::thermal(&request, element)?;
            let recovered = kernel.recover(&displacements, element.section_modulus)?;
            max_moment = max_moment.max(recovered.max_moment);
            Ok(ThermalBeam1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length: kernel.length,
                temperature_gradient_y: element.temperature_gradient_y,
                thermal_curvature: kernel.thermal_curvature,
                shear_force_i: recovered.forces[0],
                moment_i: recovered.forces[1],
                shear_force_j: recovered.forces[2],
                moment_j: recovered.forces[3],
                max_bending_stress: recovered.max_stress,
                strain_energy: recovered.energy,
            })
        }),
    )?;
    let max_displacement = max_results(SolverStage::ResultNodeSummary, &nodes, |n| {
        n.displacement_magnitude
    })?;
    let max_rotation = max_results(SolverStage::ResultNodeSummary, &nodes, |n| n.rz.abs())?;
    let max_stress = max_results(SolverStage::ResultElementSummary, &elements, |e| {
        e.max_bending_stress
    })?;
    let max_temperature_gradient =
        max_results(SolverStage::ResultElementSummary, &elements, |e| {
            e.temperature_gradient_y.abs()
        })?;
    let total_strain_energy = total_energy(elements.iter().map(|e| e.strain_energy))?;
    Ok(SolveThermalBeam1dResult {
        input: request.into_owned(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
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
