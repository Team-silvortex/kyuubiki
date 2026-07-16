use super::common::assert_close;
use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, PlaneTriangleElementInput, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_plane_quad_2d, solve_plane_triangle_2d};

#[test]
fn plane_triangle_2d_load_material_and_thickness_scaling_is_linear() {
    let base = solve_plane_triangle_2d(&triangle_patch(1.0, 1.0, 1.0))
        .expect("base triangle patch should solve");

    for case in [
        PlaneScaleCase {
            load_factor: 2.5,
            young_factor: 1.0,
            thickness_factor: 1.0,
        },
        PlaneScaleCase {
            load_factor: 1.0,
            young_factor: 3.0,
            thickness_factor: 1.0,
        },
        PlaneScaleCase {
            load_factor: 1.0,
            young_factor: 1.0,
            thickness_factor: 2.0,
        },
        PlaneScaleCase {
            load_factor: 1.7,
            young_factor: 2.3,
            thickness_factor: 1.4,
        },
    ] {
        let result = solve_plane_triangle_2d(&triangle_patch(
            case.load_factor,
            case.young_factor,
            case.thickness_factor,
        ))
        .expect("scaled triangle patch should solve");

        let displacement_scale = case.load_factor / (case.young_factor * case.thickness_factor);
        let stress_scale = case.load_factor / case.thickness_factor;
        let energy_density_scale =
            case.load_factor.powi(2) / (case.young_factor * case.thickness_factor.powi(2));
        let total_energy_scale =
            case.load_factor.powi(2) / (case.young_factor * case.thickness_factor);

        assert_plane_node_scaling(&base.nodes, &result.nodes, displacement_scale, "triangle");
        assert_close(
            result.max_displacement,
            base.max_displacement * displacement_scale,
            "triangle max displacement scaling",
        );
        assert_close(
            result.max_stress,
            base.max_stress * stress_scale,
            "triangle max stress scaling",
        );
        assert_close(
            result.total_strain_energy,
            base.total_strain_energy * total_energy_scale,
            "triangle total energy scaling",
        );
        assert_close(
            result.max_strain_energy_density,
            base.max_strain_energy_density * energy_density_scale,
            "triangle max energy density scaling",
        );

        for (base_element, element) in base.elements.iter().zip(result.elements.iter()) {
            assert_close(element.area, base_element.area, "triangle area invariant");
            assert_scaled_plane_quantity(
                element.strain_x,
                base_element.strain_x * displacement_scale,
                "triangle strain x scaling",
            );
            assert_scaled_plane_quantity(
                element.strain_y,
                base_element.strain_y * displacement_scale,
                "triangle strain y scaling",
            );
            assert_scaled_plane_quantity(
                element.gamma_xy,
                base_element.gamma_xy * displacement_scale,
                "triangle gamma xy scaling",
            );
            assert_scaled_plane_quantity(
                element.stress_x,
                base_element.stress_x * stress_scale,
                "triangle stress x scaling",
            );
            assert_scaled_plane_quantity(
                element.stress_y,
                base_element.stress_y * stress_scale,
                "triangle stress y scaling",
            );
            assert_scaled_plane_quantity(
                element.tau_xy,
                base_element.tau_xy * stress_scale,
                "triangle tau xy scaling",
            );
            assert_close(
                element.von_mises,
                base_element.von_mises * stress_scale,
                "triangle von mises scaling",
            );
            assert_close(
                element.strain_energy_density,
                base_element.strain_energy_density * energy_density_scale,
                "triangle energy density scaling",
            );
        }
    }
}

#[test]
fn plane_quad_2d_load_material_and_thickness_scaling_is_linear() {
    let base =
        solve_plane_quad_2d(&quad_patch(1.0, 1.0, 1.0)).expect("base quad patch should solve");

    for case in [
        PlaneScaleCase {
            load_factor: 2.0,
            young_factor: 1.0,
            thickness_factor: 1.0,
        },
        PlaneScaleCase {
            load_factor: 1.0,
            young_factor: 2.5,
            thickness_factor: 1.0,
        },
        PlaneScaleCase {
            load_factor: 1.0,
            young_factor: 1.0,
            thickness_factor: 1.8,
        },
        PlaneScaleCase {
            load_factor: 1.6,
            young_factor: 2.2,
            thickness_factor: 1.3,
        },
    ] {
        let result = solve_plane_quad_2d(&quad_patch(
            case.load_factor,
            case.young_factor,
            case.thickness_factor,
        ))
        .expect("scaled quad patch should solve");

        let displacement_scale = case.load_factor / (case.young_factor * case.thickness_factor);
        let stress_scale = case.load_factor / case.thickness_factor;
        let energy_density_scale =
            case.load_factor.powi(2) / (case.young_factor * case.thickness_factor.powi(2));
        let total_energy_scale =
            case.load_factor.powi(2) / (case.young_factor * case.thickness_factor);

        assert_plane_node_scaling(&base.nodes, &result.nodes, displacement_scale, "quad");
        assert_close(
            result.max_displacement,
            base.max_displacement * displacement_scale,
            "quad max displacement scaling",
        );
        assert_close(
            result.max_stress,
            base.max_stress * stress_scale,
            "quad max stress scaling",
        );
        assert_close(
            result.total_strain_energy,
            base.total_strain_energy * total_energy_scale,
            "quad total energy scaling",
        );
        assert_close(
            result.max_strain_energy_density,
            base.max_strain_energy_density * energy_density_scale,
            "quad max energy density scaling",
        );

        let base_element = &base.elements[0];
        let element = &result.elements[0];
        assert_close(element.area, base_element.area, "quad area invariant");
        assert_scaled_plane_quantity(
            element.strain_x,
            base_element.strain_x * displacement_scale,
            "quad strain x scaling",
        );
        assert_scaled_plane_quantity(
            element.strain_y,
            base_element.strain_y * displacement_scale,
            "quad strain y scaling",
        );
        assert_scaled_plane_quantity(
            element.gamma_xy,
            base_element.gamma_xy * displacement_scale,
            "quad gamma xy scaling",
        );
        assert_scaled_plane_quantity(
            element.stress_x,
            base_element.stress_x * stress_scale,
            "quad stress x scaling",
        );
        assert_scaled_plane_quantity(
            element.stress_y,
            base_element.stress_y * stress_scale,
            "quad stress y scaling",
        );
        assert_scaled_plane_quantity(
            element.tau_xy,
            base_element.tau_xy * stress_scale,
            "quad tau xy scaling",
        );
        assert_close(
            element.von_mises,
            base_element.von_mises * stress_scale,
            "quad von mises scaling",
        );
        assert_close(
            element.strain_energy_density,
            base_element.strain_energy_density * energy_density_scale,
            "quad energy density scaling",
        );
    }
}

#[derive(Clone, Copy)]
struct PlaneScaleCase {
    load_factor: f64,
    young_factor: f64,
    thickness_factor: f64,
}

fn triangle_patch(
    load_factor: f64,
    young_factor: f64,
    thickness_factor: f64,
) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: vec![
            plane_node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            plane_node("bottom_right", 1.0, 0.0, false, true, 0.0, 0.0),
            plane_node(
                "top_right",
                1.0,
                1.0,
                false,
                false,
                0.0,
                -1000.0 * load_factor,
            ),
            plane_node(
                "top_left",
                0.0,
                1.0,
                true,
                false,
                0.0,
                -1000.0 * load_factor,
            ),
        ],
        elements: vec![
            triangle_element("tri_lower", 0, 1, 2, young_factor, thickness_factor),
            triangle_element("tri_upper", 0, 2, 3, young_factor, thickness_factor),
        ],
    }
}

fn quad_patch(
    load_factor: f64,
    young_factor: f64,
    thickness_factor: f64,
) -> SolvePlaneQuad2dRequest {
    SolvePlaneQuad2dRequest {
        nodes: vec![
            plane_node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            plane_node("bottom_right", 1.0, 0.0, false, true, 0.0, 0.0),
            plane_node(
                "top_right",
                1.0,
                0.8,
                false,
                false,
                200.0 * load_factor,
                -1200.0 * load_factor,
            ),
            plane_node(
                "top_left",
                0.0,
                0.8,
                true,
                false,
                200.0 * load_factor,
                -1200.0 * load_factor,
            ),
        ],
        elements: vec![PlaneQuadElementInput {
            id: "quad_panel".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02 * thickness_factor,
            youngs_modulus: 210.0e9 * young_factor,
            poisson_ratio: 0.3,
        }],
    }
}

fn plane_node(
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

fn triangle_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    young_factor: f64,
    thickness_factor: f64,
) -> PlaneTriangleElementInput {
    PlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02 * thickness_factor,
        youngs_modulus: 70.0e9 * young_factor,
        poisson_ratio: 0.33,
    }
}

fn assert_plane_node_scaling(
    base: &[kyuubiki_protocol::PlaneNodeResult],
    scaled: &[kyuubiki_protocol::PlaneNodeResult],
    displacement_scale: f64,
    label: &str,
) {
    for (base_node, node) in base.iter().zip(scaled.iter()) {
        assert_close(
            node.ux,
            base_node.ux * displacement_scale,
            &format!("{label} node ux scaling"),
        );
        assert_close(
            node.uy,
            base_node.uy * displacement_scale,
            &format!("{label} node uy scaling"),
        );
        assert_close(
            node.displacement_magnitude,
            base_node.displacement_magnitude * displacement_scale,
            &format!("{label} node displacement magnitude scaling"),
        );
    }
}

fn assert_scaled_plane_quantity(actual: f64, expected: f64, label: &str) {
    if expected.abs() < 1.0e-9 {
        assert!(
            actual.abs() < 1.0e-9,
            "{label}: expected near-zero value, got {actual}",
        );
    } else {
        assert_close(actual, expected, label);
    }
}
