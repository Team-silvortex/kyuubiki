use kyuubiki_solver::*;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug)]
enum Field {
    Heat,
    Electric,
    Magnetic,
}

impl Field {
    fn keys(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Self::Heat => (
                "temperature",
                "fix_temperature",
                "conductivity",
                "temperature_gradient",
            ),
            Self::Electric => (
                "potential",
                "fix_potential",
                "permittivity",
                "potential_gradient",
            ),
            Self::Magnetic => (
                "vector_potential",
                "fix_vector_potential",
                "permeability",
                "vector_potential_gradient",
            ),
        }
    }
}

fn mesh(field: Field, quad: bool, divisions: usize, offset: f64, constant: bool) -> Value {
    let (value_key, fixed_key, material_key, _) = field.keys();
    let mut nodes = Vec::new();
    for j in 0..=divisions {
        for i in 0..=divisions {
            let x = 1.5 * i as f64 + 0.25 * j as f64;
            let y = 0.5 * i as f64 + 1.25 * j as f64;
            nodes.push(json!({
                "id": format!("n{i}-{j}"), "x": x, "y": y,
                fixed_key: i == 0 || j == 0 || i == divisions || j == divisions,
                value_key: offset + if constant { 0.0 } else { 2.0 * x - y }
            }));
        }
    }
    let mut elements = Vec::new();
    for j in 0..divisions {
        for i in 0..divisions {
            let a = j * (divisions + 1) + i;
            let b = a + 1;
            let c = b + divisions + 1;
            let d = a + divisions + 1;
            let mut first = json!({"id": format!("e{i}-{j}"), "node_i": a, "node_j": b,
                "node_k": c, "thickness": 0.25, material_key: if matches!(field, Field::Magnetic) { 0.4 } else { 2.5 }});
            if quad {
                first["node_l"] = json!(d);
            }
            elements.push(first.clone());
            if !quad {
                first["id"] = json!(format!("e{i}-{j}-second"));
                first["node_j"] = json!(c);
                first["node_k"] = json!(d);
                elements.push(first);
            }
        }
    }
    json!({"nodes": nodes, "elements": elements})
}

fn solve(field: Field, quad: bool, request: Value) -> Value {
    macro_rules! run {
        ($solver:path) => {
            serde_json::to_value($solver(&serde_json::from_value(request).unwrap()).unwrap())
                .unwrap()
        };
    }
    match (field, quad) {
        (Field::Heat, false) => run!(solve_heat_plane_triangle_2d),
        (Field::Heat, true) => run!(solve_heat_plane_quad_2d),
        (Field::Electric, false) => run!(solve_electrostatic_plane_triangle_2d),
        (Field::Electric, true) => run!(solve_electrostatic_plane_quad_2d),
        (Field::Magnetic, false) => run!(solve_magnetostatic_plane_triangle_2d),
        (Field::Magnetic, true) => run!(solve_magnetostatic_plane_quad_2d),
    }
}

fn check_offset(field: Field, quad: bool) {
    for divisions in [1, 3, 9] {
        for constant in [true, false] {
            let offset = 2.0_f64.powi(40);
            let baseline = solve(field, quad, mesh(field, quad, divisions, 0.0, constant));
            let shifted = solve(field, quad, mesh(field, quad, divisions, offset, constant));
            let (value_key, _, _, gradient_key) = field.keys();
            for (actual, expected) in shifted["elements"]
                .as_array()
                .unwrap()
                .iter()
                .zip(baseline["elements"].as_array().unwrap())
            {
                for suffix in ["_x", "_y"] {
                    let key = format!("{gradient_key}{suffix}");
                    let actual = actual[&key].as_f64().unwrap();
                    let expected = expected[&key].as_f64().unwrap();
                    let analytic = if constant {
                        0.0
                    } else if suffix == "_x" {
                        2.0
                    } else {
                        -1.0
                    };
                    assert!(
                        (actual - analytic).abs() < 1.0e-8,
                        "linear patch gradient was not recovered"
                    );
                    assert!(
                        (actual - expected).abs() < 1.0e-8,
                        "{field:?} quad={quad} n={divisions} constant={constant}: {key}={actual}, baseline={expected}"
                    );
                }
            }
            for (actual, expected) in shifted["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .zip(baseline["nodes"].as_array().unwrap())
            {
                let actual = actual[value_key].as_f64().unwrap();
                let expected = expected[value_key].as_f64().unwrap() + offset;
                assert!(
                    (actual - expected).abs() <= 0.0005,
                    "absolute node value was not restored"
                );
            }
        }
    }
}

fn compare_shifted(field: Field, actual: &Value, expected: &Value, offset: f64) {
    let (value_key, fixed_key, _, _) = field.keys();
    for (actual, expected) in actual["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["nodes"].as_array().unwrap())
    {
        assert!(
            (actual[value_key].as_f64().unwrap() - expected[value_key].as_f64().unwrap() - offset)
                .abs()
                <= 0.0005
        );
    }
    for (input, output) in actual["input"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .zip(actual["nodes"].as_array().unwrap())
    {
        if input[fixed_key].as_bool().unwrap() {
            assert_eq!(
                input[value_key], output[value_key],
                "prescribed field changed"
            );
        }
    }
    let average_key = format!("average_{value_key}");
    for (actual, expected) in actual["elements"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["elements"].as_array().unwrap())
    {
        for (key, expected) in expected.as_object().unwrap() {
            if key == &average_key {
                assert!(
                    (actual[key].as_f64().unwrap() - expected.as_f64().unwrap() - offset).abs()
                        <= 0.0005,
                    "average absolute field was not restored"
                );
            } else if let Some(expected) = expected.as_f64() {
                assert_close(actual[key].as_f64().unwrap(), expected, key);
            } else {
                assert_eq!(&actual[key], expected);
            }
        }
    }
    if !matches!(field, Field::Heat) {
        assert_close(
            actual["total_stored_energy"].as_f64().unwrap(),
            expected["total_stored_energy"].as_f64().unwrap(),
            "total energy",
        );
    }
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= 1.0e-12 + 1.0e-8 * expected.abs(),
        "{label}: actual={actual}, expected={expected}"
    );
}

fn reverse_elements(request: &mut Value, quad: bool) {
    for element in request["elements"].as_array_mut().unwrap() {
        let other = if quad { "node_l" } else { "node_k" };
        let value = element[other].clone();
        element[other] = element["node_j"].clone();
        element["node_j"] = value;
    }
}

fn check_source_and_orientation(field: Field) {
    let source_key = match field {
        Field::Heat => "heat_load",
        Field::Electric => "charge_density",
        Field::Magnetic => "current_density",
    };
    let (value_key, fixed_key, material_key, _) = field.keys();
    for quad in [false, true] {
        // 34 divisions give 1089 free DOFs, exercising PCG as well as dense LU.
        for divisions in [3, 34] {
            let mut request = mesh(field, quad, divisions, 4.25, true);
            for (index, node) in request["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                if !node[fixed_key].as_bool().unwrap() {
                    node[source_key] = json!(if index % 3 == 0 { -0.001 } else { 0.002 });
                }
            }
            for (index, element) in request["elements"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                if index % 3 == 0 {
                    element[material_key] = json!(element[material_key].as_f64().unwrap() * 2.0);
                }
            }
            let baseline = solve(field, quad, request.clone());
            for offset in [-2.0_f64.powi(40), 2.0_f64.powi(40)] {
                let mut shifted = request.clone();
                for node in shifted["nodes"].as_array_mut().unwrap() {
                    node[value_key] = json!(node[value_key].as_f64().unwrap() + offset);
                }
                let result = solve(field, quad, shifted.clone());
                compare_shifted(field, &result, &baseline, offset);
                reverse_elements(&mut shifted, quad);
                let reversed = solve(field, quad, shifted);
                // Connectivity is deliberately reversed; retain the physical outputs.
                let mut expected = result.clone();
                reverse_elements(&mut expected, quad);
                compare_shifted(field, &reversed, &expected, 0.0);
            }
        }
    }
}

#[test]
fn heat_sources_and_heterogeneous_materials_are_reference_and_orientation_invariant() {
    check_source_and_orientation(Field::Heat);
}

#[test]
fn electrostatic_sources_and_heterogeneous_materials_are_reference_and_orientation_invariant() {
    check_source_and_orientation(Field::Electric);
}

#[test]
fn magnetostatic_sources_and_heterogeneous_materials_are_reference_and_orientation_invariant() {
    check_source_and_orientation(Field::Magnetic);
}

#[test]
fn heat_triangles_preserve_constant_and_linear_fields_under_reference_shift() {
    check_offset(Field::Heat, false);
}
#[test]
fn heat_quads_preserve_constant_and_linear_fields_under_reference_shift() {
    check_offset(Field::Heat, true);
}
#[test]
fn electrostatic_triangles_preserve_constant_and_linear_fields_under_reference_shift() {
    check_offset(Field::Electric, false);
}
#[test]
fn electrostatic_quads_preserve_constant_and_linear_fields_under_reference_shift() {
    check_offset(Field::Electric, true);
}
#[test]
fn magnetostatic_triangles_preserve_constant_and_linear_fields_under_reference_shift() {
    check_offset(Field::Magnetic, false);
}
#[test]
fn magnetostatic_quads_preserve_constant_and_linear_fields_under_reference_shift() {
    check_offset(Field::Magnetic, true);
}

fn check_quad_energy(field: Field) {
    let mut totals = Vec::new();
    for quad in [false, true] {
        let mut request = mesh(field, quad, 1, 0.0, false);
        for (index, (x, y, value)) in [
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 1.0),
            (0.0, 1.0, 1.0),
            (1.0, 1.0, 0.0),
        ]
        .into_iter()
        .enumerate()
        {
            request["nodes"][index]["x"] = json!(x);
            request["nodes"][index]["y"] = json!(y);
            request["nodes"][index][field.keys().0] = json!(value);
        }
        let result = solve(field, quad, request);
        // Each half has |grad u|^2 = 2. Opposite vectors must not cancel energy.
        let energy = result["total_stored_energy"].as_f64().unwrap();
        assert!(
            (energy - 0.625).abs() < 1.0e-12,
            "{field:?} quad={quad}: energy={energy}"
        );
        totals.push(energy);
    }
    assert!((totals[0] - totals[1]).abs() < 1.0e-12);
}

#[test]
fn electrostatic_quad_energy_integrates_both_halves_before_averaging() {
    check_quad_energy(Field::Electric);
}
#[test]
fn magnetostatic_quad_energy_integrates_both_halves_before_averaging() {
    check_quad_energy(Field::Magnetic);
}

#[test]
fn unequal_area_quad_energy_matches_analytic_subtriangle_integral() {
    for field in [Field::Electric, Field::Magnetic] {
        for quad in [false, true] {
            let mut request = mesh(field, quad, 1, 0.0, false);
            for (node, (x, y, value)) in request["nodes"].as_array_mut().unwrap().iter_mut().zip([
                (0.0, 0.0, 0.0),
                (2.0, 0.0, 1.0),
                (0.0, 1.0, 2.0),
                (1.0, 1.0, 0.0),
            ]) {
                node["x"] = json!(x);
                node["y"] = json!(y);
                node[field.keys().0] = json!(value);
            }
            // Areas 1 and 0.5; gradients (0.5, -0.5) and (-2, 2).
            let result = solve(field, quad, request.clone());
            assert_close(
                result["total_stored_energy"].as_f64().unwrap(),
                1.40625,
                "unequal-area energy",
            );
            reverse_elements(&mut request, quad);
            let reversed = solve(field, quad, request);
            assert_close(
                reversed["total_stored_energy"].as_f64().unwrap(),
                1.40625,
                "reversed energy",
            );
        }
    }
}
