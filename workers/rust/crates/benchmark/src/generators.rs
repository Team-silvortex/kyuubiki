use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, PlaneNodeInput, PlaneQuadElementInput,
    PlaneTriangleElementInput, SolveBarRequest, SolveHeatPlaneQuad2dRequest,
    SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
    Truss3dElementInput, Truss3dNodeInput, TrussElementInput, TrussNodeInput,
};

pub(crate) fn generate_bar_case(elements: usize) -> SolveBarRequest {
    SolveBarRequest {
        length: 20.0,
        area: 0.01,
        youngs_modulus: 70.0e9,
        elements,
        tip_force: 1800.0,
    }
}

pub(crate) fn generate_pratt_truss(bays: usize, span: f64, height: f64) -> SolveTruss2dRequest {
    let bay_width = span / bays as f64;
    let mut nodes = Vec::new();
    let mut elements = Vec::new();

    for index in 0..=bays {
        nodes.push(TrussNodeInput {
            id: format!("b{index}"),
            x: index as f64 * bay_width,
            y: 0.0,
            fix_x: index == 0,
            fix_y: index == 0 || index == bays,
            load_x: 0.0,
            load_y: 0.0,
        });
    }

    for index in 0..bays {
        nodes.push(TrussNodeInput {
            id: format!("t{index}"),
            x: index as f64 * bay_width + bay_width * 0.5,
            y: height,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: if index == bays / 2 { -40.0 } else { 0.0 },
        });
    }

    let top_offset = bays + 1;
    for index in 0..bays {
        elements.push(TrussElementInput {
            id: format!("bb{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.03,
            youngs_modulus: 210.0e9,
        });

        elements.push(TrussElementInput {
            id: format!("s{index}_l"),
            node_i: index,
            node_j: top_offset + index,
            area: 0.03,
            youngs_modulus: 210.0e9,
        });

        elements.push(TrussElementInput {
            id: format!("s{index}_r"),
            node_i: top_offset + index,
            node_j: index + 1,
            area: 0.03,
            youngs_modulus: 210.0e9,
        });

        if index + 1 < bays {
            elements.push(TrussElementInput {
                id: format!("tt{index}"),
                node_i: top_offset + index,
                node_j: top_offset + index + 1,
                area: 0.03,
                youngs_modulus: 210.0e9,
            });
        }
    }

    SolveTruss2dRequest { nodes, elements }
}

pub(crate) fn generate_panel_mesh(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolvePlaneTriangle2dRequest {
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let mut nodes = Vec::new();
    let mut elements = Vec::new();

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            nodes.push(PlaneNodeInput {
                id: format!("n{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                fix_x: i == 0,
                fix_y: i == 0 || (j == 0 && i == 0),
                load_x: if i == nx { 15.0 } else { 0.0 },
                load_y: if i == nx { -40.0 } else { 0.0 },
            });
        }
    }

    for j in 0..ny {
        for i in 0..nx {
            let n0 = j * (nx + 1) + i;
            let n1 = n0 + 1;
            let n2 = n0 + (nx + 1);
            let n3 = n2 + 1;

            elements.push(PlaneTriangleElementInput {
                id: format!("p{}_a", j * nx + i),
                node_i: n0,
                node_j: n1,
                node_k: n3,
                thickness: 0.015,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            });
            elements.push(PlaneTriangleElementInput {
                id: format!("p{}_b", j * nx + i),
                node_i: n0,
                node_j: n3,
                node_k: n2,
                thickness: 0.015,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            });
        }
    }

    SolvePlaneTriangle2dRequest { nodes, elements }
}

pub(crate) fn generate_quad_panel_mesh(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolvePlaneQuad2dRequest {
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let mut nodes = Vec::new();
    let mut elements = Vec::new();

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            nodes.push(PlaneNodeInput {
                id: format!("n{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                fix_x: i == 0,
                fix_y: i == 0 || (j == 0 && i == 0),
                load_x: if i == nx { 15.0 } else { 0.0 },
                load_y: if i == nx { -40.0 } else { 0.0 },
            });
        }
    }

    for j in 0..ny {
        for i in 0..nx {
            let n0 = j * (nx + 1) + i;
            let n1 = n0 + 1;
            let n2 = n0 + (nx + 1);
            let n3 = n2 + 1;

            elements.push(PlaneQuadElementInput {
                id: format!("q{}", j * nx + i),
                node_i: n0,
                node_j: n1,
                node_k: n3,
                node_l: n2,
                thickness: 0.015,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            });
        }
    }

    SolvePlaneQuad2dRequest { nodes, elements }
}

pub(crate) fn generate_heat_quad_panel_mesh(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveHeatPlaneQuad2dRequest {
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let mut nodes = Vec::new();
    let mut elements = Vec::new();

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            let left_edge = i == 0;
            let right_edge = i == nx;
            nodes.push(HeatPlaneNodeInput {
                id: format!("h{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                fix_temperature: left_edge || right_edge,
                temperature: if left_edge {
                    120.0
                } else if right_edge {
                    20.0
                } else {
                    0.0
                },
                heat_load: if !left_edge && !right_edge && j == ny / 2 {
                    25.0
                } else {
                    0.0
                },
            });
        }
    }

    for j in 0..ny {
        for i in 0..nx {
            let n0 = j * (nx + 1) + i;
            let n1 = n0 + 1;
            let n2 = n0 + (nx + 1);
            let n3 = n2 + 1;

            elements.push(HeatPlaneQuadElementInput {
                id: format!("hq{}", j * nx + i),
                node_i: n0,
                node_j: n1,
                node_k: n3,
                node_l: n2,
                thickness: 0.02,
                conductivity: 45.0,
            });
        }
    }

    SolveHeatPlaneQuad2dRequest { nodes, elements }
}

pub(crate) fn generate_lattice_truss_10k(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveTruss2dRequest {
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let mut nodes = Vec::with_capacity((nx + 1) * (ny + 1));
    let mut elements = Vec::new();
    let center_i = nx / 2;
    let load_band_start = center_i.saturating_sub(1);
    let load_band_end = center_i + 1;

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            let load_band = i >= load_band_start && i <= load_band_end && j == ny;

            nodes.push(TrussNodeInput {
                id: format!("n{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                fix_x: j == 0 && i == 0,
                fix_y: j == 0 && (i == 0 || i == nx),
                load_x: 0.0,
                load_y: if load_band { -600.0 } else { 0.0 },
            });
        }
    }

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;

            if i < nx {
                elements.push(TrussElementInput {
                    id: format!("hx{index}"),
                    node_i: index,
                    node_j: index + 1,
                    area: 0.014,
                    youngs_modulus: 210.0e9,
                });
            }

            if j < ny {
                elements.push(TrussElementInput {
                    id: format!("vy{index}"),
                    node_i: index,
                    node_j: index + (nx + 1),
                    area: 0.014,
                    youngs_modulus: 210.0e9,
                });
            }

            if i < nx && j < ny {
                elements.push(TrussElementInput {
                    id: format!("d1{index}"),
                    node_i: index,
                    node_j: index + (nx + 2),
                    area: 0.012,
                    youngs_modulus: 210.0e9,
                });
                elements.push(TrussElementInput {
                    id: format!("d2{index}"),
                    node_i: index + 1,
                    node_j: index + (nx + 1),
                    area: 0.012,
                    youngs_modulus: 210.0e9,
                });
            }
        }
    }

    SolveTruss2dRequest { nodes, elements }
}

pub(crate) fn generate_space_frame_grid(
    nx: usize,
    ny: usize,
    width: f64,
    depth: f64,
    height: f64,
) -> SolveTruss3dRequest {
    let dx = width / nx as f64;
    let dy = depth / ny as f64;
    let layer_size = (nx + 1) * (ny + 1);
    let mut nodes = Vec::with_capacity(layer_size * 2);
    let mut elements = Vec::new();

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            nodes.push(Truss3dNodeInput {
                id: format!("b{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
            });
        }
    }

    let center_i = nx / 2;
    let center_j = ny / 2;

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            let radial_distance = ((i as isize - center_i as isize).abs()
                + (j as isize - center_j as isize).abs()) as f64;
            let load = if radial_distance <= 2.0 {
                -4_000.0
            } else {
                0.0
            };

            nodes.push(Truss3dNodeInput {
                id: format!("t{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                z: height,
                fix_x: false,
                fix_y: false,
                fix_z: false,
                load_x: 0.0,
                load_y: 0.0,
                load_z: load,
            });
        }
    }

    for j in 0..=ny {
        for i in 0..=nx {
            let base = j * (nx + 1) + i;
            let top = layer_size + base;

            elements.push(Truss3dElementInput {
                id: format!("v{base}"),
                node_i: base,
                node_j: top,
                area: 0.02,
                youngs_modulus: 210.0e9,
            });

            if i < nx {
                elements.push(Truss3dElementInput {
                    id: format!("bx{base}"),
                    node_i: base,
                    node_j: base + 1,
                    area: 0.018,
                    youngs_modulus: 210.0e9,
                });
                elements.push(Truss3dElementInput {
                    id: format!("tx{base}"),
                    node_i: top,
                    node_j: top + 1,
                    area: 0.018,
                    youngs_modulus: 210.0e9,
                });
                elements.push(Truss3dElementInput {
                    id: format!("dx{base}"),
                    node_i: base,
                    node_j: top + 1,
                    area: 0.016,
                    youngs_modulus: 210.0e9,
                });
                elements.push(Truss3dElementInput {
                    id: format!("ux{base}"),
                    node_i: top,
                    node_j: base + 1,
                    area: 0.016,
                    youngs_modulus: 210.0e9,
                });
            }

            if j < ny {
                elements.push(Truss3dElementInput {
                    id: format!("by{base}"),
                    node_i: base,
                    node_j: base + (nx + 1),
                    area: 0.018,
                    youngs_modulus: 210.0e9,
                });
                elements.push(Truss3dElementInput {
                    id: format!("ty{base}"),
                    node_i: top,
                    node_j: top + (nx + 1),
                    area: 0.018,
                    youngs_modulus: 210.0e9,
                });
                elements.push(Truss3dElementInput {
                    id: format!("dy{base}"),
                    node_i: base,
                    node_j: top + (nx + 1),
                    area: 0.016,
                    youngs_modulus: 210.0e9,
                });
                elements.push(Truss3dElementInput {
                    id: format!("uy{base}"),
                    node_i: top,
                    node_j: base + (nx + 1),
                    area: 0.016,
                    youngs_modulus: 210.0e9,
                });
            }
        }
    }

    SolveTruss3dRequest { nodes, elements }
}
