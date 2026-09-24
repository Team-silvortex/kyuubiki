use serde_json::{Value, json};

pub fn add_independent_axial_member(stability: &mut Value) {
    let frame = &mut stability["buckling"]["frame"];
    let nodes = frame["nodes"].as_array_mut().unwrap();
    let offset = nodes.len();
    for end in 0..2 {
        nodes.push(json!({
            "id": format!("axial-{end}"), "x": 10.0 + end as f64, "y": 0.0,
            "fix_x": end == 0, "fix_y": true, "fix_rz": true,
            "load_x": if end == 1 { 1e18 } else { 0.0 },
            "load_y": 0.0, "moment_z": 0.0
        }));
    }
    frame["elements"].as_array_mut().unwrap().push(json!({
        "id": "independent-axial", "node_i": offset, "node_j": offset + 1,
        "area": 0.01, "youngs_modulus": 1e24,
        "moment_of_inertia": 0.01, "section_modulus": 0.01
    }));
    if let Some(shape) = stability["imperfection_shape"].as_array_mut() {
        shape.extend((0..6).map(|_| json!(0.0)));
    }
}
