use crate::catalog::descriptors::{
    built_in_bridge_descriptor, built_in_export_descriptor, built_in_extract_descriptor,
    built_in_solver_descriptor, built_in_transform_descriptor,
};
use kyuubiki_protocol::OperatorDescriptor;

pub fn built_in_operator_descriptors() -> Vec<OperatorDescriptor> {
    let mut descriptors = solver_descriptors();
    descriptors.extend(bridge_descriptors());
    descriptors.extend(reporting_descriptors());
    descriptors
}

fn solver_descriptors() -> Vec<OperatorDescriptor> {
    vec![
        built_in_solver_descriptor(
            "solve.frame_3d",
            "mechanical",
            "frame_3d",
            "Solve a 3D frame model with six-DOF nodes and verified baseline coverage.",
            &["verified", "mechanical", "frame", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_frame_3d",
            "thermo_mechanical",
            "thermal_frame_3d",
            "Solve a thermal 3D frame model with restrained expansion and temperature gradients.",
            &["verified", "thermo_mechanical", "frame", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.electrostatic_bar_1d",
            "electromagnetic",
            "electrostatic_bar_1d",
            "Solve a 1D electrostatic bar model and expose potential, field, and flux results.",
            &["verified", "electromagnetic", "electrostatic", "bar", "1d"],
        ),
        built_in_solver_descriptor(
            "solve.electrostatic_plane_triangle_2d",
            "electromagnetic",
            "electrostatic_plane_triangle_2d",
            "Solve a 2D electrostatic triangle model and expose potential, field, and flux results.",
            &[
                "verified",
                "electromagnetic",
                "electrostatic",
                "plane",
                "triangle",
                "2d",
            ],
        ),
        built_in_solver_descriptor(
            "solve.electrostatic_plane_quad_2d",
            "electromagnetic",
            "electrostatic_plane_quad_2d",
            "Solve a 2D electrostatic quad model and expose potential, field, and flux results.",
            &[
                "verified",
                "electromagnetic",
                "electrostatic",
                "plane",
                "quad",
                "2d",
            ],
        ),
        built_in_solver_descriptor(
            "solve.heat_plane_quad_2d",
            "thermal",
            "heat_plane_quad_2d",
            "Solve a 2D heat-conduction quad model and expose verified temperature/flux fields.",
            &["verified", "thermal", "heat", "plane", "2d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_plane_quad_2d",
            "thermo_mechanical",
            "thermal_plane_quad_2d",
            "Solve a thermo-mechanical 2D quad model with structural and thermal coupling outputs.",
            &["verified", "thermo_mechanical", "plane", "quad", "2d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_truss_3d",
            "thermo_mechanical",
            "thermal_truss_3d",
            "Solve a thermal 3D truss model with expansion-driven axial response.",
            &["verified", "thermo_mechanical", "truss", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.spring_1d",
            "mechanical",
            "spring_1d",
            "Solve a 1D spring chain model and expose nodal displacement and spring force results.",
            &["verified", "mechanical", "spring", "1d"],
        ),
        built_in_solver_descriptor(
            "solve.truss_2d",
            "mechanical",
            "truss_2d",
            "Solve a 2D truss model and expose nodal displacement and axial stress results.",
            &["verified", "mechanical", "truss", "2d"],
        ),
        built_in_solver_descriptor(
            "solve.truss_3d",
            "mechanical",
            "truss_3d",
            "Solve a 3D truss model and expose spatial displacement and axial stress results.",
            &["verified", "mechanical", "truss", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.frame_2d",
            "mechanical",
            "frame_2d",
            "Solve a 2D frame model and expose nodal displacement, rotation, and bending results.",
            &["verified", "mechanical", "frame", "2d"],
        ),
        built_in_solver_descriptor(
            "solve.beam_1d",
            "mechanical",
            "beam_1d",
            "Solve a 1D beam bending model and expose displacement, rotation, and moment results.",
            &["verified", "mechanical", "beam", "1d"],
        ),
        built_in_solver_descriptor(
            "solve.spring_2d",
            "mechanical",
            "spring_2d",
            "Solve a 2D spring network model and expose planar displacement and spring force results.",
            &["verified", "mechanical", "spring", "2d"],
        ),
        built_in_solver_descriptor(
            "solve.spring_3d",
            "mechanical",
            "spring_3d",
            "Solve a 3D spring network model and expose spatial displacement and spring force results.",
            &["verified", "mechanical", "spring", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_beam_1d",
            "thermo_mechanical",
            "thermal_beam_1d",
            "Solve a thermal 1D beam model with temperature-gradient-induced bending response.",
            &["verified", "thermo_mechanical", "beam", "1d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_frame_2d",
            "thermo_mechanical",
            "thermal_frame_2d",
            "Solve a thermal 2D frame model and expose coupled displacement, axial force, and moment results.",
            &["verified", "thermo_mechanical", "frame", "2d"],
        ),
    ]
}

fn bridge_descriptors() -> Vec<OperatorDescriptor> {
    vec![
        built_in_bridge_descriptor(
            "bridge.temperature_field_to_thermo_quad_2d",
            "thermo_mechanical",
            "thermal_plane_quad_2d",
            "Bridge a heat quad temperature field into a thermal quad structural model.",
            &["workflow_bridge", "temperature_field", "quad", "2d"],
        ),
        built_in_bridge_descriptor(
            "bridge.electrostatic_field_to_heat_quad_2d",
            "electromagnetic",
            "electrostatic_to_heat_quad_2d",
            "Bridge electrostatic quad field magnitudes into nodal heat loads for a downstream heat quad model.",
            &["workflow_bridge", "electrostatic", "heat", "quad", "2d"],
        ),
        built_in_transform_descriptor(
            "transform.first_available",
            "multi_domain",
            "first_available",
            "Merge two branch payloads by forwarding the first available incoming artifact.",
            &["transform", "merge", "branch", "headless_safe"],
        ),
    ]
}

fn reporting_descriptors() -> Vec<OperatorDescriptor> {
    vec![
        built_in_extract_descriptor(
            "extract.result_summary",
            "multi_domain",
            "result_summary",
            "Extract a compact summary from a solver result artifact.",
            &["extract", "summary", "headless_safe"],
        ),
        built_in_export_descriptor(
            "export.summary_json",
            "multi_domain",
            "summary_json",
            "Export a compact summary artifact as structured JSON content.",
            &["export", "json", "summary", "headless_safe"],
        ),
        built_in_export_descriptor(
            "export.summary_csv",
            "multi_domain",
            "summary_csv",
            "Export a compact summary artifact as CSV text for downstream delivery.",
            &["export", "csv", "summary", "headless_safe"],
        ),
    ]
}
