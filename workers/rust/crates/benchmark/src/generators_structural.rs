use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, BucklingBeam1dElementInput, BucklingBeam1dNodeInput,
    ContactGap1dContactInput, Frame2dElementInput, Frame2dNodeInput, Frame3dNodeInput,
    ModalFrame2dElementInput, ModalFrame3dElementInput, NonlinearSpring1dElementInput,
    NonlinearSpring1dNodeInput, SolidTetra3dElementInput, SolidTetra3dNodeInput,
    SolveBeam1dRequest, SolveBucklingBeam1dRequest, SolveBucklingFrame2dRequest,
    SolveContactGap1dRequest, SolveFrame2dRequest, SolveModalFrame2dRequest,
    SolveModalFrame3dRequest, SolveNonlinearSpring1dRequest, SolveSolidTetra3dRequest,
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBeam1dRequest,
    Spring1dElementInput, Spring1dNodeInput, Spring2dElementInput, Spring2dNodeInput,
    Spring3dElementInput, Spring3dNodeInput, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
};

pub(crate) fn generate_spring_1d_case(elements: usize) -> SolveSpring1dRequest {
    let nodes = (0..=elements)
        .map(|index| Spring1dNodeInput {
            id: format!("s{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: if index == elements { 1200.0 } else { 0.0 },
        })
        .collect();
    let elements = (0..elements)
        .map(|index| Spring1dElementInput {
            id: format!("k{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: 30_000.0,
        })
        .collect();
    SolveSpring1dRequest { nodes, elements }
}

pub(crate) fn generate_nonlinear_spring_1d_case(elements: usize) -> SolveNonlinearSpring1dRequest {
    let nodes = (0..=elements)
        .map(|index| NonlinearSpring1dNodeInput {
            id: format!("ns{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: if index == elements { 350.0 } else { 0.0 },
        })
        .collect();
    let elements = (0..elements)
        .map(|index| NonlinearSpring1dElementInput {
            id: format!("nk{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: 25_000.0,
            cubic_stiffness: 1_200.0,
        })
        .collect();
    SolveNonlinearSpring1dRequest {
        nodes,
        elements,
        load_steps: Some(4),
        max_iterations: Some(24),
        tolerance: Some(1.0e-8),
    }
}

pub(crate) fn generate_contact_gap_1d_case(elements: usize) -> SolveContactGap1dRequest {
    let base = generate_nonlinear_spring_1d_case(elements);
    SolveContactGap1dRequest {
        contacts: vec![ContactGap1dContactInput {
            id: "stop".to_string(),
            node: base.nodes.len().saturating_sub(1),
            gap: 0.01,
            normal_stiffness: 150_000.0,
        }],
        nodes: base.nodes,
        elements: base.elements,
        load_steps: Some(4),
        max_iterations: Some(24),
        tolerance: Some(1.0e-8),
    }
}

pub(crate) fn generate_beam_1d_case(elements: usize) -> SolveBeam1dRequest {
    let nodes = beam_nodes(elements, |index, x, end| Beam1dNodeInput {
        id: format!("b{index}"),
        x,
        fix_y: index == 0,
        fix_rz: index == 0,
        load_y: if index == end { -1000.0 } else { 0.0 },
        moment_z: 0.0,
    });
    let elements = (0..elements)
        .map(|index| Beam1dElementInput {
            id: format!("be{index}"),
            node_i: index,
            node_j: index + 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            distributed_load_y: -80.0,
        })
        .collect();
    SolveBeam1dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_beam_1d_case(elements: usize) -> SolveThermalBeam1dRequest {
    let nodes = beam_nodes(elements, |index, x, _| ThermalBeam1dNodeInput {
        id: format!("tb{index}"),
        x,
        fix_y: index == 0,
        fix_rz: index == 0,
        load_y: 0.0,
        moment_z: 0.0,
    });
    let elements = (0..elements)
        .map(|index| ThermalBeam1dElementInput {
            id: format!("tbe{index}"),
            node_i: index,
            node_j: index + 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 1.2e-4,
            section_modulus: 1.1e-3,
            thermal_expansion: 12.0e-6,
            section_depth: 0.3,
            distributed_load_y: 0.0,
            temperature_gradient_y: 35.0,
        })
        .collect();
    SolveThermalBeam1dRequest { nodes, elements }
}

/// Generates a supported two-row spring lattice with diagonal bracing.
/// Periodic supports bound each span's condition number while retaining a real 2D topology.
pub(crate) fn generate_spring_2d_ladder_case(target_nodes: usize) -> SolveSpring2dRequest {
    const SUPPORT_INTERVAL: usize = 64;
    let columns = (target_nodes.max(4) / 2).max(2);
    let nodes = (0..columns)
        .flat_map(|column| {
            [0, 1].into_iter().map(move |row| Spring2dNodeInput {
                id: format!("sg-{column}-{row}"),
                x: column as f64,
                y: row as f64,
                fix_x: column % SUPPORT_INTERVAL == 0,
                fix_y: true,
                load_x: if column % SUPPORT_INTERVAL == SUPPORT_INTERVAL - 1 {
                    20.0
                } else {
                    0.0
                },
                load_y: 0.0,
            })
        })
        .collect::<Vec<_>>();
    let mut elements = Vec::with_capacity((columns - 1) * 4);
    for column in 0..columns - 1 {
        let left_top = column * 2;
        let left_bottom = left_top + 1;
        let right_top = left_top + 2;
        let right_bottom = left_top + 3;
        for (suffix, node_i, node_j) in [
            ("top", left_top, right_top),
            ("bottom", left_bottom, right_bottom),
            ("diag-a", left_top, right_bottom),
            ("diag-b", left_bottom, right_top),
        ] {
            elements.push(Spring2dElementInput {
                id: format!("sg-{column}-{suffix}"),
                node_i,
                node_j,
                stiffness: 48_000.0,
            });
        }
    }
    SolveSpring2dRequest { nodes, elements }
}

/// Generates a supported four-corner space-spring cage with cross-bracing.
pub(crate) fn generate_spring_3d_cage_case(target_nodes: usize) -> SolveSpring3dRequest {
    const SUPPORT_INTERVAL: usize = 64;
    let columns = (target_nodes.max(8) / 4).max(2);
    let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    let nodes = (0..columns)
        .flat_map(|column| {
            corners
                .into_iter()
                .enumerate()
                .map(move |(corner, (y, z))| Spring3dNodeInput {
                    id: format!("sc-{column}-{corner}"),
                    x: column as f64,
                    y,
                    z,
                    fix_x: column % SUPPORT_INTERVAL == 0,
                    fix_y: true,
                    fix_z: true,
                    load_x: if column % SUPPORT_INTERVAL == SUPPORT_INTERVAL - 1 {
                        16.0
                    } else {
                        0.0
                    },
                    load_y: 0.0,
                    load_z: 0.0,
                })
        })
        .collect::<Vec<_>>();
    let mut elements = Vec::with_capacity((columns - 1) * 8);
    for column in 0..columns - 1 {
        for corner in 0..4 {
            elements.push(Spring3dElementInput {
                id: format!("sc-{column}-rail-{corner}"),
                node_i: column * 4 + corner,
                node_j: (column + 1) * 4 + corner,
                stiffness: 52_000.0,
            });
            elements.push(Spring3dElementInput {
                id: format!("sc-{column}-brace-{corner}"),
                node_i: column * 4 + corner,
                node_j: (column + 1) * 4 + (corner + 1) % 4,
                stiffness: 42_000.0,
            });
        }
    }
    SolveSpring3dRequest { nodes, elements }
}

/// Generates independent, fully constrained tetrahedral material specimens for batch scaling.
/// Each cell has three fixed base nodes and a loaded free apex, exercising full 3D elasticity.
pub(crate) fn generate_solid_tetra_3d_specimen_batch(
    target_nodes: usize,
) -> SolveSolidTetra3dRequest {
    let specimen_count = (target_nodes.max(4) / 4).max(1);
    let mut nodes = Vec::with_capacity(specimen_count * 4);
    let mut elements = Vec::with_capacity(specimen_count);
    for specimen in 0..specimen_count {
        let offset = specimen as f64 * 2.0;
        let base = specimen * 4;
        for (suffix, x, y, z, fix, load_z) in [
            ("a", offset, 0.0, 0.0, true, 0.0),
            ("b", offset + 1.0, 0.0, 0.0, true, 0.0),
            ("c", offset, 1.0, 0.0, true, 0.0),
            ("tip", offset, 0.0, 1.0, false, -250.0),
        ] {
            nodes.push(SolidTetra3dNodeInput {
                id: format!("st-{specimen}-{suffix}"),
                x,
                y,
                z,
                fix_x: fix,
                fix_y: fix,
                fix_z: fix,
                load_x: 0.0,
                load_y: 0.0,
                load_z,
            });
        }
        elements.push(SolidTetra3dElementInput {
            id: format!("st-{specimen}"),
            node_a: base,
            node_b: base + 1,
            node_c: base + 2,
            node_d: base + 3,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        });
    }
    SolveSolidTetra3dRequest { nodes, elements }
}

pub(crate) fn generate_modal_frame_2d_chain_case(
    segments: usize,
    length: f64,
) -> SolveModalFrame2dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| Frame2dNodeInput {
            id: format!("mf2n{index}"),
            x: index as f64 * dx,
            y: 0.0,
            fix_x: index == 0,
            // This benchmark isolates the axial modal branch of a frame chain.
            fix_y: true,
            fix_rz: true,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        })
        .collect();
    let elements = (0..segments)
        .map(|index| ModalFrame2dElementInput {
            id: format!("mf2e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.333e-6,
            section_modulus: 1.667e-4,
            density: 7850.0,
        })
        .collect();
    SolveModalFrame2dRequest {
        nodes,
        elements,
        mode_count: Some(1),
    }
}

pub(crate) fn generate_buckling_beam_1d_case(
    segments: usize,
    length: f64,
) -> SolveBucklingBeam1dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    SolveBucklingBeam1dRequest {
        nodes: (0..=segments)
            .map(|index| BucklingBeam1dNodeInput {
                id: format!("bbn{index}"),
                x: index as f64 * dx,
                fix_y: index == 0 || index == segments,
                fix_rz: false,
            })
            .collect(),
        elements: (0..segments)
            .map(|index| BucklingBeam1dElementInput {
                id: format!("bbe{index}"),
                node_i: index,
                node_j: index + 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.333e-6,
                reference_compressive_force: 100_000.0,
            })
            .collect(),
        mode_count: Some(1),
    }
}

pub(crate) fn generate_buckling_frame_2d_case(
    segments: usize,
    length: f64,
) -> SolveBucklingFrame2dRequest {
    let segments = segments.max(1);
    let dy = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| Frame2dNodeInput {
            id: format!("bfn{index}"),
            x: 0.0,
            y: index as f64 * dy,
            fix_x: index == 0 || index == segments,
            fix_y: index == 0,
            fix_rz: false,
            load_x: 0.0,
            load_y: if index == segments { -100_000.0 } else { 0.0 },
            moment_z: 0.0,
        })
        .collect();
    let elements = (0..segments)
        .map(|index| Frame2dElementInput {
            id: format!("bfe{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.333e-6,
            section_modulus: 1.667e-4,
        })
        .collect();
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest { nodes, elements },
        mode_count: Some(1),
    }
}

pub(crate) fn generate_modal_frame_3d_chain_case(
    segments: usize,
    length: f64,
) -> SolveModalFrame3dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| Frame3dNodeInput {
            id: format!("mf3n{index}"),
            x: index as f64 * dx,
            y: 0.0,
            z: 0.0,
            fix_x: index == 0,
            // This benchmark isolates the axial modal branch of a space-frame chain.
            fix_y: true,
            fix_z: true,
            fix_rx: true,
            fix_ry: true,
            fix_rz: true,
            load_x: 0.0,
            load_y: 0.0,
            load_z: 0.0,
            moment_x: 0.0,
            moment_y: 0.0,
            moment_z: 0.0,
        })
        .collect();
    let elements = (0..segments)
        .map(|index| ModalFrame3dElementInput {
            id: format!("mf3e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 1.0e-5,
            moment_of_inertia_y: 8.333e-6,
            moment_of_inertia_z: 8.333e-6,
            density: 7850.0,
        })
        .collect();
    SolveModalFrame3dRequest {
        nodes,
        elements,
        mode_count: Some(1),
    }
}

fn beam_nodes<T>(elements: usize, mut build: impl FnMut(usize, f64, usize) -> T) -> Vec<T> {
    let length = 4.0;
    (0..=elements)
        .map(|index| {
            build(
                index,
                index as f64 * length / elements.max(1) as f64,
                elements,
            )
        })
        .collect()
}
