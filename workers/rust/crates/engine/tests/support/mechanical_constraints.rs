use serde_json::{Value, json};

pub fn model(kind: &str) -> Value {
    let points = match kind {
        "truss_3d" | "thermal_truss_3d" => vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        "plane_triangle_2d" | "thermal_plane_triangle_2d" => {
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        }
        "plane_quad_2d" | "thermal_plane_quad_2d" => vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        "solid_tetra_3d" => vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        "thermal_bar_1d" => (0..=4).map(|index| [index as f64, 0.0, 0.0]).collect(),
        _ => vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
    };
    // The shared superset is decoded into each operator's concrete request type.
    let mut nodes: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(index, p)| {
            json!({
                "id":format!("n{index}"), "x":p[0], "y":p[1], "z":p[2],
                "fix_x":index != 1, "fix_y":true, "fix_z":true,
                "fix_rx":true, "fix_ry":true, "fix_rz":true,
                "load_x":1.0, "load_y":-1.0, "load_z":0.5,
                "moment_x":0.1, "moment_y":0.2, "moment_z":0.3,
                "torque_z":1.0, "temperature_delta":2.0
            })
        })
        .collect();
    if matches!(
        kind,
        "beam_1d" | "thermal_beam_1d" | "frame_2d" | "thermal_frame_2d"
    ) {
        nodes[1]["fix_y"] = json!(false);
        nodes[1]["fix_rz"] = json!(false);
    }
    if kind == "torsion_1d" {
        nodes[1]["fix_rz"] = json!(false);
    }
    let mut elements = vec![element(0, 1)];
    if matches!(kind, "truss_3d" | "thermal_truss_3d") {
        elements.push(element(0, 2));
    }
    if kind == "thermal_bar_1d" {
        elements.extend((2..=4).map(|index| element(0, index)));
    }
    json!({"nodes":nodes, "elements":elements})
}

fn element(first: usize, second: usize) -> Value {
    json!({
        "id":format!("e{first}_{second}"), "node_i":first, "node_j":second,
        "node_k":2, "node_l":3, "node_a":0, "node_b":1, "node_c":2, "node_d":3,
        "stiffness":1000.0, "area":1.0, "youngs_modulus":200000.0,
        "shear_modulus":80000.0, "moment_of_inertia":0.1, "section_modulus":0.2,
        "moment_of_inertia_y":0.1, "moment_of_inertia_z":0.1,
        "section_modulus_y":0.2, "section_modulus_z":0.2,
        "torsion_constant":0.1, "polar_moment":0.1,
        "thickness":0.1, "poisson_ratio":0.25,
        "thermal_expansion":0.0001, "section_depth":1.0,
        "temperature_gradient_y":0.5, "distributed_load_y":-2.0
    })
}

pub fn dof_count(kind: &str, input: &Value) -> usize {
    let per_node = match kind {
        "spring_1d" | "thermal_bar_1d" | "torsion_1d" => 1,
        "spring_2d"
        | "truss_2d"
        | "thermal_truss_2d"
        | "beam_1d"
        | "thermal_beam_1d"
        | "plane_triangle_2d"
        | "plane_quad_2d"
        | "thermal_plane_triangle_2d"
        | "thermal_plane_quad_2d" => 2,
        "frame_3d" => 6,
        _ => 3,
    };
    per_node * input["nodes"].as_array().unwrap().len()
}

pub fn fix_all(input: &mut Value) {
    for node in input["nodes"].as_array_mut().unwrap() {
        for (key, value) in node.as_object_mut().unwrap() {
            if key.starts_with("fix_") {
                *value = json!(true);
            }
        }
    }
}
