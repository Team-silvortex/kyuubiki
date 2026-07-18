use crate::chain_tridiagonal::{is_indexed_chain, solve_with_prescribed};
use crate::linear_dense::{solve_linear_system, zero_matrix};
use crate::transport_bar_1d_validation::validate_request;
use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementResult, AdvectionDiffusionBar1dNodeResult,
    SolveAdvectionDiffusionBar1dRequest, SolveAdvectionDiffusionBar1dResult,
};

pub fn solve_advection_diffusion_bar_1d(
    request: &SolveAdvectionDiffusionBar1dRequest,
) -> Result<SolveAdvectionDiffusionBar1dResult, String> {
    validate_request(request)?;

    let concentrations = solve_concentrations(request)?;

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| AdvectionDiffusionBar1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            concentration: concentrations[index],
            source: node.source,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let average_concentration =
                0.5 * (concentrations[element.node_i] + concentrations[element.node_j]);
            let concentration_gradient =
                (concentrations[element.node_j] - concentrations[element.node_i]) / length;
            let diffusive_flux = -element.diffusivity * concentration_gradient;
            let advective_flux = element.velocity * average_concentration;
            let total_flux = diffusive_flux + advective_flux;
            let peclet_number = element.velocity.abs() * length / (2.0 * element.diffusivity);

            AdvectionDiffusionBar1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_concentration,
                concentration_gradient,
                diffusive_flux,
                advective_flux,
                total_flux,
                peclet_number,
            }
        })
        .collect::<Vec<_>>();

    let max_concentration = nodes
        .iter()
        .map(|node| node.concentration.abs())
        .fold(0.0_f64, f64::max);
    let max_total_flux = elements
        .iter()
        .map(|element| element.total_flux.abs())
        .fold(0.0_f64, f64::max);
    let max_peclet_number = elements
        .iter()
        .map(|element| element.peclet_number)
        .fold(0.0_f64, f64::max);

    Ok(SolveAdvectionDiffusionBar1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_concentration,
        max_total_flux,
        max_peclet_number,
    })
}

fn solve_concentrations(request: &SolveAdvectionDiffusionBar1dRequest) -> Result<Vec<f64>, String> {
    let size = request.nodes.len();
    let rhs = request
        .nodes
        .iter()
        .map(|node| node.source)
        .collect::<Vec<_>>();
    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            node.fix_concentration
                .then_some((index, node.concentration))
        })
        .collect::<Vec<_>>();

    if is_indexed_chain(
        size,
        request
            .elements
            .iter()
            .map(|element| (element.node_i, element.node_j)),
    ) {
        let mut diagonal = vec![0.0; size];
        let mut lower = vec![0.0; size - 1];
        let mut upper = vec![0.0; size - 1];
        for element in &request.elements {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let diffusion = element.diffusivity * element.area / length;
            let advection = element.velocity * element.area * 0.5;
            let local = [
                [diffusion - advection, -diffusion + advection],
                [-diffusion - advection, diffusion + advection],
            ];
            let map = [element.node_i, element.node_j];
            for row in 0..2 {
                for column in 0..2 {
                    let row_index = map[row];
                    let column_index = map[column];
                    if row_index == column_index {
                        diagonal[row_index] += local[row][column];
                    } else {
                        let left = row_index.min(column_index);
                        if row_index == left {
                            upper[left] += local[row][column];
                        } else {
                            lower[left] += local[row][column];
                        }
                    }
                }
            }
        }
        return solve_with_prescribed(&diagonal, &lower, &upper, &rhs, &prescribed);
    }

    let mut matrix = zero_matrix(size);
    let mut adjusted_rhs = rhs;
    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let diffusion = element.diffusivity * element.area / length;
        let advection = element.velocity * element.area * 0.5;
        let local = [
            [diffusion - advection, -diffusion + advection],
            [-diffusion - advection, diffusion + advection],
        ];
        let map = [element.node_i, element.node_j];
        for row in 0..2 {
            for column in 0..2 {
                matrix[map[row]][map[column]] += local[row][column];
            }
        }
    }
    apply_prescribed_concentrations(request, &mut matrix, &mut adjusted_rhs);
    solve_linear_system(matrix, adjusted_rhs)
}

fn apply_prescribed_concentrations(
    request: &SolveAdvectionDiffusionBar1dRequest,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) {
    for (fixed_index, node) in request.nodes.iter().enumerate() {
        if !node.fix_concentration {
            continue;
        }

        for row in 0..matrix.len() {
            if row != fixed_index {
                rhs[row] -= matrix[row][fixed_index] * node.concentration;
                matrix[row][fixed_index] = 0.0;
            }
        }

        matrix[fixed_index].fill(0.0);
        matrix[fixed_index][fixed_index] = 1.0;
        rhs[fixed_index] = node.concentration;
    }
}
