use crate::{
    chain_tridiagonal::path_forest_order,
    dynamic_spring_1d_validation::validate_harmonic_request,
    harmonic_spring_linear::{
        Complex, ComplexTridiagonalSystem, arithmetic, solve_complex_system,
        solve_complex_tridiagonal,
    },
    solver_control::{SolverStage, checkpoint, checkpoint_chunk},
    solver_postprocess::{fold_results, max_results, try_collect_results},
};
use kyuubiki_protocol::{
    HarmonicSpring1dElementResponse, HarmonicSpring1dFrequencyResult, HarmonicSpring1dNodeResponse,
    SolveHarmonicSpring1dRequest, SolveHarmonicSpring1dResult, TransientSpring1dElementInput,
};
use std::{borrow::Cow, collections::BTreeSet, f64::consts::PI};

const MAX_DENSE_HARMONIC_DOFS: usize = 512;

#[path = "harmonic_spring_constraint_validation.rs"]
mod constraint_validation;
use constraint_validation::validate_harmonic_equilibrium;

#[path = "harmonic_spring_frequency_range.rs"]
mod frequency_range;

#[path = "harmonic_spring_assembly.rs"]
mod assembly;
use assembly::Corrections;

#[cfg(test)]
#[path = "harmonic_spring_constraint_tests.rs"]
mod constraint_tests;

#[cfg(test)]
#[path = "harmonic_spring_assembly_tests.rs"]
mod assembly_tests;

#[cfg(test)]
#[path = "harmonic_spring_inertia_tests.rs"]
mod inertia_tests;

struct ReducedLayout {
    free: Vec<usize>,
    reduced_index: Vec<Option<usize>>,
    path_position: Option<Vec<usize>>,
}

pub fn solve_harmonic_spring_1d(
    request: &SolveHarmonicSpring1dRequest,
) -> Result<SolveHarmonicSpring1dResult, String> {
    solve_harmonic_spring_1d_internal(Cow::Borrowed(request))
}

pub fn solve_harmonic_spring_1d_owned(
    request: SolveHarmonicSpring1dRequest,
) -> Result<SolveHarmonicSpring1dResult, String> {
    solve_harmonic_spring_1d_internal(Cow::Owned(request))
}

fn solve_harmonic_spring_1d_internal(
    request: Cow<'_, SolveHarmonicSpring1dRequest>,
) -> Result<SolveHarmonicSpring1dResult, String> {
    validate_harmonic_request(request.as_ref())?;

    let layout = reduced_layout(request.as_ref())?;

    let frequencies = request
        .frequencies_hz
        .iter()
        .enumerate()
        .map(|(index, &frequency_hz)| {
            checkpoint(SolverStage::HarmonicSweep, index)?;
            solve_frequency(request.as_ref(), frequency_hz, &layout).map_err(|error| {
                format!("harmonic spring 1d frequency {index} ({frequency_hz:.6e} Hz): {error}")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    checkpoint(SolverStage::HarmonicSweep, frequencies.len())?;

    let (max_displacement, max_velocity, max_acceleration, max_force, peak_frequency_hz) =
        fold_results(
            SolverStage::ResultTotals,
            frequencies.iter(),
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
            |(displacement, velocity, acceleration, force, peak), result| {
                (
                    displacement.max(result.max_displacement),
                    velocity.max(result.max_velocity),
                    acceleration.max(result.max_acceleration),
                    force.max(result.max_force),
                    if result.max_displacement >= displacement {
                        result.frequency_hz
                    } else {
                        peak
                    },
                )
            },
        )?;

    Ok(SolveHarmonicSpring1dResult {
        input: request.into_owned(),
        max_displacement,
        max_velocity,
        max_acceleration,
        max_force,
        peak_frequency_hz,
        frequencies,
    })
}

fn reduced_layout(request: &SolveHarmonicSpring1dRequest) -> Result<ReducedLayout, String> {
    checkpoint(SolverStage::ConstraintIndex, 0)?;
    let mut free = Vec::new();
    let mut reduced_index = vec![None; request.nodes.len()];
    for (index, node) in request.nodes.iter().enumerate() {
        if !node.fix_x {
            reduced_index[index] = Some(free.len());
            free.push(index);
        }
        checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, request.nodes.len())?;
    }

    let mut free_edges = BTreeSet::new();
    for (index, element) in request.elements.iter().enumerate() {
        if let (Some(first), Some(second)) =
            (reduced_index[element.node_i], reduced_index[element.node_j])
        {
            free_edges.insert(if first < second {
                (first, second)
            } else {
                (second, first)
            });
        }
        checkpoint_chunk(
            SolverStage::ConstraintMap,
            index + 1,
            request.elements.len(),
        )?;
    }
    let path_order = path_forest_order(free.len(), free_edges.len(), free_edges.iter().copied());
    checkpoint(SolverStage::ConstraintMap, request.elements.len())?;
    if path_order.is_none() && free.len() > MAX_DENSE_HARMONIC_DOFS {
        return Err(format!(
            "harmonic spring 1d non-path network has {} free degrees of freedom; the dense frequency fallback supports at most {MAX_DENSE_HARMONIC_DOFS}",
            free.len()
        ));
    }
    let path_position = if let Some(order) = path_order {
        let mut position = vec![0_usize; order.len()];
        for (path_index, reduced) in order.into_iter().enumerate() {
            position[reduced] = path_index;
            checkpoint_chunk(
                SolverStage::ConstraintReduce,
                path_index + 1,
                position.len(),
            )?;
        }
        Some(position)
    } else {
        None
    };

    Ok(ReducedLayout {
        free,
        reduced_index,
        path_position,
    })
}

fn solve_frequency(
    request: &SolveHarmonicSpring1dRequest,
    frequency_hz: f64,
    layout: &ReducedLayout,
) -> Result<HarmonicSpring1dFrequencyResult, String> {
    let omega = 2.0 * PI * frequency_hz;
    if !omega.is_finite() {
        return Err("harmonic spring 1d angular frequency exceeds the numeric range".to_string());
    }
    let solved = if let Some(path_position) = &layout.path_position {
        let system = assemble_path_dynamic_stiffness(request, omega, layout, path_position)?;
        let ordered = solve_complex_tridiagonal(system)?;
        let mut reduced = vec![Complex::default(); ordered.len()];
        for (reduced_index, &path_index) in path_position.iter().enumerate() {
            reduced[reduced_index] = ordered[path_index];
            checkpoint_chunk(
                SolverStage::ResultFreeDofs,
                reduced_index + 1,
                reduced.len(),
            )?;
        }
        reduced
    } else {
        let (matrix, rhs) = assemble_reduced_dynamic_stiffness(
            request,
            omega,
            &layout.free,
            &layout.reduced_index,
        )?;
        solve_complex_system(matrix, rhs)?
    };
    let mut displacement = vec![Complex::default(); request.nodes.len()];
    for (index, &dof) in layout.free.iter().enumerate() {
        displacement[dof] = solved[index];
        checkpoint_chunk(SolverStage::ResultFreeDofs, index + 1, layout.free.len())?;
    }
    validate_harmonic_equilibrium(request, omega, &displacement, &layout.reduced_index)?;

    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let amplitude = displacement[index].amplitude();
            if !amplitude.is_finite() {
                return Err(format!(
                    "node {index} displacement amplitude exceeds the finite numeric range"
                ));
            }
            let velocity = frequency_range::product(omega, amplitude)
                .map_err(|error| format!("node {index} velocity amplitude {error}"))?;
            let acceleration = frequency_range::square_product(omega, amplitude)
                .map_err(|error| format!("node {index} acceleration amplitude {error}"))?;
            Ok(HarmonicSpring1dNodeResponse {
                index,
                id: node.id.clone(),
                displacement_amplitude: amplitude,
                displacement_phase_deg: displacement[index].phase_deg(),
                velocity_amplitude: velocity,
                acceleration_amplitude: acceleration,
            })
        }),
    )?;
    let elements = harmonic_elements(request, &displacement, omega)?;

    Ok(HarmonicSpring1dFrequencyResult {
        frequency_hz,
        angular_frequency: omega,
        max_displacement: max_results(SolverStage::ResultNodeSummary, &nodes, |node| {
            node.displacement_amplitude
        })?,
        max_velocity: max_results(SolverStage::ResultNodeSummary, &nodes, |node| {
            node.velocity_amplitude
        })?,
        max_acceleration: max_results(SolverStage::ResultNodeSummary, &nodes, |node| {
            node.acceleration_amplitude
        })?,
        max_force: max_results(SolverStage::ResultElementSummary, &elements, |element| {
            element.force_amplitude
        })?,
        nodes,
        elements,
    })
}

fn harmonic_elements(
    request: &SolveHarmonicSpring1dRequest,
    displacement: &[Complex],
    omega: f64,
) -> Result<Vec<HarmonicSpring1dElementResponse>, String> {
    try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, element)| {
            let extension = displacement[element.node_j] - displacement[element.node_i];
            let extension_amplitude = extension.amplitude();
            if !extension_amplitude.is_finite() {
                return Err(format!(
                    "element {index} extension amplitude exceeds the finite numeric range"
                ));
            }
            let force = arithmetic::product(
                extension,
                element_coefficient(element, omega, index)?,
                SolverStage::ResultElements,
            )
            .map_err(|error| format!("element {index} force: {error}"))?;
            let force_amplitude = force.amplitude();
            if !force_amplitude.is_finite() {
                return Err(format!(
                    "element {index} force amplitude exceeds the finite numeric range"
                ));
            }
            Ok(HarmonicSpring1dElementResponse {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                extension_amplitude,
                force_amplitude,
            })
        }),
    )
}

fn assemble_reduced_dynamic_stiffness(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    free: &[usize],
    reduced_index: &[Option<usize>],
) -> Result<(Vec<Vec<Complex>>, Vec<Complex>), String> {
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let mut matrix = vec![vec![Complex::default(); free.len()]; free.len()];
    let mut rhs = vec![Complex::default(); free.len()];
    let mut corrections = Corrections::default();
    for (row, &node) in free.iter().enumerate() {
        let inertia = inertia_coefficient(omega, request.nodes[node].mass, node)?;
        corrections.add_scaled_real(
            row * free.len() + row,
            &mut matrix[row][row],
            inertia.parts.map(|part| -part),
            inertia.exponent,
        )?;
        rhs[row] = Complex::real(request.nodes[node].load_x);
        checkpoint_chunk(SolverStage::ConstraintReduce, row + 1, free.len())?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = element_coefficient(element, omega, index)?;
        let first = reduced_index[element.node_i];
        let second = reduced_index[element.node_j];
        if let Some(first) = first {
            corrections.add(
                first * free.len() + first,
                &mut matrix[first][first],
                coefficient,
            )?;
        }
        if let Some(second) = second {
            corrections.add(
                second * free.len() + second,
                &mut matrix[second][second],
                coefficient,
            )?;
        }
        if let (Some(first), Some(second)) = (first, second) {
            let negative = Complex {
                re: -coefficient.re,
                im: -coefficient.im,
            };
            corrections.add(
                first * free.len() + second,
                &mut matrix[first][second],
                negative,
            )?;
            corrections.add(
                second * free.len() + first,
                &mut matrix[second][first],
                negative,
            )?;
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }
    corrections.finish(matrix.iter_mut().flatten(), request.elements.len())?;
    Ok((matrix, rhs))
}

fn assemble_path_dynamic_stiffness(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    layout: &ReducedLayout,
    path_position: &[usize],
) -> Result<ComplexTridiagonalSystem, String> {
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let size = layout.free.len();
    let mut diagonal = vec![Complex::default(); size];
    let mut lower = vec![Complex::default(); size.saturating_sub(1)];
    let mut upper = vec![Complex::default(); size.saturating_sub(1)];
    let mut rhs = vec![Complex::default(); size];
    let mut corrections = Corrections::default();
    for (reduced, &node) in layout.free.iter().enumerate() {
        let row = path_position[reduced];
        let inertia = inertia_coefficient(omega, request.nodes[node].mass, node)?;
        corrections.add_scaled_real(
            row,
            &mut diagonal[row],
            inertia.parts.map(|part| -part),
            inertia.exponent,
        )?;
        rhs[row] = Complex::real(request.nodes[node].load_x);
        checkpoint_chunk(SolverStage::ConstraintReduce, reduced + 1, size)?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = element_coefficient(element, omega, index)?;
        let first = layout.reduced_index[element.node_i];
        let second = layout.reduced_index[element.node_j];
        if let Some(first) = first {
            let row = path_position[first];
            corrections.add(row, &mut diagonal[row], coefficient)?;
        }
        if let Some(second) = second {
            let row = path_position[second];
            corrections.add(row, &mut diagonal[row], coefficient)?;
        }
        if let (Some(first), Some(second)) = (first, second) {
            let first = path_position[first];
            let second = path_position[second];
            let left = first.min(second);
            if first.abs_diff(second) != 1 {
                return Err(
                    "harmonic spring 1d path layout is inconsistent with its topology".to_string(),
                );
            }
            let negative = Complex {
                re: -coefficient.re,
                im: -coefficient.im,
            };
            corrections.add(size + lower.len() + left, &mut upper[left], negative)?;
            corrections.add(size + left, &mut lower[left], negative)?;
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }
    corrections.finish(
        diagonal.iter_mut().chain(&mut lower).chain(&mut upper),
        request.elements.len(),
    )?;
    Ok(ComplexTridiagonalSystem {
        diagonal,
        lower,
        upper,
        rhs,
    })
}

fn inertia_coefficient(
    omega: f64,
    mass: f64,
    node: usize,
) -> Result<frequency_range::Inertia, String> {
    frequency_range::inertia_parts(omega, mass)
        .map_err(|error| format!("node {node} inertia coefficient {error}"))
}

fn element_coefficient(
    element: &TransientSpring1dElementInput,
    omega: f64,
    index: usize,
) -> Result<Complex, String> {
    Ok(Complex {
        re: element.stiffness,
        im: frequency_range::product(omega, element.damping)
            .map_err(|error| format!("element {index} damping coefficient {error}"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn residual_validation_does_not_erase_an_independent_soft_row() {
        let request: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
            "nodes": [
                {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0, "mass":1.0},
                {"id":"soft", "x":1.0, "fix_x":false, "load_x":1e-200, "mass":1.0},
                {"id":"stiff", "x":2.0, "fix_x":false, "load_x":1e200, "mass":1.0}
            ],
            "elements": [
                {"id":"soft", "node_i":0, "node_j":1, "stiffness":1e-200},
                {"id":"stiff", "node_i":0, "node_j":2, "stiffness":1e200}
            ], "frequencies_hz":[0.0]
        }))
        .unwrap();
        let mut displacement = [Complex::real(0.0), Complex::real(1.0), Complex::real(1.0)];
        let reduced = [None, Some(0), Some(1)];
        validate_harmonic_equilibrium(&request, 0.0, &displacement, &reduced).unwrap();
        displacement[1] = Complex::real(2.0);
        let error =
            validate_harmonic_equilibrium(&request, 0.0, &displacement, &reduced).unwrap_err();
        assert!(
            error.contains("backward-error") && error.contains("free row 0"),
            "{error}"
        );
    }
}
