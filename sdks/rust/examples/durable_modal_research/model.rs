use serde_json::{Value, json};

/// Synthetic two-material cantilever, not a calibrated material database.
pub fn model(reversed: bool) -> Value {
    let mut nodes: Vec<_> = [0.0, 1.0, 3.0]
        .into_iter()
        .enumerate()
        .map(|(i, x)| {
            json!({"id":format!("node-{i}"), "x":x, "y":0.0,
            "fix_x":true, "fix_y":i==0, "fix_rz":i==0,
            "load_x":0.0, "load_y":0.0, "moment_z":0.0})
        })
        .collect();
    let mut elements = vec![
        json!({"id":"material-a", "node_i":0, "node_j":1, "area":2.0,
            "youngs_modulus":4.0, "density":3.0, "moment_of_inertia":0.3, "section_modulus":1.0}),
        json!({"id":"material-b", "node_i":1, "node_j":2, "area":1.5,
            "youngs_modulus":9.0, "density":5.0, "moment_of_inertia":0.7, "section_modulus":1.0}),
    ];
    if reversed {
        nodes.reverse();
        elements.reverse();
        for element in &mut elements {
            for key in ["node_i", "node_j"] {
                element[key] = json!(2 - element[key].as_u64().unwrap());
            }
        }
    }
    json!({"nodes":nodes, "elements":elements, "mode_count":4})
}
