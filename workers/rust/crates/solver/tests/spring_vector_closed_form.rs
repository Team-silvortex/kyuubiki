use kyuubiki_protocol::{
    SolveSpring2dRequest, SolveSpring2dResult, SolveSpring3dRequest, SolveSpring3dResult,
    Spring2dElementInput, Spring2dNodeInput, Spring3dElementInput, Spring3dNodeInput,
};
use kyuubiki_solver::{solve_spring_2d, solve_spring_3d};

const TOL: f64 = 1.0e-10;

#[test]
fn spring_2d_matches_orthogonal_vector_stiffness_closed_form() {
    let load_x = 800.0;
    let load_y = -600.0;
    let stiffness_x = 40_000.0;
    let stiffness_y = 30_000.0;
    let result = solve_spring_2d(&SolveSpring2dRequest {
        nodes: vec![
            node_2d("free", 0.0, 0.0, false, false, load_x, load_y),
            node_2d("fixed-x", 1.0, 0.0, true, true, 0.0, 0.0),
            node_2d("fixed-y", 0.0, 1.0, true, true, 0.0, 0.0),
        ],
        elements: vec![
            element_2d("kx", 0, 1, stiffness_x),
            element_2d("ky", 0, 2, stiffness_y),
        ],
    })
    .expect("2D vector spring closed-form fixture should solve");

    let free = &result.nodes[0];
    let expected_ux = load_x / stiffness_x;
    let expected_uy = load_y / stiffness_y;
    assert_close(free.ux, expected_ux);
    assert_close(free.uy, expected_uy);
    assert_close(result.nodes[1].ux, 0.0);
    assert_close(result.nodes[1].uy, 0.0);
    assert_close(result.nodes[2].ux, 0.0);
    assert_close(result.nodes[2].uy, 0.0);
    assert_close(result.elements[0].force, -load_x);
    assert_close(result.elements[1].force, -load_y);
    assert_close(result.elements[0].extension, -expected_ux);
    assert_close(result.elements[1].extension, -expected_uy);
    assert_close(
        result.total_strain_energy,
        0.5 * (load_x * expected_ux + load_y * expected_uy),
    );
    assert_energy_balance_2d(&result);
}

#[test]
fn spring_3d_matches_orthogonal_vector_stiffness_closed_form() {
    let load_x = 450.0;
    let load_y = -300.0;
    let load_z = 900.0;
    let stiffness_x = 45_000.0;
    let stiffness_y = 30_000.0;
    let stiffness_z = 60_000.0;
    let result = solve_spring_3d(&SolveSpring3dRequest {
        nodes: vec![
            node_3d(
                "free", 0.0, 0.0, 0.0, false, false, false, load_x, load_y, load_z,
            ),
            node_3d("fixed-x", 1.0, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            node_3d("fixed-y", 0.0, 1.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            node_3d("fixed-z", 0.0, 0.0, 1.0, true, true, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![
            element_3d("kx", 0, 1, stiffness_x),
            element_3d("ky", 0, 2, stiffness_y),
            element_3d("kz", 0, 3, stiffness_z),
        ],
    })
    .expect("3D vector spring closed-form fixture should solve");

    let free = &result.nodes[0];
    let expected_ux = load_x / stiffness_x;
    let expected_uy = load_y / stiffness_y;
    let expected_uz = load_z / stiffness_z;
    assert_close(free.ux, expected_ux);
    assert_close(free.uy, expected_uy);
    assert_close(free.uz, expected_uz);
    for (index, fixed) in result.nodes[1..].iter().enumerate() {
        assert_eq!(fixed.index, index + 1);
        assert_close(fixed.ux, 0.0);
        assert_close(fixed.uy, 0.0);
        assert_close(fixed.uz, 0.0);
    }
    assert_close(result.elements[0].force, -load_x);
    assert_close(result.elements[1].force, -load_y);
    assert_close(result.elements[2].force, -load_z);
    assert_close(result.elements[0].extension, -expected_ux);
    assert_close(result.elements[1].extension, -expected_uy);
    assert_close(result.elements[2].extension, -expected_uz);
    assert_close(
        result.total_strain_energy,
        0.5 * (load_x * expected_ux + load_y * expected_uy + load_z * expected_uz),
    );
    assert_energy_balance_3d(&result);
}

#[test]
fn spring_2d_tracks_load_and_stiffness_scaling() {
    let baseline = solve_spring_2d(&spring_2d_request(700.0, -500.0, 35_000.0, 25_000.0))
        .expect("baseline 2D vector spring should solve");
    assert_energy_balance_2d(&baseline);

    let load_scale = 1.4;
    let load_scaled = solve_spring_2d(&spring_2d_request(
        700.0 * load_scale,
        -500.0 * load_scale,
        35_000.0,
        25_000.0,
    ))
    .expect("load-scaled 2D vector spring should solve");
    assert_close(load_scaled.nodes[0].ux / baseline.nodes[0].ux, load_scale);
    assert_close(load_scaled.nodes[0].uy / baseline.nodes[0].uy, load_scale);
    assert_close(
        load_scaled.elements[0].force / baseline.elements[0].force,
        load_scale,
    );
    assert_close(
        load_scaled.elements[1].force / baseline.elements[1].force,
        load_scale,
    );
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
    );
    assert_energy_balance_2d(&load_scaled);

    let stiffness_scale = 1.6;
    let stiffness_scaled = solve_spring_2d(&spring_2d_request(
        700.0,
        -500.0,
        35_000.0 * stiffness_scale,
        25_000.0 * stiffness_scale,
    ))
    .expect("stiffness-scaled 2D vector spring should solve");
    assert_close(
        stiffness_scaled.nodes[0].ux / baseline.nodes[0].ux,
        1.0 / stiffness_scale,
    );
    assert_close(
        stiffness_scaled.nodes[0].uy / baseline.nodes[0].uy,
        1.0 / stiffness_scale,
    );
    assert_close(
        stiffness_scaled.elements[0].force,
        baseline.elements[0].force,
    );
    assert_close(
        stiffness_scaled.elements[1].force,
        baseline.elements[1].force,
    );
    assert_close(
        stiffness_scaled.total_strain_energy / baseline.total_strain_energy,
        1.0 / stiffness_scale,
    );
    assert_energy_balance_2d(&stiffness_scaled);

    let geometry_scale = 2.0;
    let longer = solve_spring_2d(&spring_2d_request_scaled(
        geometry_scale,
        700.0,
        -500.0,
        35_000.0,
        25_000.0,
    ))
    .expect("geometry-scaled 2D vector spring should solve");
    assert_close(
        longer.elements[0].length / baseline.elements[0].length,
        geometry_scale,
    );
    assert_close(
        longer.elements[1].length / baseline.elements[1].length,
        geometry_scale,
    );
    assert_close(longer.nodes[0].ux, baseline.nodes[0].ux);
    assert_close(longer.nodes[0].uy, baseline.nodes[0].uy);
    assert_close(longer.elements[0].force, baseline.elements[0].force);
    assert_close(longer.elements[1].force, baseline.elements[1].force);
    assert_close(longer.total_strain_energy, baseline.total_strain_energy);
    assert_energy_balance_2d(&longer);
}

#[test]
fn spring_3d_tracks_load_and_stiffness_scaling() {
    let baseline = solve_spring_3d(&spring_3d_request(
        420.0, -280.0, 700.0, 42_000.0, 28_000.0, 56_000.0,
    ))
    .expect("baseline 3D vector spring should solve");
    assert_energy_balance_3d(&baseline);

    let load_scale = 1.5;
    let load_scaled = solve_spring_3d(&spring_3d_request(
        420.0 * load_scale,
        -280.0 * load_scale,
        700.0 * load_scale,
        42_000.0,
        28_000.0,
        56_000.0,
    ))
    .expect("load-scaled 3D vector spring should solve");
    assert_close(load_scaled.nodes[0].ux / baseline.nodes[0].ux, load_scale);
    assert_close(load_scaled.nodes[0].uy / baseline.nodes[0].uy, load_scale);
    assert_close(load_scaled.nodes[0].uz / baseline.nodes[0].uz, load_scale);
    assert_close(load_scaled.max_force / baseline.max_force, load_scale);
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
    );
    assert_energy_balance_3d(&load_scaled);

    let stiffness_scale = 1.75;
    let stiffness_scaled = solve_spring_3d(&spring_3d_request(
        420.0,
        -280.0,
        700.0,
        42_000.0 * stiffness_scale,
        28_000.0 * stiffness_scale,
        56_000.0 * stiffness_scale,
    ))
    .expect("stiffness-scaled 3D vector spring should solve");
    assert_close(
        stiffness_scaled.nodes[0].ux / baseline.nodes[0].ux,
        1.0 / stiffness_scale,
    );
    assert_close(
        stiffness_scaled.nodes[0].uy / baseline.nodes[0].uy,
        1.0 / stiffness_scale,
    );
    assert_close(
        stiffness_scaled.nodes[0].uz / baseline.nodes[0].uz,
        1.0 / stiffness_scale,
    );
    assert_close(stiffness_scaled.max_force, baseline.max_force);
    assert_close(
        stiffness_scaled.total_strain_energy / baseline.total_strain_energy,
        1.0 / stiffness_scale,
    );
    assert_energy_balance_3d(&stiffness_scaled);

    let geometry_scale = 1.8;
    let longer = solve_spring_3d(&spring_3d_request_scaled(
        geometry_scale,
        420.0,
        -280.0,
        700.0,
        42_000.0,
        28_000.0,
        56_000.0,
    ))
    .expect("geometry-scaled 3D vector spring should solve");
    assert_close(
        longer.elements[0].length / baseline.elements[0].length,
        geometry_scale,
    );
    assert_close(
        longer.elements[1].length / baseline.elements[1].length,
        geometry_scale,
    );
    assert_close(
        longer.elements[2].length / baseline.elements[2].length,
        geometry_scale,
    );
    assert_close(longer.nodes[0].ux, baseline.nodes[0].ux);
    assert_close(longer.nodes[0].uy, baseline.nodes[0].uy);
    assert_close(longer.nodes[0].uz, baseline.nodes[0].uz);
    assert_close(longer.max_force, baseline.max_force);
    assert_close(longer.total_strain_energy, baseline.total_strain_energy);
    assert_energy_balance_3d(&longer);
}

fn spring_2d_request(
    load_x: f64,
    load_y: f64,
    stiffness_x: f64,
    stiffness_y: f64,
) -> SolveSpring2dRequest {
    spring_2d_request_scaled(1.0, load_x, load_y, stiffness_x, stiffness_y)
}

fn spring_2d_request_scaled(
    geometry_scale: f64,
    load_x: f64,
    load_y: f64,
    stiffness_x: f64,
    stiffness_y: f64,
) -> SolveSpring2dRequest {
    SolveSpring2dRequest {
        nodes: vec![
            node_2d("free", 0.0, 0.0, false, false, load_x, load_y),
            node_2d("fixed-x", geometry_scale, 0.0, true, true, 0.0, 0.0),
            node_2d("fixed-y", 0.0, geometry_scale, true, true, 0.0, 0.0),
        ],
        elements: vec![
            element_2d("kx", 0, 1, stiffness_x),
            element_2d("ky", 0, 2, stiffness_y),
        ],
    }
}

fn spring_3d_request(
    load_x: f64,
    load_y: f64,
    load_z: f64,
    stiffness_x: f64,
    stiffness_y: f64,
    stiffness_z: f64,
) -> SolveSpring3dRequest {
    spring_3d_request_scaled(
        1.0,
        load_x,
        load_y,
        load_z,
        stiffness_x,
        stiffness_y,
        stiffness_z,
    )
}

fn spring_3d_request_scaled(
    geometry_scale: f64,
    load_x: f64,
    load_y: f64,
    load_z: f64,
    stiffness_x: f64,
    stiffness_y: f64,
    stiffness_z: f64,
) -> SolveSpring3dRequest {
    SolveSpring3dRequest {
        nodes: vec![
            node_3d(
                "free", 0.0, 0.0, 0.0, false, false, false, load_x, load_y, load_z,
            ),
            node_3d(
                "fixed-x",
                geometry_scale,
                0.0,
                0.0,
                true,
                true,
                true,
                0.0,
                0.0,
                0.0,
            ),
            node_3d(
                "fixed-y",
                0.0,
                geometry_scale,
                0.0,
                true,
                true,
                true,
                0.0,
                0.0,
                0.0,
            ),
            node_3d(
                "fixed-z",
                0.0,
                0.0,
                geometry_scale,
                true,
                true,
                true,
                0.0,
                0.0,
                0.0,
            ),
        ],
        elements: vec![
            element_3d("kx", 0, 1, stiffness_x),
            element_3d("ky", 0, 2, stiffness_y),
            element_3d("kz", 0, 3, stiffness_z),
        ],
    }
}

fn node_2d(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> Spring2dNodeInput {
    Spring2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

fn element_2d(id: &str, node_i: usize, node_j: usize, stiffness: f64) -> Spring2dElementInput {
    Spring2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

#[allow(clippy::too_many_arguments)]
fn node_3d(
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
) -> Spring3dNodeInput {
    Spring3dNodeInput {
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
    }
}

fn element_3d(id: &str, node_i: usize, node_j: usize, stiffness: f64) -> Spring3dElementInput {
    Spring3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

fn assert_energy_balance_2d(result: &SolveSpring2dResult) {
    assert_spring_summary_2d(result);
    assert_close(
        result.total_strain_energy,
        result
            .elements
            .iter()
            .map(|element| element.strain_energy)
            .sum(),
    );
    let external_work = result
        .input
        .nodes
        .iter()
        .zip(result.nodes.iter())
        .map(|(input, result)| input.load_x * result.ux + input.load_y * result.uy)
        .sum::<f64>();
    assert_close(result.total_strain_energy, 0.5 * external_work);
}

fn assert_spring_summary_2d(result: &SolveSpring2dResult) {
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());
    let max_displacement = result
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let input = &result.input.nodes[index];
            assert_eq!(node.index, index);
            assert_eq!(node.id, input.id);
            assert_close(node.x, input.x);
            assert_close(node.y, input.y);
            hypot2(node.ux, node.uy)
        })
        .fold(0.0, f64::max);
    assert_close(result.max_displacement, max_displacement);

    let mut max_force: f64 = 0.0;
    let mut total_energy = 0.0;
    for (index, element) in result.elements.iter().enumerate() {
        let input = &result.input.elements[index];
        assert_eq!(element.index, index);
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = hypot2(dx, dy);
        let expected_extension =
            ((node_j.ux - node_i.ux) * dx + (node_j.uy - node_i.uy) * dy) / length;
        let expected_force = input.stiffness * expected_extension;
        let expected_energy = 0.5 * expected_force * expected_extension;
        assert_close(element.length, length);
        assert_close(element.extension, expected_extension);
        assert_close(element.force, expected_force);
        assert_close(element.strain_energy, expected_energy);
        max_force = max_force.max(element.force.abs());
        total_energy += element.strain_energy;
    }
    assert_close(result.max_force, max_force);
    assert_close(result.total_strain_energy, total_energy);
}

fn assert_energy_balance_3d(result: &SolveSpring3dResult) {
    assert_spring_summary_3d(result);
    assert_close(
        result.total_strain_energy,
        result
            .elements
            .iter()
            .map(|element| element.strain_energy)
            .sum(),
    );
    let external_work = result
        .input
        .nodes
        .iter()
        .zip(result.nodes.iter())
        .map(|(input, result)| {
            input.load_x * result.ux + input.load_y * result.uy + input.load_z * result.uz
        })
        .sum::<f64>();
    assert_close(result.total_strain_energy, 0.5 * external_work);
}

fn assert_spring_summary_3d(result: &SolveSpring3dResult) {
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());
    let max_displacement = result
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let input = &result.input.nodes[index];
            assert_eq!(node.index, index);
            assert_eq!(node.id, input.id);
            assert_close(node.x, input.x);
            assert_close(node.y, input.y);
            assert_close(node.z, input.z);
            hypot3(node.ux, node.uy, node.uz)
        })
        .fold(0.0, f64::max);
    assert_close(result.max_displacement, max_displacement);

    let mut max_force: f64 = 0.0;
    let mut total_energy = 0.0;
    for (index, element) in result.elements.iter().enumerate() {
        let input = &result.input.elements[index];
        assert_eq!(element.index, index);
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = hypot3(dx, dy, dz);
        let expected_extension = ((node_j.ux - node_i.ux) * dx
            + (node_j.uy - node_i.uy) * dy
            + (node_j.uz - node_i.uz) * dz)
            / length;
        let expected_force = input.stiffness * expected_extension;
        let expected_energy = 0.5 * expected_force * expected_extension;
        assert_close(element.length, length);
        assert_close(element.extension, expected_extension);
        assert_close(element.force, expected_force);
        assert_close(element.strain_energy, expected_energy);
        max_force = max_force.max(element.force.abs());
        total_energy += element.strain_energy;
    }
    assert_close(result.max_force, max_force);
    assert_close(result.total_strain_energy, total_energy);
}

fn hypot2(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    (x * x + y * y + z * z).sqrt()
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
