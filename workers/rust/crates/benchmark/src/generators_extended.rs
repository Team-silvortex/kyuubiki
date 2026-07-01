use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dNodeInput, ElectrostaticBar1dElementInput,
    ElectrostaticBar1dNodeInput, ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneTriangleElementInput, HeatBar1dElementInput, HeatBar1dNodeInput,
    HeatPlaneNodeInput, HeatPlaneTriangleElementInput, MagnetostaticBar1dElementInput,
    MagnetostaticBar1dNodeInput, MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneTriangleElementInput, SolveAcousticBar1dRequest,
    SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveHeatBar1dRequest,
    SolveHeatPlaneTriangle2dRequest, SolveMagnetostaticBar1dRequest,
    SolveMagnetostaticPlaneQuad2dRequest, SolveMagnetostaticPlaneTriangle2dRequest,
    SolveStokesFlowPlaneQuad2dRequest, SolveTorsion1dRequest, StokesFlowPlaneNodeInput,
    StokesFlowPlaneQuadElementInput, Torsion1dElementInput, Torsion1dNodeInput,
};

pub(crate) fn generate_heat_bar_case(elements: usize) -> SolveHeatBar1dRequest {
    let nodes = line_nodes(elements, |index, x, end| HeatBar1dNodeInput {
        id: format!("h{index}"),
        x,
        fix_temperature: index == 0 || index == end,
        temperature: if index == 0 { 120.0 } else { 20.0 },
        heat_load: if index == end / 2 { 25.0 } else { 0.0 },
    });
    let elements = line_elements(elements, |index| HeatBar1dElementInput {
        id: format!("he{index}"),
        node_i: index,
        node_j: index + 1,
        area: 0.02,
        conductivity: 45.0,
    });
    SolveHeatBar1dRequest { nodes, elements }
}

pub(crate) fn generate_electrostatic_bar_case(elements: usize) -> SolveElectrostaticBar1dRequest {
    let nodes = line_nodes(elements, |index, x, end| ElectrostaticBar1dNodeInput {
        id: format!("e{index}"),
        x,
        fix_potential: index == 0 || index == end,
        potential: if index == 0 { 12.0 } else { 0.0 },
        charge_density: if index == end / 2 { 1.5e-6 } else { 0.0 },
    });
    let elements = line_elements(elements, |index| ElectrostaticBar1dElementInput {
        id: format!("ee{index}"),
        node_i: index,
        node_j: index + 1,
        area: 0.01,
        permittivity: 3.0,
    });
    SolveElectrostaticBar1dRequest { nodes, elements }
}

pub(crate) fn generate_magnetostatic_bar_case(elements: usize) -> SolveMagnetostaticBar1dRequest {
    let nodes = line_nodes(elements, |index, x, end| MagnetostaticBar1dNodeInput {
        id: format!("m{index}"),
        x,
        fix_magnetic_potential: index == 0 || index == end,
        magnetic_potential: if index == 0 { 0.2 } else { 0.0 },
        magnetomotive_source: if index == end / 2 { 20.0 } else { 0.0 },
    });
    let elements = line_elements(elements, |index| MagnetostaticBar1dElementInput {
        id: format!("me{index}"),
        node_i: index,
        node_j: index + 1,
        area: 0.012,
        permeability: 4.0 * std::f64::consts::PI * 1.0e-7,
    });
    SolveMagnetostaticBar1dRequest { nodes, elements }
}

pub(crate) fn generate_acoustic_bar_case(elements: usize) -> SolveAcousticBar1dRequest {
    let nodes = line_nodes(elements, |index, x, end| AcousticBar1dNodeInput {
        id: format!("a{index}"),
        x,
        fix_pressure: index == 0,
        pressure: if index == 0 { 1.0 } else { 0.0 },
        volume_velocity_source: if index == end { 0.003 } else { 0.0 },
    });
    let elements = line_elements(elements, |index| AcousticBar1dElementInput {
        id: format!("ae{index}"),
        node_i: index,
        node_j: index + 1,
        area: 0.08,
        density: 1.225,
        bulk_modulus: 142_000.0,
        damping_ratio: 0.02,
    });
    SolveAcousticBar1dRequest {
        frequency_hz: 440.0,
        nodes,
        elements,
    }
}

pub(crate) fn generate_torsion_case(elements: usize) -> SolveTorsion1dRequest {
    let nodes = line_nodes(elements, |index, x, end| Torsion1dNodeInput {
        id: format!("r{index}"),
        x,
        fix_rz: index == 0,
        torque_z: if index == end { 120.0 } else { 0.0 },
    });
    let elements = line_elements(elements, |index| Torsion1dElementInput {
        id: format!("te{index}"),
        node_i: index,
        node_j: index + 1,
        shear_modulus: 79.3e9,
        polar_moment: 1.6e-6,
        section_modulus: 3.2e-5,
    });
    SolveTorsion1dRequest { nodes, elements }
}

pub(crate) fn generate_heat_triangle_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveHeatPlaneTriangle2dRequest {
    let nodes = scalar_plane_nodes(nx, ny, width, height, |index, x, y, left, right, mid| {
        HeatPlaneNodeInput {
            id: format!("ht{index}"),
            x,
            y,
            fix_temperature: left || right,
            temperature: if left {
                100.0
            } else if right {
                25.0
            } else {
                0.0
            },
            heat_load: if mid { 12.0 } else { 0.0 },
        }
    });
    let elements = triangle_cells(nx, ny, |id, a, b, c| HeatPlaneTriangleElementInput {
        id,
        node_i: a,
        node_j: b,
        node_k: c,
        thickness: 0.02,
        conductivity: 45.0,
    });
    SolveHeatPlaneTriangle2dRequest { nodes, elements }
}

pub(crate) fn generate_electrostatic_quad_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveElectrostaticPlaneQuad2dRequest {
    let nodes = electrostatic_nodes(nx, ny, width, height);
    let elements = quad_cells(nx, ny, |id, a, b, c, d| {
        ElectrostaticPlaneQuadElementInput {
            id,
            node_i: a,
            node_j: b,
            node_k: c,
            node_l: d,
            thickness: 0.01,
            permittivity: 3.0,
        }
    });
    SolveElectrostaticPlaneQuad2dRequest { nodes, elements }
}

pub(crate) fn generate_magnetostatic_quad_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveMagnetostaticPlaneQuad2dRequest {
    let nodes = magnetostatic_nodes(nx, ny, width, height);
    let elements = quad_cells(nx, ny, |id, a, b, c, d| {
        MagnetostaticPlaneQuadElementInput {
            id,
            node_i: a,
            node_j: b,
            node_k: c,
            node_l: d,
            thickness: 0.012,
            permeability: 4.0 * std::f64::consts::PI * 1.0e-7,
        }
    });
    SolveMagnetostaticPlaneQuad2dRequest { nodes, elements }
}

pub(crate) fn generate_stokes_quad_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveStokesFlowPlaneQuad2dRequest {
    let nodes = scalar_plane_nodes(nx, ny, width, height, |index, x, y, left, right, mid| {
        StokesFlowPlaneNodeInput {
            id: format!("s{index}"),
            x,
            y,
            fix_velocity_x: left || right,
            velocity_x: if left { 1.0 } else { 0.0 },
            fix_velocity_y: left || right,
            velocity_y: 0.0,
            fix_pressure: right,
            pressure: 0.0,
            body_force_x: if mid { 0.01 } else { 0.0 },
            body_force_y: 0.0,
        }
    });
    let elements = quad_cells(nx, ny, |id, a, b, c, d| StokesFlowPlaneQuadElementInput {
        id,
        node_i: a,
        node_j: b,
        node_k: c,
        node_l: d,
        thickness: 0.02,
        viscosity: 1.8e-5,
        density: 1.225,
    });
    SolveStokesFlowPlaneQuad2dRequest { nodes, elements }
}

pub(crate) fn generate_electrostatic_triangle_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveElectrostaticPlaneTriangle2dRequest {
    let nodes = electrostatic_nodes(nx, ny, width, height);
    let elements = triangle_cells(nx, ny, |id, a, b, c| {
        ElectrostaticPlaneTriangleElementInput {
            id,
            node_i: a,
            node_j: b,
            node_k: c,
            thickness: 0.01,
            permittivity: 3.0,
        }
    });
    SolveElectrostaticPlaneTriangle2dRequest { nodes, elements }
}

pub(crate) fn generate_magnetostatic_triangle_panel(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolveMagnetostaticPlaneTriangle2dRequest {
    let nodes = magnetostatic_nodes(nx, ny, width, height);
    let elements = triangle_cells(nx, ny, |id, a, b, c| {
        MagnetostaticPlaneTriangleElementInput {
            id,
            node_i: a,
            node_j: b,
            node_k: c,
            thickness: 0.012,
            permeability: 4.0 * std::f64::consts::PI * 1.0e-7,
        }
    });
    SolveMagnetostaticPlaneTriangle2dRequest { nodes, elements }
}

fn line_nodes<T>(elements: usize, mut build: impl FnMut(usize, f64, usize) -> T) -> Vec<T> {
    let length = 20.0;
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

fn line_elements<T>(elements: usize, build: impl Fn(usize) -> T) -> Vec<T> {
    (0..elements).map(build).collect()
}

fn electrostatic_nodes(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> Vec<ElectrostaticPlaneNodeInput> {
    scalar_plane_nodes(nx, ny, width, height, |index, x, y, left, right, mid| {
        ElectrostaticPlaneNodeInput {
            id: format!("ep{index}"),
            x,
            y,
            fix_potential: left || right,
            potential: if left { 12.0 } else { 0.0 },
            charge_density: if mid { 1.0e-6 } else { 0.0 },
        }
    })
}

fn magnetostatic_nodes(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> Vec<MagnetostaticPlaneNodeInput> {
    scalar_plane_nodes(nx, ny, width, height, |index, x, y, left, right, mid| {
        MagnetostaticPlaneNodeInput {
            id: format!("mp{index}"),
            x,
            y,
            fix_vector_potential: left || right,
            vector_potential: if left { 0.2 } else { 0.0 },
            current_density: if mid { 25.0 } else { 0.0 },
        }
    })
}

fn scalar_plane_nodes<T>(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
    mut build: impl FnMut(usize, f64, f64, bool, bool, bool) -> T,
) -> Vec<T> {
    let dx = width / nx.max(1) as f64;
    let dy = height / ny.max(1) as f64;
    let mut nodes = Vec::with_capacity((nx + 1) * (ny + 1));
    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            let mid = i == nx / 2 && j == ny / 2;
            nodes.push(build(
                index,
                i as f64 * dx,
                j as f64 * dy,
                i == 0,
                i == nx,
                mid,
            ));
        }
    }
    nodes
}

fn triangle_cells<T>(
    nx: usize,
    ny: usize,
    mut build: impl FnMut(String, usize, usize, usize) -> T,
) -> Vec<T> {
    let mut elements = Vec::with_capacity(nx * ny * 2);
    for j in 0..ny {
        for i in 0..nx {
            let n0 = j * (nx + 1) + i;
            let n1 = n0 + 1;
            let n2 = n0 + (nx + 1);
            let n3 = n2 + 1;
            let base = j * nx + i;
            elements.push(build(format!("t{base}_a"), n0, n1, n3));
            elements.push(build(format!("t{base}_b"), n0, n3, n2));
        }
    }
    elements
}

fn quad_cells<T>(
    nx: usize,
    ny: usize,
    mut build: impl FnMut(String, usize, usize, usize, usize) -> T,
) -> Vec<T> {
    let mut elements = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let n0 = j * (nx + 1) + i;
            let n1 = n0 + 1;
            let n2 = n0 + (nx + 1);
            let n3 = n2 + 1;
            elements.push(build(format!("q{}", j * nx + i), n0, n1, n3, n2));
        }
    }
    elements
}
