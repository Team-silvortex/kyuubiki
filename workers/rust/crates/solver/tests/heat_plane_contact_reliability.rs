use kyuubiki_protocol::{
    HeatPlaneContactInput, HeatPlaneContactResult, HeatPlaneNodeInput, HeatPlaneQuadElementInput,
    HeatPlaneTriangleElementInput, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
};
use kyuubiki_solver::{
    SpdPreconditioner, SpdSolveOptions, profile_heat_plane_quad_2d_with_options,
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d,
};

#[derive(Debug)]
struct HeatSnapshot {
    temperatures: Vec<f64>,
    fluxes_x: Vec<f64>,
    contacts: Vec<HeatPlaneContactResult>,
}

fn solve(input: &SolveHeatPlaneQuad2dRequest, triangle: bool) -> Result<HeatSnapshot, String> {
    if triangle {
        let request = SolveHeatPlaneTriangle2dRequest {
            nodes: input.nodes.clone(),
            contact_interfaces: input.contact_interfaces.clone(),
            elements: input
                .elements
                .iter()
                .flat_map(|quad| {
                    [
                        [quad.node_i, quad.node_j, quad.node_k],
                        [quad.node_i, quad.node_k, quad.node_l],
                    ]
                    .into_iter()
                    .enumerate()
                    .map(
                        |(side, [node_i, node_j, node_k])| HeatPlaneTriangleElementInput {
                            id: format!("{}-{side}", quad.id),
                            node_i,
                            node_j,
                            node_k,
                            conductivity: quad.conductivity,
                            thickness: quad.thickness,
                        },
                    )
                })
                .collect(),
        };
        let result = solve_heat_plane_triangle_2d(&request)?;
        Ok(HeatSnapshot {
            temperatures: result.nodes.iter().map(|node| node.temperature).collect(),
            fluxes_x: result
                .elements
                .iter()
                .map(|element| element.heat_flux_x)
                .collect(),
            contacts: result.contact_interfaces,
        })
    } else {
        let result = solve_heat_plane_quad_2d(input)?;
        Ok(HeatSnapshot {
            temperatures: result.nodes.iter().map(|node| node.temperature).collect(),
            fluxes_x: result
                .elements
                .iter()
                .map(|element| element.heat_flux_x)
                .collect(),
            contacts: result.contact_interfaces,
        })
    }
}

fn model() -> SolveHeatPlaneQuad2dRequest {
    serde_json::from_str(include_str!(
        "../../protocol/fixtures/heat-plane-contact-quad.json"
    ))
    .unwrap()
}

fn relative_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        actual.is_finite() && (actual / expected - 1.0).abs() < tolerance,
        "actual={actual:e}, expected={expected:e}"
    );
}

#[test]
fn joint_material_and_contact_conductance_scaling_preserves_solution() {
    for scale in [1e-12, 1e-6, 1.0, 1e6, 1e12] {
        let mut input = model();
        for element in &mut input.elements {
            element.conductivity *= scale;
        }
        input.contact_interfaces[0].thermal_resistance_m2_k_w /= scale;
        let power = 20.0 / 3.5;
        for triangle in [false, true] {
            let result = solve(&input, triangle).unwrap();
            relative_close(result.temperatures[1], 20.0 - power, 1e-10);
            relative_close(result.temperatures[4], 0.5 * power, 1e-10);
            relative_close(result.contacts[0].heat_flow_a_to_b_w / scale, power, 1e-10);
            for flux in &result.fluxes_x {
                relative_close(flux * 0.5 / scale, power, 1e-10);
            }
        }
    }
}

#[test]
fn very_conductive_contact_cannot_succeed_with_inconsistent_heat_flows() {
    for resistance in [1e-6, 1e-9, 1e-12, 1e-15] {
        let mut input = model();
        input.contact_interfaces[0].thermal_resistance_m2_k_w = resistance;
        for triangle in [false, true] {
            match solve(&input, triangle) {
                Ok(result) => {
                    let expected_power = 20.0 / (1.5 + 2.0 * resistance);
                    relative_close(result.contacts[0].heat_flow_a_to_b_w, expected_power, 1e-6);
                    for flux in &result.fluxes_x {
                        relative_close(flux * 0.5, expected_power, 1e-6);
                    }
                }
                Err(error) => assert!(
                    error.contains("thermal contact")
                        || error.contains("residual")
                        || error.contains("singular"),
                    "unexpected failure for R={resistance:e}: {error}"
                ),
            }
        }
    }
}

#[test]
fn interface_sources_and_sinks_enter_balance_once_and_preserve_signed_flow() {
    for power in [-28.0, 4.0, 28.0] {
        let mut input = model();
        for node in [1, 2] {
            input.nodes[node].heat_load = power / 2.0;
        }
        // (T_a-20)/1 + (T_a-T_b)/2 = power; T_b/0.5 = (T_a-T_b)/2.
        let contact_power = (20.0 + power) / 3.5;
        for triangle in [false, true] {
            let result = solve(&input, triangle).unwrap();
            relative_close(result.temperatures[1], 2.5 * contact_power, 1e-10);
            relative_close(result.temperatures[4], 0.5 * contact_power, 1e-10);
            relative_close(result.contacts[0].heat_flow_a_to_b_w, contact_power, 1e-10);
            let first_body_elements = if triangle { 2 } else { 1 };
            for (index, flux) in result.fluxes_x.iter().enumerate() {
                let expected = if index < first_body_elements {
                    contact_power - power
                } else {
                    contact_power
                };
                relative_close(flux * 0.5, expected, 1e-10);
            }
        }
    }
}

fn series_model(bodies: usize) -> SolveHeatPlaneQuad2dRequest {
    let mut input = model();
    input.nodes.clear();
    input.elements.clear();
    input.contact_interfaces.clear();
    for body in 0..bodies {
        let base = body * 4;
        for (corner, [dx, y]) in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
            .into_iter()
            .enumerate()
        {
            let x = body as f64 + dx;
            input.nodes.push(HeatPlaneNodeInput {
                id: format!("{body}-{corner}"),
                x,
                y,
                fix_temperature: x == 0.0 || x == bodies as f64,
                temperature: if x == 0.0 { 20.0 } else { 0.0 },
                heat_load: 0.0,
            });
        }
        input.elements.push(HeatPlaneQuadElementInput {
            id: format!("bulk-{body}"),
            node_i: base,
            node_j: base + 1,
            node_k: base + 2,
            node_l: base + 3,
            thickness: 0.5,
            conductivity: 2.0 + (body % 4) as f64,
        });
        if body + 1 < bodies {
            input.contact_interfaces.push(HeatPlaneContactInput {
                id: format!("contact-{body}"),
                side_a: [base + 1, base + 2],
                side_b: [base + 4, base + 7],
                thermal_resistance_m2_k_w: 0.125 * (1 + body % 3) as f64,
            });
        }
    }
    input
}

#[test]
fn twenty_layer_contact_chain_preserves_temperatures_and_serial_power() {
    for bodies in [2, 3, 8, 20] {
        let input = series_model(bodies);
        let total_resistance = input
            .elements
            .iter()
            .map(|e| 2.0 / e.conductivity)
            .sum::<f64>()
            + input
                .contact_interfaces
                .iter()
                .map(|c| 2.0 * c.thermal_resistance_m2_k_w)
                .sum::<f64>();
        let power = 20.0 / total_resistance;
        let mut expected_temperatures = Vec::new();
        let mut temperature = 20.0;
        for (body, element) in input.elements.iter().enumerate() {
            let next = temperature - power * 2.0 / element.conductivity;
            expected_temperatures.extend([temperature, next, next, temperature]);
            temperature = next;
            if let Some(contact) = input.contact_interfaces.get(body) {
                temperature -= power * 2.0 * contact.thermal_resistance_m2_k_w;
            }
        }
        for triangle in [false, true] {
            for reversed in [false, true] {
                let mut input = input.clone();
                if reversed {
                    input.elements.reverse();
                    input.contact_interfaces.reverse();
                    for contact in &mut input.contact_interfaces {
                        std::mem::swap(&mut contact.side_a, &mut contact.side_b);
                    }
                }
                let result = solve(&input, triangle).unwrap();
                for (actual, expected) in result.temperatures.iter().zip(&expected_temperatures) {
                    assert!((actual - expected).abs() < 1e-9);
                }
                for flux in result.fluxes_x {
                    relative_close(flux * 0.5, power, 1e-9);
                }
                for contact in result.contacts {
                    relative_close(
                        contact.heat_flow_a_to_b_w,
                        if reversed { -power } else { power },
                        1e-9,
                    );
                }
            }
        }
    }
}

fn joined_branches(inputs: &[SolveHeatPlaneQuad2dRequest]) -> SolveHeatPlaneQuad2dRequest {
    let mut result = model();
    result.nodes.clear();
    result.elements.clear();
    result.contact_interfaces.clear();
    for (branch, input) in inputs.iter().enumerate() {
        let offset = result.nodes.len();
        result
            .nodes
            .extend(input.nodes.iter().cloned().map(|mut node| {
                node.id = format!("branch-{branch}-{}", node.id);
                node.y += 2.0 * branch as f64;
                node
            }));
        result
            .elements
            .extend(input.elements.iter().cloned().map(|mut element| {
                element.id = format!("branch-{branch}-{}", element.id);
                element.node_i += offset;
                element.node_j += offset;
                element.node_k += offset;
                element.node_l += offset;
                element
            }));
        result
            .contact_interfaces
            .extend(input.contact_interfaces.iter().cloned().map(|mut contact| {
                contact.id = format!("branch-{branch}-{}", contact.id);
                contact.side_a = contact.side_a.map(|index| index + offset);
                contact.side_b = contact.side_b.map(|index| index + offset);
                contact
            }));
    }
    result
}

#[test]
fn unrelated_high_power_branch_cannot_hide_an_unresolved_contact() {
    let mut strong = model();
    for element in &mut strong.elements {
        element.conductivity *= 1e12;
    }
    strong.contact_interfaces[0].thermal_resistance_m2_k_w /= 1e12;
    let mut unresolved = model();
    unresolved.contact_interfaces[0].thermal_resistance_m2_k_w = 1e-12;
    for triangle in [false, true] {
        let error = solve(
            &joined_branches(&[strong.clone(), unresolved.clone()]),
            triangle,
        )
        .unwrap_err();
        assert!(error.contains("thermal contact heat balance"), "{error}");
        let recovered = solve(&joined_branches(&[strong.clone(), model()]), triangle).unwrap();
        relative_close(
            recovered.contacts[0].heat_flow_a_to_b_w / 1e12,
            20.0 / 3.5,
            1e-10,
        );
        relative_close(recovered.contacts[1].heat_flow_a_to_b_w, 20.0 / 3.5, 1e-10);
    }
}

fn refined_model(divisions: usize) -> SolveHeatPlaneQuad2dRequest {
    let mut input = model();
    input.nodes.clear();
    input.elements.clear();
    input.contact_interfaces.clear();
    let width = divisions + 1;
    let body_nodes = width * width;
    for body in 0..2 {
        for row in 0..=divisions {
            for column in 0..=divisions {
                let x = body as f64 + column as f64 / divisions as f64;
                input.nodes.push(HeatPlaneNodeInput {
                    id: format!("{body}-{row}-{column}"),
                    x,
                    y: row as f64 / divisions as f64,
                    fix_temperature: x == 0.0 || x == 2.0,
                    temperature: if x == 0.0 { 20.0 } else { 0.0 },
                    heat_load: 0.0,
                });
            }
        }
        for row in 0..divisions {
            for column in 0..divisions {
                let base = body * body_nodes + row * width + column;
                input.elements.push(HeatPlaneQuadElementInput {
                    id: format!("bulk-{body}-{row}-{column}"),
                    node_i: base,
                    node_j: base + 1,
                    node_k: base + width + 1,
                    node_l: base + width,
                    thickness: 0.5,
                    conductivity: if body == 0 { 2.0 } else { 4.0 },
                });
            }
        }
    }
    for row in 0..divisions {
        input.contact_interfaces.push(HeatPlaneContactInput {
            id: format!("contact-{row}"),
            side_a: [row * width + divisions, (row + 1) * width + divisions],
            side_b: [body_nodes + row * width, body_nodes + (row + 1) * width],
            thermal_resistance_m2_k_w: 1.0,
        });
    }
    input
}

#[test]
fn sparse_contact_patch_satisfies_balance_with_all_preconditioners() {
    let input = refined_model(32);
    let mut refined_cases = 0;
    assert!(
        input
            .nodes
            .iter()
            .filter(|node| !node.fix_temperature)
            .count()
            > 1024
    );
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        let profile = profile_heat_plane_quad_2d_with_options(
            &input,
            SpdSolveOptions {
                preconditioner,
                ..SpdSolveOptions::default()
            },
        )
        .unwrap();
        assert!(profile.solver_iterations > 0);
        refined_cases += usize::from(
            profile
                .memory_stages
                .iter()
                .any(|stage| stage.label == "contact_balance_refinement"),
        );
        let power = profile
            .result
            .contact_interfaces
            .iter()
            .map(|contact| contact.heat_flow_a_to_b_w)
            .sum::<f64>();
        relative_close(power, 20.0 / 3.5, 1e-7);
        for element in &profile.result.elements {
            relative_close(element.heat_flux_x * 0.5, 20.0 / 3.5, 1e-6);
        }
    }
    assert!(
        refined_cases > 0,
        "exercise conditional refinement, not only the initial solve"
    );
}
