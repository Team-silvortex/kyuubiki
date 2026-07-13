#[path = "generators_thermal_structural_frames.rs"]
mod frames;

pub(crate) use frames::{
    generate_frame_2d_case, generate_frame_3d_case, generate_thermal_frame_2d_case,
    generate_thermal_frame_3d_case,
};

use kyuubiki_protocol::{
    SolveThermalBar1dRequest, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, ThermalBar1dElementInput,
    ThermalBar1dNodeInput, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
    ThermalPlaneTriangleElementInput, ThermalTruss2dElementInput, ThermalTruss2dNodeInput,
    ThermalTruss3dElementInput, ThermalTruss3dNodeInput,
};

pub(crate) fn generate_thermal_bar_case(elements: usize) -> SolveThermalBar1dRequest {
    let nodes = (0..=elements)
        .map(|index| ThermalBar1dNodeInput {
            id: format!("tb{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: if index == elements { 1500.0 } else { 0.0 },
            temperature_delta: index as f64 * 20.0 / elements.max(1) as f64,
        })
        .collect();
    let elements = (0..elements)
        .map(|index| ThermalBar1dElementInput {
            id: format!("tbe{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.015,
            youngs_modulus: 210.0e9,
            thermal_expansion: 12.0e-6,
        })
        .collect();
    SolveThermalBar1dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_lattice_truss_2d_case(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveThermalTruss2dRequest {
    let dx = width / nx.max(1) as f64;
    let dy = height / ny.max(1) as f64;
    let mut nodes = Vec::with_capacity((nx + 1) * (ny + 1));
    let mut elements = Vec::new();
    let load_i = nx / 2;

    for row in 0..=ny {
        for col in 0..=nx {
            let index = grid_index(row, col, nx);
            nodes.push(thermal_truss_2d_node(
                &format!("tt2n{index}"),
                col as f64 * dx,
                row as f64 * dy,
                row == 0 && col == 0,
                row == 0 && (col == 0 || col == nx),
                0.0,
                if row == ny && col.abs_diff(load_i) <= 1 {
                    -600.0
                } else {
                    0.0
                },
                20.0 + 35.0 * row as f64 / ny.max(1) as f64,
            ));
        }
    }

    for row in 0..=ny {
        for col in 0..=nx {
            let index = grid_index(row, col, nx);
            if col < nx {
                elements.push(thermal_truss_2d_element(
                    &format!("tt2hx{index}"),
                    index,
                    index + 1,
                ));
            }
            if row < ny {
                elements.push(thermal_truss_2d_element(
                    &format!("tt2vy{index}"),
                    index,
                    index + nx + 1,
                ));
            }
            if col < nx && row < ny {
                elements.push(thermal_truss_2d_element(
                    &format!("tt2d1{index}"),
                    index,
                    index + nx + 2,
                ));
                elements.push(thermal_truss_2d_element(
                    &format!("tt2d2{index}"),
                    index + 1,
                    index + nx + 1,
                ));
            }
        }
    }

    SolveThermalTruss2dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_space_truss_3d_case(
    nx: usize,
    ny: usize,
    width: f64,
    depth: f64,
    height: f64,
) -> SolveThermalTruss3dRequest {
    let dx = width / nx.max(1) as f64;
    let dy = depth / ny.max(1) as f64;
    let layer_size = (nx + 1) * (ny + 1);
    let mut nodes = Vec::with_capacity(layer_size * 2);
    let mut elements = Vec::new();

    for row in 0..=ny {
        for col in 0..=nx {
            let index = grid_index(row, col, nx);
            nodes.push(thermal_truss_3d_node(
                &format!("tt3b{index}"),
                col as f64 * dx,
                row as f64 * dy,
                0.0,
                true,
                true,
                true,
                0.0,
                0.0,
                0.0,
                18.0,
            ));
        }
    }

    let center_i = nx / 2;
    let center_j = ny / 2;
    for row in 0..=ny {
        for col in 0..=nx {
            let index = grid_index(row, col, nx);
            let near_center = col.abs_diff(center_i) + row.abs_diff(center_j) <= 2;
            nodes.push(thermal_truss_3d_node(
                &format!("tt3t{index}"),
                col as f64 * dx,
                row as f64 * dy,
                height,
                false,
                false,
                false,
                0.0,
                0.0,
                if near_center { -4_000.0 } else { 0.0 },
                18.0 + 35.0 * row as f64 / ny.max(1) as f64,
            ));
        }
    }

    for row in 0..=ny {
        for col in 0..=nx {
            let base = grid_index(row, col, nx);
            let top = layer_size + base;
            elements.push(thermal_truss_3d_element(&format!("tt3v{base}"), base, top));
            if col < nx {
                elements.push(thermal_truss_3d_element(
                    &format!("tt3bx{base}"),
                    base,
                    base + 1,
                ));
                elements.push(thermal_truss_3d_element(
                    &format!("tt3tx{base}"),
                    top,
                    top + 1,
                ));
                elements.push(thermal_truss_3d_element(
                    &format!("tt3dx{base}"),
                    base,
                    top + 1,
                ));
            }
            if row < ny {
                elements.push(thermal_truss_3d_element(
                    &format!("tt3by{base}"),
                    base,
                    base + nx + 1,
                ));
                elements.push(thermal_truss_3d_element(
                    &format!("tt3ty{base}"),
                    top,
                    top + nx + 1,
                ));
                elements.push(thermal_truss_3d_element(
                    &format!("tt3dy{base}"),
                    base,
                    top + nx + 1,
                ));
            }
        }
    }

    SolveThermalTruss3dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_triangle_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveThermalPlaneTriangle2dRequest {
    let nodes = thermal_plane_nodes(nx, ny, width, height);
    let mut elements = Vec::with_capacity(nx * ny * 2);
    for row in 0..ny {
        for col in 0..nx {
            let n0 = grid_index(row, col, nx);
            let n1 = grid_index(row, col + 1, nx);
            let n2 = grid_index(row + 1, col, nx);
            let n3 = grid_index(row + 1, col + 1, nx);
            elements.push(thermal_triangle_element(elements.len(), n0, n1, n3));
            elements.push(thermal_triangle_element(elements.len(), n0, n3, n2));
        }
    }
    SolveThermalPlaneTriangle2dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_quad_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveThermalPlaneQuad2dRequest {
    let nodes = thermal_plane_nodes(nx, ny, width, height);
    let mut elements = Vec::with_capacity(nx * ny);
    for row in 0..ny {
        for col in 0..nx {
            elements.push(ThermalPlaneQuadElementInput {
                id: format!("tq{row}-{col}"),
                node_i: grid_index(row, col, nx),
                node_j: grid_index(row, col + 1, nx),
                node_k: grid_index(row + 1, col + 1, nx),
                node_l: grid_index(row + 1, col, nx),
                thickness: 0.05,
                youngs_modulus: 210.0e9,
                poisson_ratio: 0.29,
                thermal_expansion: 12.0e-6,
            });
        }
    }
    SolveThermalPlaneQuad2dRequest { nodes, elements }
}

fn thermal_plane_nodes(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> Vec<ThermalPlaneNodeInput> {
    let mut nodes = Vec::with_capacity((nx + 1) * (ny + 1));
    for row in 0..=ny {
        for col in 0..=nx {
            let y = row as f64 * height / ny.max(1) as f64;
            nodes.push(ThermalPlaneNodeInput {
                id: format!("tp{row}-{col}"),
                x: col as f64 * width / nx.max(1) as f64,
                y,
                fix_x: col == 0,
                fix_y: col == 0,
                load_x: if col == nx { 150.0 } else { 0.0 },
                load_y: if row == ny { -80.0 } else { 0.0 },
                temperature_delta: 20.0 + 40.0 * y / height.max(1.0e-9),
            });
        }
    }
    nodes
}

fn thermal_triangle_element(
    index: usize,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> ThermalPlaneTriangleElementInput {
    ThermalPlaneTriangleElementInput {
        id: format!("tt{index}"),
        node_i,
        node_j,
        node_k,
        thickness: 0.05,
        youngs_modulus: 210.0e9,
        poisson_ratio: 0.29,
        thermal_expansion: 12.0e-6,
    }
}

fn grid_index(row: usize, col: usize, nx: usize) -> usize {
    row * (nx + 1) + col
}
fn thermal_truss_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
    temperature_delta: f64,
) -> ThermalTruss2dNodeInput {
    ThermalTruss2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
        temperature_delta,
    }
}
fn thermal_truss_2d_element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss2dElementInput {
    ThermalTruss2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 210.0e9,
        thermal_expansion: 12.0e-6,
    }
}
#[allow(clippy::too_many_arguments)]
fn thermal_truss_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
    temperature_delta: f64,
) -> ThermalTruss3dNodeInput {
    ThermalTruss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        load_x,
        load_y,
        load_z,
        temperature_delta,
    }
}
fn thermal_truss_3d_element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss3dElementInput {
    ThermalTruss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.012,
        youngs_modulus: 210.0e9,
        thermal_expansion: 12.0e-6,
    }
}
