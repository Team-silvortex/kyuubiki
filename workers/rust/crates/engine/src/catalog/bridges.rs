use crate::catalog::descriptors::{
    built_in_bridge_descriptor, built_in_explicit_bridge_descriptor,
};
use crate::catalog::workflow_transforms::workflow_transform_descriptors;
use kyuubiki_protocol::OperatorDescriptor;

pub(crate) fn bridge_descriptors() -> Vec<OperatorDescriptor> {
    let mut descriptors = vec![
        built_in_bridge_descriptor(
            "bridge.temperature_field_to_thermo_quad_2d",
            "thermo_mechanical",
            "thermal_plane_quad_2d",
            "Bridge a heat quad temperature field into a thermal quad structural model.",
            &["workflow_bridge", "temperature_field", "quad", "2d"],
        ),
        built_in_explicit_bridge_descriptor(
            "bridge.temperature_field_to_thermo_triangle_2d",
            "thermo_mechanical",
            "thermal_plane_triangle_2d",
            "Bridge a heat triangle temperature field into a thermal triangle structural model.",
            &["workflow_bridge", "temperature_field", "triangle", "2d"],
            "result/heat_plane_triangle_2d",
            "heat_plane_triangle_2d_result",
            "study_model/thermal_plane_triangle_2d",
            "thermal_plane_triangle_2d_model",
        ),
        built_in_bridge_descriptor(
            "bridge.electrostatic_field_to_heat_quad_2d",
            "electromagnetic",
            "electrostatic_to_heat_quad_2d",
            "Bridge electrostatic quad fields or stored-energy density into nodal heat loads for a downstream heat quad model.",
            &[
                "workflow_bridge",
                "electrostatic",
                "stored_energy",
                "heat",
                "quad",
                "2d",
            ],
        ),
        built_in_explicit_bridge_descriptor(
            "bridge.electrostatic_field_to_heat_triangle_2d",
            "electromagnetic",
            "electrostatic_to_heat_triangle_2d",
            "Bridge electrostatic triangle fields or stored-energy density into nodal heat loads for a downstream heat triangle model.",
            &[
                "workflow_bridge",
                "electrostatic",
                "stored_energy",
                "heat",
                "triangle",
                "2d",
            ],
            "result/electrostatic_plane_triangle_2d",
            "electrostatic_plane_triangle_2d_result",
            "study_model/heat_plane_triangle_2d",
            "heat_plane_triangle_2d_model",
        ),
        built_in_explicit_bridge_descriptor(
            "bridge.magnetostatic_field_to_heat_quad_2d",
            "electromagnetic",
            "magnetostatic_to_heat_quad_2d",
            "Bridge magnetostatic quad fields or stored magnetic energy density into heat loads for downstream thermal solves.",
            &[
                "verified",
                "workflow_bridge",
                "magnetostatic",
                "stored_energy",
                "heat",
                "quad",
                "2d",
            ],
            "result/magnetostatic_plane_quad_2d",
            "magnetostatic_plane_quad_2d_result",
            "study_model/heat_plane_quad_2d",
            "heat_plane_quad_2d_model",
        ),
    ];

    descriptors.extend(workflow_transform_descriptors());
    descriptors
}
