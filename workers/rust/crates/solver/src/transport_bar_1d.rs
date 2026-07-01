use crate::linear_dense::{solve_linear_system, zero_matrix};
use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementResult, AdvectionDiffusionBar1dNodeResult,
    SolveAdvectionDiffusionBar1dRequest, SolveAdvectionDiffusionBar1dResult,
};

pub fn solve_advection_diffusion_bar_1d(
    request: &SolveAdvectionDiffusionBar1dRequest,
) -> Result<SolveAdvectionDiffusionBar1dResult, String> {
    validate_request(request)?;

    let size = request.nodes.len();
    let mut matrix = zero_matrix(size);
    let mut rhs = request
        .nodes
        .iter()
        .map(|node| node.source)
        .collect::<Vec<_>>();

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

    apply_prescribed_concentrations(request, &mut matrix, &mut rhs);
    let concentrations = solve_linear_system(matrix, rhs)?;

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

fn validate_request(request: &SolveAdvectionDiffusionBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d advection-diffusion model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d advection-diffusion model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_concentration) {
        return Err(
            "1d advection-diffusion model must include at least one concentration support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !node.x.is_finite() || !node.concentration.is_finite() || !node.source.is_finite() {
            return Err("1d advection-diffusion node values must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err(
                "1d advection-diffusion element references an out-of-range node".to_string(),
            );
        }
        if element.node_i == element.node_j {
            return Err(
                "1d advection-diffusion element must connect two distinct nodes".to_string(),
            );
        }
        if !element.area.is_finite() || element.area <= 0.0 {
            return Err("1d advection-diffusion element area must be positive".to_string());
        }
        if !element.diffusivity.is_finite() || element.diffusivity <= 0.0 {
            return Err("1d advection-diffusion element diffusivity must be positive".to_string());
        }
        if !element.velocity.is_finite() {
            return Err("1d advection-diffusion element velocity must be finite".to_string());
        }

        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        if length <= 1.0e-12 {
            return Err("1d advection-diffusion element length must be positive".to_string());
        }
    }

    Ok(())
}
