use crate::linear_algebra::{SparseMatrix, add_at};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};
use crate::solver_control::{SolverStage, checkpoint};
use crate::thermal_plane_2d_results::build_thermal_plane_nodes;
use crate::thermal_plane_2d_solve::solve_thermal_plane_displacements;
use crate::{thermal_q9_element as element, thermal_q9_validation as validation};
use kyuubiki_protocol::{
    SolveThermalQ9RectRequest, SolveThermalQ9RectResult, THERMAL_Q9_RECT_SCHEMA,
    ThermalPlaneNodeInput,
};

/// Validate a Q9 model without assembly or solving, for package-owned input bridges.
pub fn validate_thermal_q9_rect_model(request: &SolveThermalQ9RectRequest) -> Result<(), String> {
    validation::validate(request).map(|_| ())
}

/// Bounded conforming axis-aligned Q9 plane stress with supplied nodal temperature deltas.
pub fn solve_thermal_q9_rect(
    request: &SolveThermalQ9RectRequest,
) -> Result<SolveThermalQ9RectResult, String> {
    checkpoint(SolverStage::ElementPrecompute, 0)?;
    let rects = validation::validate(request)?;
    let mut matrix = SparseMatrix::with_uniform_row_capacity(2 * request.nodes.len(), 50);
    let mut force: Vec<_> = request
        .nodes
        .iter()
        .flat_map(|n| [n.load_x, n.load_y])
        .collect();
    for (index, e) in request.elements.iter().enumerate() {
        checkpoint(SolverStage::ElementAssembly, index)?;
        let points = element::points(request, e, rects[index])?;
        let (k, f) = element::stiffness(e, &points)?;
        let dofs: [usize; 18] = std::array::from_fn(|i| 2 * e.nodes[i / 2] + i % 2);
        for i in 0..18 {
            force[dofs[i]] += f[i];
            for j in 0..18 {
                add_at(&mut matrix, dofs[i], dofs[j], k[i][j]);
            }
        }
    }
    if force.iter().any(|v| !v.is_finite()) {
        return Err("Q9 assembled load is not representable".into());
    }
    let nodes: Vec<_> = request
        .nodes
        .iter()
        .map(|n| ThermalPlaneNodeInput {
            id: n.id.clone(),
            x: n.x,
            y: n.y,
            fix_x: n.fix_x,
            fix_y: n.fix_y,
            load_x: n.load_x,
            load_y: n.load_y,
            temperature_delta: n.temperature_delta,
        })
        .collect();
    let solved = solve_thermal_plane_displacements(
        &nodes,
        &matrix,
        &force,
        SpdSolveOptions {
            preconditioner: SpdPreconditioner::IncompleteCholesky,
            progress_interval: None,
        },
        &mut vec![],
        false,
    )?;
    let mut elements = Vec::with_capacity(request.elements.len());
    let mut total = 0.0;
    for (i, e) in request.elements.iter().enumerate() {
        checkpoint(SolverStage::ResultElements, i)?;
        let points = element::points(request, e, rects[i])?;
        let result = element::result(e, rects[i], &points, &solved.displacements)?;
        total += result.strain_energy_j;
        elements.push(result);
    }
    if !total.is_finite() {
        return Err("Q9 total energy is not representable".into());
    }
    Ok(SolveThermalQ9RectResult {
        schema_version: THERMAL_Q9_RECT_SCHEMA.into(),
        nodes: build_thermal_plane_nodes(&nodes, &solved.displacements)?,
        elements,
        total_strain_energy_j: total,
        solver_iterations: solved.solver_iterations,
        solver_matrix_non_zero_count: solved.solver_matrix_non_zero_count,
        solver_residual_norm: solved.solver_residual_norm,
        solver_rhs_norm: solved.solver_rhs_norm,
    })
}
