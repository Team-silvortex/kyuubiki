use crate::{HeadlessWorkflowDraft, HeadlessWorkflowStep};
use serde_json::json;

pub(super) fn build_template_workflow(
    template_id: &str,
    workflow_id: &str,
) -> HeadlessWorkflowDraft {
    let steps = match template_id {
        "solve_wait_result" => vec![HeadlessWorkflowStep::new(
            "solve_and_wait_from_model_version",
            json!({ "model_version_id": "ver_123", "endpoints": ["http://127.0.0.1:7001"], "timeout_ms": 60000 }),
        )],
        "workflow_submit_monitor" => vec![
            HeadlessWorkflowStep::new(
                "workflow_submit_catalog",
                json!({ "workflow_id": "wf_demo", "input_artifacts": {} }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_mesh_pipeline" => vec![
            HeadlessWorkflowStep::new(
                "direct_mesh_solve",
                json!({ "study_kind": "truss_3d", "input": { "nodes": [], "elements": [] }, "endpoints": ["http://127.0.0.1:7001"] }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "material_heat_spreader_screening" => crate::build_heat_spreader_screening_steps(),
        "material_dielectric_screening" => crate::build_dielectric_screening_steps(),
        "material_thermo_shield_screening" => crate::build_thermo_shield_screening_steps(),
        "material_structural_panel_screening" => crate::build_structural_panel_screening_steps(),
        "material_composite_thermo_electric_panel" => crate::build_composite_panel_steps(),
        "material_study_envelope_ranking" => vec![
            HeadlessWorkflowStep::new(
                "workflow_submit_graph",
                json!({
                    "graph": crate::material_envelope_workflow::material_study_envelope_graph_payload(),
                    "input_artifacts": crate::material_envelope_workflow::material_study_envelope_input_artifacts()
                }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "material_study_envelope_catalog" => vec![
            HeadlessWorkflowStep::new(
                "workflow_submit_catalog",
                json!({
                    "workflow_id": "workflow.material-study-envelope-ranking-json",
                    "input_artifacts": crate::material_envelope_workflow::material_study_envelope_input_artifacts()
                }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_plane_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_plane_quad_2d",
                json!({ "model": { "nodes": [{ "id": "q0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "q1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "q2", "x": 1.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 }, { "id": "q3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": false, "load_x": 0.0, "load_y": 0.0 }], "elements": [{ "id": "pq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_plane_triangle" => vec![
            HeadlessWorkflowStep::new(
                "solve_plane_triangle_2d",
                json!({ "model": { "nodes": [{ "id": "p0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "p1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "p2", "x": 0.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 }], "elements": [{ "id": "pt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_bar_1d" => vec![
            HeadlessWorkflowStep::new(
                "solve_bar_1d",
                json!({ "model": { "length": 1.0, "area": 0.01, "youngs_modulus_gpa": 210.0, "elements": 2, "tip_force": 1200.0 } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_truss_3d" => vec![
            HeadlessWorkflowStep::new(
                "solve_truss_3d",
                json!({ "model": { "nodes": [{ "id": "b0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 }, { "id": "b1", "x": 1.2, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 }, { "id": "b2", "x": 0.0, "y": 1.2, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 }, { "id": "top", "x": 0.35, "y": 0.35, "z": 1.0, "fix_x": false, "fix_y": false, "fix_z": false, "load_x": 0.0, "load_y": 0.0, "load_z": -1600.0 }], "elements": [{ "id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e2", "node_i": 2, "node_j": 0, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e3", "node_i": 0, "node_j": 3, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e4", "node_i": 1, "node_j": 3, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e5", "node_i": 2, "node_j": 3, "area": 0.01, "youngs_modulus": 70000000000.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_frame_2d" => vec![
            HeadlessWorkflowStep::new(
                "solve_frame_2d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0 }, { "id": "n1", "x": 2.0, "y": 0.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": -1000.0, "moment_z": 0.0 }], "elements": [{ "id": "f0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.00016 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_beam_1d" => vec![
            HeadlessWorkflowStep::new(
                "solve_beam_1d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "fix_y": true, "fix_rz": true, "load_y": 0.0, "moment_z": 0.0 }, { "id": "n1", "x": 2.0, "fix_y": false, "fix_rz": false, "load_y": -1000.0, "moment_z": 0.0 }], "elements": [{ "id": "b0", "node_i": 0, "node_j": 1, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.00016, "distributed_load_y": 0.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_truss_2d" => vec![
            HeadlessWorkflowStep::new(
                "solve_truss_2d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "n2", "x": 0.5, "y": 0.75, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 }], "elements": [{ "id": "e0", "node_i": 0, "node_j": 2, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 70000000000.0 }, { "id": "e2", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 70000000000.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_spring_2d" => vec![
            HeadlessWorkflowStep::new(
                "solve_spring_2d",
                json!({ "model": { "nodes": [{ "id": "s0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "s1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }, { "id": "s2", "x": 1.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 1200.0, "load_y": -600.0 }, { "id": "s3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": false, "load_x": 0.0, "load_y": 0.0 }], "elements": [{ "id": "sp0", "node_i": 0, "node_j": 1, "stiffness": 25000.0 }, { "id": "sp1", "node_i": 1, "node_j": 2, "stiffness": 18000.0 }, { "id": "sp2", "node_i": 2, "node_j": 3, "stiffness": 22000.0 }, { "id": "sp3", "node_i": 3, "node_j": 0, "stiffness": 18000.0 }, { "id": "sp4", "node_i": 0, "node_j": 2, "stiffness": 12000.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_torsion_1d" => vec![
            HeadlessWorkflowStep::new(
                "solve_torsion_1d",
                json!({ "model": { "nodes": [{ "id": "t0", "x": 0.0, "fix_rz": true, "torque_z": 0.0 }, { "id": "t1", "x": 1.0, "fix_rz": false, "torque_z": 500.0 }], "elements": [{ "id": "te0", "node_i": 0, "node_j": 1, "shear_modulus": 80000000000.0, "polar_moment": 0.000005, "section_modulus": 0.00016 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_spring_3d" => vec![
            HeadlessWorkflowStep::new(
                "solve_spring_3d",
                json!({ "model": { "nodes": [{ "id": "s0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 }, { "id": "s1", "x": 1.2, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 }, { "id": "s2", "x": 0.0, "y": 1.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 }, { "id": "top", "x": 0.45, "y": 0.35, "z": 1.1, "fix_x": false, "fix_y": false, "fix_z": false, "load_x": 250.0, "load_y": 0.0, "load_z": -1100.0 }], "elements": [{ "id": "k0", "node_i": 0, "node_j": 3, "stiffness": 18000.0 }, { "id": "k1", "node_i": 1, "node_j": 3, "stiffness": 22000.0 }, { "id": "k2", "node_i": 2, "node_j": 3, "stiffness": 16000.0 }, { "id": "k3", "node_i": 0, "node_j": 1, "stiffness": 9000.0 }, { "id": "k4", "node_i": 1, "node_j": 2, "stiffness": 7000.0 }, { "id": "k5", "node_i": 2, "node_j": 0, "stiffness": 8000.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_frame_3d" => vec![
            HeadlessWorkflowStep::new(
                "solve_frame_3d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "fix_rx": true, "fix_ry": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0 }, { "id": "n1", "x": 2.0, "y": 0.0, "z": 0.0, "fix_x": false, "fix_y": false, "fix_z": false, "fix_rx": false, "fix_ry": false, "fix_rz": false, "load_x": 0.0, "load_y": -1000.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0 }], "elements": [{ "id": "f0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210000000000.0, "shear_modulus": 80000000000.0, "torsion_constant": 0.000005, "moment_of_inertia_y": 0.000008, "moment_of_inertia_z": 0.000008, "section_modulus_y": 0.00016, "section_modulus_z": 0.00016 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_spring_1d" => vec![
            HeadlessWorkflowStep::new(
                "solve_spring_1d",
                json!({ "model": { "nodes": [{ "id": "s0", "x": 0.0, "fix_x": true, "load_x": 0.0 }, { "id": "s1", "x": 1.2, "fix_x": false, "load_x": 0.0 }, { "id": "s2", "x": 2.4, "fix_x": false, "load_x": 1200.0 }], "elements": [{ "id": "k0", "node_i": 0, "node_j": 1, "stiffness": 35000.0 }, { "id": "k1", "node_i": 1, "node_j": 2, "stiffness": 20000.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_heat_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_heat_plane_quad_2d",
                json!({ "model": { "nodes": [{ "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 }, { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 }, { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }, { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }], "elements": [{ "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_heat_triangle" => vec![
            HeadlessWorkflowStep::new(
                "solve_heat_plane_triangle_2d",
                json!({ "model": { "nodes": [{ "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 }, { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }, { "id": "h2", "x": 0.0, "y": 1.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 }], "elements": [{ "id": "ht0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "conductivity": 45.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_heat_bar_1d" => vec![
            HeadlessWorkflowStep::new(
                "solve_heat_bar_1d",
                json!({ "model": { "nodes": [{ "id": "h0", "x": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 }, { "id": "h1", "x": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }], "elements": [{ "id": "he0", "node_i": 0, "node_j": 1, "area": 0.02, "conductivity": 45.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_plane_quad_2d",
                json!({ "model": { "nodes": [{ "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }, { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }, { "id": "t2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }, { "id": "t3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }], "elements": [{ "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 210000000000.0, "poisson_ratio": 0.3, "thermal_expansion": 0.000011 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_triangle" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_plane_triangle_2d",
                json!({ "model": { "nodes": [{ "id": "tp0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 }, { "id": "tp1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 }, { "id": "tp2", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 }], "elements": [{ "id": "tpt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_truss_2d" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_truss_2d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 }, { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 }, { "id": "n2", "x": 0.5, "y": 0.8, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 }], "elements": [{ "id": "tt0", "node_i": 0, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }, { "id": "tt1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_frame_2d" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_frame_2d",
                json!({ "model": { "nodes": [{ "id": "tf0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 0.0 }, { "id": "tf1", "x": 0.0, "y": 3.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 35.0 }, { "id": "tf2", "x": 4.0, "y": 3.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 35.0 }, { "id": "tf3", "x": 4.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 0.0 }], "elements": [{ "id": "tfe0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00012, "section_modulus": 0.0011, "thermal_expansion": 0.000012, "section_depth": 0.3 }, { "id": "tfe1", "node_i": 1, "node_j": 2, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00012, "section_modulus": 0.0011, "thermal_expansion": 0.000012, "section_depth": 0.3 }, { "id": "tfe2", "node_i": 2, "node_j": 3, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00012, "section_modulus": 0.0011, "thermal_expansion": 0.000012, "section_depth": 0.3 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_beam_1d" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_beam_1d",
                json!({ "model": { "nodes": [{ "id": "tb0", "x": 0.0, "fix_y": true, "fix_rz": true, "load_y": 0.0, "moment_z": 0.0 }, { "id": "tb1", "x": 2.4, "fix_y": false, "fix_rz": false, "load_y": 0.0, "moment_z": 0.0 }], "elements": [{ "id": "tm0", "node_i": 0, "node_j": 1, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00012, "section_modulus": 0.0011, "thermal_expansion": 0.000012, "section_depth": 0.3, "distributed_load_y": 0.0, "temperature_gradient_y": 45.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_truss_3d" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_truss_3d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "temperature_delta": 40.0 }, { "id": "n1", "x": 1.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "temperature_delta": 40.0 }, { "id": "n2", "x": 0.0, "y": 1.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "temperature_delta": 40.0 }], "elements": [{ "id": "tt3-0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }, { "id": "tt3-1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }, { "id": "tt3-2", "node_i": 2, "node_j": 0, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_thermal_frame_3d" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_frame_3d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "fix_rx": true, "fix_ry": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0, "temperature_delta": 35.0 }, { "id": "n1", "x": 2.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "fix_rx": true, "fix_ry": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0, "temperature_delta": 35.0 }], "elements": [{ "id": "tf3-0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210000000000.0, "shear_modulus": 80000000000.0, "torsion_constant": 0.000005, "moment_of_inertia_y": 0.000008, "moment_of_inertia_z": 0.000006, "section_modulus_y": 0.00016, "section_modulus_z": 0.00012, "thermal_expansion": 0.000012, "section_depth_y": 0.2, "section_depth_z": 0.15, "temperature_gradient_y": 30.0, "temperature_gradient_z": 20.0 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_electrostatic_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_electrostatic_plane_quad_2d",
                json!({ "model": { "nodes": [{ "id": "e0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 }, { "id": "e1", "x": 1.0, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 }, { "id": "e2", "x": 1.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }, { "id": "e3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }], "elements": [{ "id": "eq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.05, "permittivity": 2.5 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_electrostatic_triangle" => vec![
            HeadlessWorkflowStep::new(
                "solve_electrostatic_plane_triangle_2d",
                json!({ "model": { "nodes": [{ "id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 }, { "id": "n1", "x": 1.0, "y": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }, { "id": "n2", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }], "elements": [{ "id": "et0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.01, "permittivity": 2.5 }] } }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "browser_capture_review" => vec![
            HeadlessWorkflowStep::new(
                "open_page",
                json!({ "url": "https://example.com", "waitUntil": "domcontentloaded" }),
            ),
            HeadlessWorkflowStep::new("wait", json!({ "selector": "body", "timeout": 1500 })),
            HeadlessWorkflowStep::new(
                "snapshot",
                json!({ "file": "browser-review.png", "fullPage": true }),
            ),
        ],
        "browser_submit_then_poll" => vec![
            HeadlessWorkflowStep::new(
                "open_page",
                json!({ "url": "https://example.com/jobs", "waitUntil": "domcontentloaded" }),
            ),
            HeadlessWorkflowStep::new("click", json!({ "selector": "[data-run-job]" })),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "job_123", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new("result_fetch", json!({ "job_id": "job_123" })),
        ],
        _ => vec![],
    };
    HeadlessWorkflowDraft {
        id: workflow_id.to_string(),
        steps,
    }
}
