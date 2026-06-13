use super::helpers::{exported_content, run_solver_summary_json_graph};

#[test]
fn runs_electrostatic_plane_triangle_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.electrostatic-plane-triangle-summary-json",
        "Electrostatic plane triangle summary json",
        "electrostatic_triangle_model",
        "study_model/electrostatic_plane_triangle_2d",
        "solve_electrostatic_triangle",
        "solve.electrostatic_plane_triangle_2d",
        "result/electrostatic_plane_triangle_2d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                { "id": "n1", "x": 1.0, "y": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                { "id": "n2", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
            ],
            "elements": [
                { "id": "et0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.01, "permittivity": 2.5 }
            ]
        }),
        &["max_potential", "max_electric_field", "max_flux_density"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_potential"));
    assert!(content.contains("max_electric_field"));
    assert!(content.contains("max_flux_density"));
}
