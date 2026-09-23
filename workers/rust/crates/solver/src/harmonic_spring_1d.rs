use crate::{
    chain_tridiagonal::path_forest_order,
    dynamic_spring_1d_validation::validate_harmonic_request,
    harmonic_spring_linear::{
        Complex, ComplexTridiagonalSystem, solve_complex_system, solve_complex_tridiagonal,
    },
    solver_control::{SolverStage, checkpoint, checkpoint_chunk},
    solver_postprocess::{fold_results, max_results, try_collect_results},
};
use kyuubiki_protocol::{
    HarmonicSpring1dElementResponse, HarmonicSpring1dFrequencyResult, HarmonicSpring1dNodeResponse,
    SolveHarmonicSpring1dRequest, SolveHarmonicSpring1dResult,
};
use std::{borrow::Cow, collections::BTreeSet, f64::consts::PI};

const MAX_DENSE_HARMONIC_DOFS: usize = 512;
const MIN_COMPLEX_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-12;
const MAX_COMPLEX_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-8;

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
    let omega_squared = omega * omega;
    if !(omega.is_finite() && omega_squared.is_finite()) {
        return Err("harmonic spring 1d angular frequency exceeds the numeric range".to_string());
    }
    let solved = if let Some(path_position) = &layout.path_position {
        let system =
            assemble_path_dynamic_stiffness(request, omega, omega_squared, layout, path_position)?;
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
            omega_squared,
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
    validate_harmonic_equilibrium(
        request,
        omega,
        omega_squared,
        &displacement,
        &layout.reduced_index,
    )?;

    let nodes = try_collect_results(
        SolverStage::ResultNodes,
        request.nodes.iter().enumerate().map(|(index, node)| {
            let amplitude = displacement[index].amplitude();
            let velocity = omega * amplitude;
            let acceleration = omega_squared * amplitude;
            for (field, value) in [
                ("displacement", amplitude),
                ("velocity", velocity),
                ("acceleration", acceleration),
            ] {
                if !value.is_finite() {
                    return Err(format!(
                        "node {index} {field} amplitude exceeds the finite numeric range"
                    ));
                }
            }
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
            let force = extension
                * Complex {
                    re: element.stiffness,
                    im: omega * element.damping,
                };
            let extension_amplitude = extension.amplitude();
            let force_amplitude = force.amplitude();
            if !(extension_amplitude.is_finite() && force_amplitude.is_finite()) {
                return Err(format!(
                    "element {index} extension or force amplitude exceeds the finite numeric range"
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
    omega_squared: f64,
    free: &[usize],
    reduced_index: &[Option<usize>],
) -> Result<(Vec<Vec<Complex>>, Vec<Complex>), String> {
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let mut matrix = vec![vec![Complex::default(); free.len()]; free.len()];
    let mut rhs = vec![Complex::default(); free.len()];
    for (row, &node) in free.iter().enumerate() {
        matrix[row][row].re = -omega_squared * request.nodes[node].mass;
        rhs[row] = Complex::real(request.nodes[node].load_x);
        checkpoint_chunk(SolverStage::ConstraintReduce, row + 1, free.len())?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = Complex {
            re: element.stiffness,
            im: omega * element.damping,
        };
        let first = reduced_index[element.node_i];
        let second = reduced_index[element.node_j];
        if let Some(first) = first {
            matrix[first][first] = matrix[first][first] + coefficient;
        }
        if let Some(second) = second {
            matrix[second][second] = matrix[second][second] + coefficient;
        }
        if let (Some(first), Some(second)) = (first, second) {
            matrix[first][second] = matrix[first][second] - coefficient;
            matrix[second][first] = matrix[second][first] - coefficient;
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }
    Ok((matrix, rhs))
}

fn assemble_path_dynamic_stiffness(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    omega_squared: f64,
    layout: &ReducedLayout,
    path_position: &[usize],
) -> Result<ComplexTridiagonalSystem, String> {
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let size = layout.free.len();
    let mut diagonal = vec![Complex::default(); size];
    let mut lower = vec![Complex::default(); size.saturating_sub(1)];
    let mut upper = vec![Complex::default(); size.saturating_sub(1)];
    let mut rhs = vec![Complex::default(); size];
    for (reduced, &node) in layout.free.iter().enumerate() {
        let row = path_position[reduced];
        diagonal[row].re = -omega_squared * request.nodes[node].mass;
        rhs[row] = Complex::real(request.nodes[node].load_x);
        checkpoint_chunk(SolverStage::ConstraintReduce, reduced + 1, size)?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = Complex {
            re: element.stiffness,
            im: omega * element.damping,
        };
        let first = layout.reduced_index[element.node_i];
        let second = layout.reduced_index[element.node_j];
        if let Some(first) = first {
            diagonal[path_position[first]] = diagonal[path_position[first]] + coefficient;
        }
        if let Some(second) = second {
            diagonal[path_position[second]] = diagonal[path_position[second]] + coefficient;
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
            upper[left] = upper[left] - coefficient;
            lower[left] = lower[left] - coefficient;
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }
    Ok(ComplexTridiagonalSystem {
        diagonal,
        lower,
        upper,
        rhs,
    })
}

fn validate_harmonic_equilibrium(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    omega_squared: f64,
    displacement: &[Complex],
    reduced_index: &[Option<usize>],
) -> Result<(), String> {
    checkpoint(SolverStage::HarmonicResidual, 0)?;
    let free_count = reduced_index.iter().flatten().count();
    let mut coefficient_scales = vec![0.0_f64; free_count];
    let mut displacement_scales = vec![0.0_f64; free_count];
    for (index, node) in request.nodes.iter().enumerate() {
        if let Some(row) = reduced_index[index] {
            coefficient_scales[row] = omega_squared * node.mass;
            displacement_scales[row] = displacement[index].component_scale();
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.nodes.len(),
        )?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = Complex {
            re: element.stiffness,
            im: omega * element.damping,
        };
        let local_displacement = displacement[element.node_i]
            .component_scale()
            .max(displacement[element.node_j].component_scale());
        for row in [reduced_index[element.node_i], reduced_index[element.node_j]]
            .into_iter()
            .flatten()
        {
            coefficient_scales[row] = coefficient_scales[row].max(coefficient.component_scale());
            displacement_scales[row] = displacement_scales[row].max(local_displacement);
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.elements.len(),
        )?;
    }

    // Normalize each equation and its incident displacements, not the whole model.
    // This avoids overflowing the error denominator or erasing an independent soft row.
    let mut residuals = vec![Complex::default(); free_count];
    let mut equation_scales = vec![0.0_f64; free_count];
    for (index, node) in request.nodes.iter().enumerate() {
        if let Some(row) = reduced_index[index] {
            let coefficient_scale = coefficient_scales[row];
            if !(coefficient_scale.is_finite() && coefficient_scale > 0.0) {
                return Err(format!("node {index} dynamic stiffness has invalid scale"));
            }
            if displacement_scales[row] == 0.0 {
                displacement_scales[row] = 1.0;
            }
            let displacement_scale = displacement_scales[row];
            let rhs = normalized_load(node.load_x, coefficient_scale, displacement_scale)?;
            let inertia = Complex::real(omega_squared * node.mass / coefficient_scale)
                * displacement[index].scaled(displacement_scale);
            residuals[row] = Complex::real(rhs) + inertia;
            equation_scales[row] = rhs.abs() + inertia.amplitude();
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.nodes.len(),
        )?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = Complex {
            re: element.stiffness,
            im: omega * element.damping,
        };
        for (node, sign) in [(element.node_i, 1.0), (element.node_j, -1.0)] {
            if let Some(row) = reduced_index[node] {
                let coefficient = coefficient.scaled(coefficient_scales[row]);
                let first = displacement[element.node_i].scaled(displacement_scales[row]);
                let second = displacement[element.node_j].scaled(displacement_scales[row]);
                let force = coefficient * (second - first);
                residuals[row] = residuals[row] + Complex::real(sign) * force;
                equation_scales[row] +=
                    coefficient.amplitude() * (first.amplitude() + second.amplitude());
            }
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.elements.len(),
        )?;
    }

    let tolerance = (128.0 * f64::EPSILON * free_count as f64).clamp(
        MIN_COMPLEX_BACKWARD_ERROR_TOLERANCE,
        MAX_COMPLEX_BACKWARD_ERROR_TOLERANCE,
    );
    for (row, (residual, equation_scale)) in residuals.into_iter().zip(equation_scales).enumerate()
    {
        if !(residual.is_finite() && equation_scale.is_finite()) {
            return Err("harmonic spring 1d dynamic stiffness residual is non-finite".to_string());
        }
        let relative = if equation_scale == 0.0 {
            0.0
        } else {
            residual.amplitude() / equation_scale
        };
        if !relative.is_finite() || relative > tolerance {
            return Err(format!(
                "harmonic spring 1d dynamic stiffness failed backward-error validation at free row {row} ({relative:.6e})"
            ));
        }
        checkpoint_chunk(SolverStage::HarmonicResidual, row + 1, free_count)?;
    }
    Ok(())
}

fn normalized_load(
    load: f64,
    coefficient_scale: f64,
    displacement_scale: f64,
) -> Result<f64, String> {
    let intermediate = load / coefficient_scale;
    let normalized = if !intermediate.is_finite() || (load != 0.0 && intermediate == 0.0) {
        (load / displacement_scale) / coefficient_scale
    } else {
        intermediate / displacement_scale
    };
    if !normalized.is_finite() || (load != 0.0 && normalized == 0.0) {
        return Err(
            "harmonic spring 1d load cannot be represented in residual normalization".to_string(),
        );
    }
    Ok(normalized)
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
        validate_harmonic_equilibrium(&request, 0.0, 0.0, &displacement, &reduced).unwrap();
        displacement[1] = Complex::real(2.0);
        let error =
            validate_harmonic_equilibrium(&request, 0.0, 0.0, &displacement, &reduced).unwrap_err();
        assert!(
            error.contains("backward-error") && error.contains("free row 0"),
            "{error}"
        );
    }
}
