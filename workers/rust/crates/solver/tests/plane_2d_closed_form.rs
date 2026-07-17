use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, PlaneTriangleElementInput, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_plane_quad_2d, solve_plane_triangle_2d};

#[test]
fn plane_triangle_2d_matches_retained_patch_stiffness_reference() {
    let result = solve_plane_triangle_2d(&triangle_patch()).expect("triangle patch should solve");

    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[3].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[2].ux, 4.714_285_714_285_715e-7, 1.0e-12);
    assert_close(result.nodes[2].uy, -1.428_571_428_571_429e-6, 1.0e-12);
    assert_close(result.max_displacement, 1.504_347_441_414_315e-6, 1.0e-12);
    assert_close(result.max_stress, 100_000.0, 1.0e-10);

    let mut expected_total_energy = 0.0;
    for (element, input) in result.elements.iter().zip(result.input.elements.iter()) {
        assert_close(element.area, 0.5, 1.0e-12);
        assert_planar_metrics(
            element.stress_x,
            element.stress_y,
            element.tau_xy,
            element.principal_stress_1,
            element.principal_stress_2,
            element.max_in_plane_shear,
            element.von_mises,
        );
        let expected_density = 0.5
            * ((element.stress_x * element.strain_x)
                + (element.stress_y * element.strain_y)
                + (element.tau_xy * element.gamma_xy));
        assert_close(element.strain_energy_density, expected_density, 1.0e-12);
        expected_total_energy += expected_density * element.area * input.thickness;
    }
    assert_close(result.elements[0].von_mises, 100_000.0, 1.0e-10);
    assert_close(result.total_strain_energy, expected_total_energy, 1.0e-12);
    assert_external_work_energy(
        &result.input.nodes,
        &result.nodes,
        result.total_strain_energy,
    );
    assert_close(
        result.max_strain_energy_density,
        result
            .elements
            .iter()
            .map(|element| element.strain_energy_density.abs())
            .fold(0.0_f64, f64::max),
        1.0e-12,
    );
}

#[test]
fn plane_quad_2d_matches_retained_split_triangle_patch_reference() {
    let result = solve_plane_quad_2d(&quad_patch()).expect("quad patch should solve");

    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[3].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[2].ux, 2.576_145_151_695_419e-7, 1.0e-12);
    assert_close(result.nodes[2].uy, -4.670_094_331_605_336_6e-7, 1.0e-12);
    assert_close(result.max_displacement, 5.333_507_749_004_975e-7, 1.0e-12);
    assert_close(result.max_stress, 126_981.385_278_360_32, 1.0e-10);

    let element = &result.elements[0];
    assert_close(element.area, 0.8, 1.0e-12);
    assert_close(element.stress_x, 12_500.0, 1.0e-10);
    assert_close(element.stress_y, -120_000.0, 1.0e-10);
    assert_close(element.tau_xy, 3_048.780_487_804_874_6, 1.0e-10);
    assert!(element.principal_stress_1 >= element.principal_stress_2);
    assert!(element.max_in_plane_shear >= 0.0);
    assert_close(element.von_mises, result.max_stress, 1.0e-12);
    assert_close(
        result.total_strain_energy,
        element.strain_energy_density * element.area * result.input.elements[0].thickness,
        1.0e-12,
    );
    assert_external_work_energy(
        &result.input.nodes,
        &result.nodes,
        result.total_strain_energy,
    );
    assert_close(
        result.max_strain_energy_density,
        element.strain_energy_density.abs(),
        1.0e-12,
    );
}

#[test]
fn plane_triangle_2d_tracks_load_and_thickness_scaling() {
    let baseline = solve_plane_triangle_2d(&triangle_patch()).expect("triangle baseline");
    assert_external_work_energy(
        &baseline.input.nodes,
        &baseline.nodes,
        baseline.total_strain_energy,
    );

    let load_scale = 1.5;
    let mut load_request = triangle_patch();
    for node in &mut load_request.nodes {
        node.load_x *= load_scale;
        node.load_y *= load_scale;
    }
    let load_scaled = solve_plane_triangle_2d(&load_request).expect("load-scaled triangle patch");
    assert_close(
        load_scaled.nodes[2].uy / baseline.nodes[2].uy,
        load_scale,
        1.0e-10,
    );
    assert_close(
        load_scaled.max_stress / baseline.max_stress,
        load_scale,
        1.0e-10,
    );
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
        1.0e-10,
    );
    assert_external_work_energy(
        &load_scaled.input.nodes,
        &load_scaled.nodes,
        load_scaled.total_strain_energy,
    );

    let thickness_scale = 1.6;
    let mut thick_request = triangle_patch();
    for element in &mut thick_request.elements {
        element.thickness *= thickness_scale;
    }
    let thickened =
        solve_plane_triangle_2d(&thick_request).expect("thickness-scaled triangle patch");
    assert_close(
        thickened.nodes[2].uy / baseline.nodes[2].uy,
        1.0 / thickness_scale,
        1.0e-10,
    );
    assert_close(
        thickened.max_stress / baseline.max_stress,
        1.0 / thickness_scale,
        1.0e-10,
    );
    assert_close(
        thickened.total_strain_energy / baseline.total_strain_energy,
        1.0 / thickness_scale,
        1.0e-10,
    );
    assert_external_work_energy(
        &thickened.input.nodes,
        &thickened.nodes,
        thickened.total_strain_energy,
    );

    let modulus_scale = 1.4;
    let mut stiff_request = triangle_patch();
    for element in &mut stiff_request.elements {
        element.youngs_modulus *= modulus_scale;
    }
    let stiffened = solve_plane_triangle_2d(&stiff_request).expect("modulus-scaled triangle patch");
    assert_close(
        stiffened.nodes[2].uy / baseline.nodes[2].uy,
        1.0 / modulus_scale,
        1.0e-10,
    );
    assert_close(stiffened.max_stress, baseline.max_stress, 1.0e-10);
    assert_close(
        stiffened.total_strain_energy / baseline.total_strain_energy,
        1.0 / modulus_scale,
        1.0e-10,
    );
    assert_external_work_energy(
        &stiffened.input.nodes,
        &stiffened.nodes,
        stiffened.total_strain_energy,
    );

    let geometry_scale = 1.5;
    let scaled_geometry = solve_plane_triangle_2d(&triangle_patch_scaled(geometry_scale))
        .expect("geometry-scaled triangle patch");
    assert_close(
        scaled_geometry.elements[0].area / baseline.elements[0].area,
        geometry_scale * geometry_scale,
        1.0e-10,
    );
    assert_close(scaled_geometry.nodes[2].uy, baseline.nodes[2].uy, 1.0e-10);
    assert_close(
        scaled_geometry.max_stress / baseline.max_stress,
        1.0 / geometry_scale,
        1.0e-10,
    );
    assert_close(
        scaled_geometry.total_strain_energy,
        baseline.total_strain_energy,
        1.0e-10,
    );
    assert_external_work_energy(
        &scaled_geometry.input.nodes,
        &scaled_geometry.nodes,
        scaled_geometry.total_strain_energy,
    );
}

#[test]
fn plane_quad_2d_tracks_load_and_modulus_scaling() {
    let baseline = solve_plane_quad_2d(&quad_patch()).expect("quad baseline");
    assert_external_work_energy(
        &baseline.input.nodes,
        &baseline.nodes,
        baseline.total_strain_energy,
    );

    let load_scale = 1.35;
    let mut load_request = quad_patch();
    for node in &mut load_request.nodes {
        node.load_x *= load_scale;
        node.load_y *= load_scale;
    }
    let load_scaled = solve_plane_quad_2d(&load_request).expect("load-scaled quad patch");
    assert_close(
        load_scaled.nodes[2].uy / baseline.nodes[2].uy,
        load_scale,
        1.0e-10,
    );
    assert_close(
        load_scaled.max_stress / baseline.max_stress,
        load_scale,
        1.0e-10,
    );
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
        1.0e-10,
    );
    assert_external_work_energy(
        &load_scaled.input.nodes,
        &load_scaled.nodes,
        load_scaled.total_strain_energy,
    );

    let modulus_scale = 1.7;
    let mut stiff_request = quad_patch();
    stiff_request.elements[0].youngs_modulus *= modulus_scale;
    let stiffened = solve_plane_quad_2d(&stiff_request).expect("modulus-scaled quad patch");
    assert_close(
        stiffened.nodes[2].uy / baseline.nodes[2].uy,
        1.0 / modulus_scale,
        1.0e-10,
    );
    assert_close(stiffened.max_stress, baseline.max_stress, 1.0e-10);
    assert_close(
        stiffened.total_strain_energy / baseline.total_strain_energy,
        1.0 / modulus_scale,
        1.0e-10,
    );
    assert_external_work_energy(
        &stiffened.input.nodes,
        &stiffened.nodes,
        stiffened.total_strain_energy,
    );

    let thickness_scale = 1.8;
    let mut thick_request = quad_patch();
    thick_request.elements[0].thickness *= thickness_scale;
    let thickened = solve_plane_quad_2d(&thick_request).expect("thickness-scaled quad patch");
    assert_close(
        thickened.nodes[2].uy / baseline.nodes[2].uy,
        1.0 / thickness_scale,
        1.0e-10,
    );
    assert_close(
        thickened.max_stress / baseline.max_stress,
        1.0 / thickness_scale,
        1.0e-10,
    );
    assert_close(
        thickened.total_strain_energy / baseline.total_strain_energy,
        1.0 / thickness_scale,
        1.0e-10,
    );
    assert_external_work_energy(
        &thickened.input.nodes,
        &thickened.nodes,
        thickened.total_strain_energy,
    );

    let geometry_scale = 1.4;
    let scaled_geometry =
        solve_plane_quad_2d(&quad_patch_scaled(geometry_scale)).expect("geometry-scaled quad");
    assert_close(
        scaled_geometry.elements[0].area / baseline.elements[0].area,
        geometry_scale * geometry_scale,
        1.0e-10,
    );
    assert_close(scaled_geometry.nodes[2].uy, baseline.nodes[2].uy, 1.0e-10);
    assert_close(
        scaled_geometry.max_stress / baseline.max_stress,
        1.0 / geometry_scale,
        1.0e-10,
    );
    assert_close(
        scaled_geometry.total_strain_energy,
        baseline.total_strain_energy,
        1.0e-10,
    );
    assert_external_work_energy(
        &scaled_geometry.input.nodes,
        &scaled_geometry.nodes,
        scaled_geometry.total_strain_energy,
    );
}

fn triangle_patch() -> SolvePlaneTriangle2dRequest {
    triangle_patch_scaled(1.0)
}

fn triangle_patch_scaled(geometry_scale: f64) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: vec![
            node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            node("bottom_right", geometry_scale, 0.0, false, true, 0.0, 0.0),
            node(
                "top_right",
                geometry_scale,
                geometry_scale,
                false,
                false,
                0.0,
                -1000.0,
            ),
            node("top_left", 0.0, geometry_scale, true, false, 0.0, -1000.0),
        ],
        elements: vec![
            triangle("tri_lower", 0, 1, 2),
            triangle("tri_upper", 0, 2, 3),
        ],
    }
}

fn quad_patch() -> SolvePlaneQuad2dRequest {
    quad_patch_scaled(1.0)
}

fn quad_patch_scaled(geometry_scale: f64) -> SolvePlaneQuad2dRequest {
    SolvePlaneQuad2dRequest {
        nodes: vec![
            node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            node("bottom_right", geometry_scale, 0.0, false, true, 0.0, 0.0),
            node(
                "top_right",
                geometry_scale,
                0.8 * geometry_scale,
                false,
                false,
                200.0,
                -1200.0,
            ),
            node(
                "top_left",
                0.0,
                0.8 * geometry_scale,
                true,
                false,
                200.0,
                -1200.0,
            ),
        ],
        elements: vec![PlaneQuadElementInput {
            id: "quad_panel".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 210.0e9,
            poisson_ratio: 0.3,
        }],
    }
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> PlaneNodeInput {
    PlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

fn triangle(id: &str, node_i: usize, node_j: usize, node_k: usize) -> PlaneTriangleElementInput {
    PlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02,
        youngs_modulus: 70.0e9,
        poisson_ratio: 0.33,
    }
}

fn assert_planar_metrics(
    sigma_x: f64,
    sigma_y: f64,
    tau_xy: f64,
    principal_1: f64,
    principal_2: f64,
    max_shear: f64,
    von_mises: f64,
) {
    let center = 0.5 * (sigma_x + sigma_y);
    let radius = (((0.5 * (sigma_x - sigma_y)).powi(2)) + tau_xy.powi(2)).sqrt();
    assert_close(principal_1, center + radius, 1.0e-12);
    assert_close(principal_2, center - radius, 1.0e-12);
    assert_close(max_shear, radius, 1.0e-12);
    assert_close(
        von_mises,
        ((sigma_x * sigma_x) - (sigma_x * sigma_y) + (sigma_y * sigma_y) + 3.0 * tau_xy * tau_xy)
            .sqrt(),
        1.0e-12,
    );
}

fn assert_external_work_energy(
    input_nodes: &[PlaneNodeInput],
    result_nodes: &[kyuubiki_protocol::PlaneNodeResult],
    total_strain_energy: f64,
) {
    let external_work = input_nodes
        .iter()
        .zip(result_nodes.iter())
        .map(|(input, result)| input.load_x * result.ux + input.load_y * result.uy)
        .sum::<f64>();
    assert_close(total_strain_energy, 0.5 * external_work, 1.0e-10);
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
