use crate::solver_control::SolverStage;
use crate::solver_postprocess::{max_results, sum_strain_energy};
use crate::truss_numerics::validate_small_displacement;
use kyuubiki_protocol::{
    SolveTruss2dRequest, SolveTruss3dRequest, Truss3dElementResult, Truss3dNodeResult,
    TrussElementResult, TrussNodeResult,
};

pub(super) fn max_truss_2d_displacement(nodes: &[TrussNodeResult]) -> Result<f64, String> {
    max_results(SolverStage::ResultNodeSummary, nodes, |node| {
        node.ux.hypot(node.uy)
    })
}

pub(super) fn max_truss_3d_displacement(nodes: &[Truss3dNodeResult]) -> Result<f64, String> {
    max_results(SolverStage::ResultNodeSummary, nodes, |node| {
        node.ux.hypot(node.uy).hypot(node.uz)
    })
}

pub(super) fn max_truss_stress(elements: &[TrussElementResult]) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.stress.abs()
    })
}

pub(super) fn max_truss_3d_stress(elements: &[Truss3dElementResult]) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.stress.abs()
    })
}

pub(super) fn total_truss_2d_strain_energy(
    request: &SolveTruss2dRequest,
    elements: &[TrussElementResult],
) -> Result<f64, String> {
    sum_strain_energy(
        "truss",
        elements
            .iter()
            .zip(&request.elements)
            .map(|(element, input)| {
                (
                    element.id.as_str(),
                    element.strain_energy_density,
                    input.area,
                    element.length,
                )
            }),
    )
}

pub(super) fn total_truss_3d_strain_energy(
    request: &SolveTruss3dRequest,
    elements: &[Truss3dElementResult],
) -> Result<f64, String> {
    sum_strain_energy(
        "3d truss",
        elements
            .iter()
            .zip(&request.elements)
            .map(|(element, input)| {
                (
                    element.id.as_str(),
                    element.strain_energy_density,
                    input.area,
                    element.length,
                )
            }),
    )
}

pub(super) fn max_truss_strain_energy_density(
    elements: &[TrussElementResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.strain_energy_density
    })
}

pub(super) fn max_truss_3d_strain_energy_density(
    elements: &[Truss3dElementResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.strain_energy_density
    })
}

pub(super) fn validate_small_displacement_truss(
    request: &SolveTruss2dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    validate_small_displacement(
        "truss",
        request.nodes.iter().map(|node| [node.x, node.y]),
        max_displacement,
    )
}

pub(super) fn validate_small_displacement_truss_3d(
    request: &SolveTruss3dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    validate_small_displacement(
        "3d truss",
        request.nodes.iter().map(|node| [node.x, node.y, node.z]),
        max_displacement,
    )
}
