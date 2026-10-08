use crate::chain_tridiagonal::solve_path_with_prescribed;
use crate::linear_dense::{solve_linear_system, zero_matrix};
use crate::solver_control::SolverStage;
use crate::solver_postprocess::{collect_results, max_results, try_collect_results};
use crate::transport_bar_1d_numerics::{
    average, checked, gradient, local_matrix, product_ratio, upwind_stabilization,
};
use crate::transport_bar_1d_validation::validate_request;
use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementResult, AdvectionDiffusionBar1dNodeResult,
    AdvectionDiffusionBar1dScheme, SolveAdvectionDiffusionBar1dRequest,
    SolveAdvectionDiffusionBar1dResult,
};

const MAX_DENSE_NETWORK_NODES: usize = 512;

pub fn solve_advection_diffusion_bar_1d(
    request: &SolveAdvectionDiffusionBar1dRequest,
) -> Result<SolveAdvectionDiffusionBar1dResult, String> {
    validate_request(request)?;
    solve_validated_advection_diffusion_bar_1d(request.clone())
}

pub fn solve_advection_diffusion_bar_1d_owned(
    request: SolveAdvectionDiffusionBar1dRequest,
) -> Result<SolveAdvectionDiffusionBar1dResult, String> {
    validate_request(&request)?;
    solve_validated_advection_diffusion_bar_1d(request)
}

fn solve_validated_advection_diffusion_bar_1d(
    request: SolveAdvectionDiffusionBar1dRequest,
) -> Result<SolveAdvectionDiffusionBar1dResult, String> {
    let concentrations = solve_concentrations(&request)?;

    let nodes =
        collect_results(
            SolverStage::ResultNodes,
            request.nodes.iter().enumerate().map(|(index, node)| {
                AdvectionDiffusionBar1dNodeResult {
                    index,
                    id: node.id.clone(),
                    x: node.x,
                    concentration: concentrations[index],
                    source: node.source,
                }
            }),
        )?;

    let elements = try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let signed_length = node_j.x - node_i.x;
            let length = signed_length.abs();
            let average_concentration = average(
                concentrations[element.node_i],
                concentrations[element.node_j],
            );
            let concentration_gradient = gradient(
                &element.id,
                concentrations[element.node_i],
                concentrations[element.node_j],
                signed_length,
            )?;
            let diffusive_flux = product_ratio(
                &element.id,
                "diffusive flux",
                -element.diffusivity,
                concentration_gradient,
                1.0,
            )?;
            let advective_flux = product_ratio(
                &element.id,
                "advective flux",
                element.velocity,
                average_concentration,
                1.0,
            )?;
            let total_flux = checked(&element.id, "total flux", diffusive_flux + advective_flux)?;
            let peclet_number = product_ratio(
                &element.id,
                "Peclet number",
                element.velocity.abs(),
                length * 0.5,
                element.diffusivity,
            )?;

            Ok(AdvectionDiffusionBar1dElementResult {
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
                stabilization: if request.scheme == AdvectionDiffusionBar1dScheme::Upwind {
                    Some(upwind_stabilization(
                        element,
                        signed_length,
                        [
                            concentrations[element.node_i],
                            concentrations[element.node_j],
                        ],
                        concentration_gradient,
                        diffusive_flux,
                    )?)
                } else {
                    None
                },
            })
        }),
    )?;

    let max_concentration = max_results(SolverStage::ResultNodeSummary, &nodes, |node| {
        node.concentration.abs()
    })?;
    let max_total_flux = max_results(SolverStage::ResultElementSummary, &elements, |element| {
        element.total_flux.abs()
    })?;
    let max_peclet_number = max_results(SolverStage::ResultElementSummary, &elements, |element| {
        element.peclet_number
    })?;
    let max_numerical_flux = if request.scheme == AdvectionDiffusionBar1dScheme::Upwind {
        Some(max_results(
            SolverStage::ResultElementSummary,
            &elements,
            |element| {
                element
                    .stabilization
                    .as_ref()
                    .expect("upwind element diagnostics")
                    .numerical_flux
                    .abs()
            },
        )?)
    } else {
        None
    };

    Ok(SolveAdvectionDiffusionBar1dResult {
        input: request,
        nodes,
        elements,
        max_concentration,
        max_total_flux,
        max_peclet_number,
        max_numerical_flux,
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

    if let Some(result) = solve_path_with_prescribed(
        size,
        &request.elements,
        |element| (element.node_i, element.node_j),
        |element| local_matrix(request, element),
        &rhs,
        &prescribed,
    ) {
        return result;
    }

    if size > MAX_DENSE_NETWORK_NODES {
        return Err(format!(
            "1d advection-diffusion non-path network has {size} nodes; the dense fallback supports at most {MAX_DENSE_NETWORK_NODES}"
        ));
    }

    let mut matrix = zero_matrix(size);
    let mut adjusted_rhs = rhs;
    for element in &request.elements {
        let local = local_matrix(request, element)?;
        let map = [element.node_i, element.node_j];
        for row in 0..2 {
            for column in 0..2 {
                let entry = &mut matrix[map[row]][map[column]];
                *entry = checked(&element.id, "assembled matrix", *entry + local[row][column])?;
            }
        }
    }
    apply_prescribed_concentrations(request, &mut matrix, &mut adjusted_rhs)?;
    solve_linear_system(matrix, adjusted_rhs)
}

fn apply_prescribed_concentrations(
    request: &SolveAdvectionDiffusionBar1dRequest,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), String> {
    for (fixed_index, node) in request.nodes.iter().enumerate() {
        if !node.fix_concentration {
            continue;
        }

        for row in 0..matrix.len() {
            if !request.nodes[row].fix_concentration {
                rhs[row] = checked(
                    &node.id,
                    "prescribed concentration RHS",
                    (-matrix[row][fixed_index]).mul_add(node.concentration, rhs[row]),
                )?;
                matrix[row][fixed_index] = 0.0;
            }
        }

        matrix[fixed_index].fill(0.0);
        matrix[fixed_index][fixed_index] = 1.0;
        rhs[fixed_index] = node.concentration;
    }
    Ok(())
}
