use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, Frame3dElementInput, Frame3dNodeInput,
    SolveFrame2dRequest, SolveFrame3dRequest, SolveThermalBar1dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, ThermalBar1dElementInput,
    ThermalBar1dNodeInput, ThermalFrame2dElementInput, ThermalFrame2dNodeInput,
    ThermalFrame3dElementInput, ThermalFrame3dNodeInput, ThermalPlaneNodeInput,
    ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput, ThermalTruss2dElementInput,
    ThermalTruss2dNodeInput, ThermalTruss3dElementInput, ThermalTruss3dNodeInput,
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

pub(crate) fn generate_thermal_truss_2d_case() -> SolveThermalTruss2dRequest {
    SolveThermalTruss2dRequest {
        nodes: vec![
            thermal_truss_2d_node("t0", 0.0, 0.0, true, true, 0.0, 0.0, 20.0),
            thermal_truss_2d_node("t1", 2.0, 0.0, true, true, 0.0, 0.0, 20.0),
            thermal_truss_2d_node("top", 1.0, 1.4, false, false, 400.0, -900.0, 45.0),
        ],
        elements: vec![
            thermal_truss_2d_element("tt0", 0, 2),
            thermal_truss_2d_element("tt1", 1, 2),
            thermal_truss_2d_element("tt2", 0, 1),
        ],
    }
}

pub(crate) fn generate_thermal_truss_3d_case() -> SolveThermalTruss3dRequest {
    SolveThermalTruss3dRequest {
        nodes: vec![
            thermal_truss_3d_node("a", 0.0, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0, 15.0),
            thermal_truss_3d_node("b", 1.6, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0, 15.0),
            thermal_truss_3d_node("c", 0.0, 1.4, 0.0, true, true, true, 0.0, 0.0, 0.0, 15.0),
            thermal_truss_3d_node(
                "tip", 0.7, 0.5, 1.4, false, false, false, 300.0, -200.0, -1100.0, 42.0,
            ),
        ],
        elements: vec![
            thermal_truss_3d_element("tt3-0", 0, 3),
            thermal_truss_3d_element("tt3-1", 1, 3),
            thermal_truss_3d_element("tt3-2", 2, 3),
            thermal_truss_3d_element("tt3-3", 0, 1),
            thermal_truss_3d_element("tt3-4", 1, 2),
            thermal_truss_3d_element("tt3-5", 2, 0),
        ],
    }
}

pub(crate) fn generate_frame_2d_case() -> SolveFrame2dRequest {
    SolveFrame2dRequest {
        nodes: vec![
            frame_2d_node("f0", 0.0, 0.0, true, 0.0, 0.0, 0.0),
            frame_2d_node("f1", 2.4, 0.0, false, 0.0, -1000.0, 0.0),
        ],
        elements: vec![Frame2dElementInput {
            id: "f2e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        }],
    }
}

pub(crate) fn generate_frame_3d_case() -> SolveFrame3dRequest {
    SolveFrame3dRequest {
        nodes: vec![
            frame_3d_node("f3-0", 0.0, true, 0.0, 0.0, 0.0),
            frame_3d_node("f3-1", 2.4, false, 0.0, -1000.0, 0.0),
        ],
        elements: vec![Frame3dElementInput {
            id: "f3e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 8.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.6e-4,
        }],
    }
}

pub(crate) fn generate_thermal_frame_2d_case() -> SolveThermalFrame2dRequest {
    SolveThermalFrame2dRequest {
        nodes: vec![
            thermal_frame_2d_node("tf0", 0.0, true, 35.0),
            thermal_frame_2d_node("tf1", 2.0, true, 35.0),
        ],
        elements: vec![ThermalFrame2dElementInput {
            id: "tf2e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion: 12.0e-6,
            section_depth: 0.2,
            temperature_gradient_y: 30.0,
        }],
    }
}

pub(crate) fn generate_thermal_frame_3d_case() -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            thermal_frame_3d_node("tf3-0", 0.0, true, 35.0),
            thermal_frame_3d_node("tf3-1", 2.0, true, 35.0),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "tf3e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 6.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.2e-4,
            thermal_expansion: 12.0e-6,
            section_depth_y: 0.2,
            section_depth_z: 0.15,
            temperature_gradient_y: 30.0,
            temperature_gradient_z: 20.0,
        }],
    }
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

fn frame_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fixed: bool,
    load_x: f64,
    load_y: f64,
    moment_z: f64,
) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x,
        load_y,
        moment_z,
    }
}

fn frame_3d_node(
    id: &str,
    x: f64,
    fixed: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x,
        load_y,
        load_z,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
    }
}

fn thermal_frame_2d_node(
    id: &str,
    x: f64,
    fixed: bool,
    temperature_delta: f64,
) -> ThermalFrame2dNodeInput {
    ThermalFrame2dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

fn thermal_frame_3d_node(
    id: &str,
    x: f64,
    fixed: bool,
    temperature_delta: f64,
) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}
