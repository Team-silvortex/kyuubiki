use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, Frame3dElementInput, Frame3dNodeInput,
    SolveFrame2dRequest, SolveFrame3dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, ThermalFrame2dElementInput, ThermalFrame2dNodeInput,
    ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
};

const FRAME_SUPPORT_INTERVAL: usize = 64;

pub(crate) fn generate_frame_2d_case(segments: usize, length: f64) -> SolveFrame2dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| {
            frame_2d_node(
                &format!("f2n{index}"),
                index as f64 * dx,
                0.02 * (index % 3) as f64,
                is_frame_support(index, segments),
                0.0,
                if index == segments { -1000.0 } else { 0.0 },
                0.0,
            )
        })
        .collect();
    let elements = (0..segments)
        .map(|index| Frame2dElementInput {
            id: format!("f2e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        })
        .collect();

    SolveFrame2dRequest { nodes, elements }
}

pub(crate) fn generate_frame_3d_case(segments: usize, length: f64) -> SolveFrame3dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| {
            frame_3d_node(
                &format!("f3n{index}"),
                index as f64 * dx,
                is_frame_support(index, segments),
                0.0,
                if index == segments { -1000.0 } else { 0.0 },
                0.0,
            )
        })
        .collect();
    let elements = (0..segments)
        .map(|index| Frame3dElementInput {
            id: format!("f3e{index}"),
            node_i: index,
            node_j: index + 1,
            local_y_axis: None,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 8.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.6e-4,
        })
        .collect();

    SolveFrame3dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_frame_2d_case(
    segments: usize,
    length: f64,
) -> SolveThermalFrame2dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| {
            thermal_frame_2d_node(
                &format!("tf2n{index}"),
                index as f64 * dx,
                is_frame_support(index, segments),
                20.0 + 30.0 * index as f64 / segments as f64,
            )
        })
        .collect();
    let elements = (0..segments)
        .map(|index| ThermalFrame2dElementInput {
            id: format!("tf2e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion: 12.0e-6,
            section_depth: 0.2,
            temperature_gradient_y: 30.0,
        })
        .collect();

    SolveThermalFrame2dRequest { nodes, elements }
}

pub(crate) fn generate_thermal_frame_3d_case(
    segments: usize,
    length: f64,
) -> SolveThermalFrame3dRequest {
    let segments = segments.max(1);
    let dx = length / segments as f64;
    let nodes = (0..=segments)
        .map(|index| {
            thermal_frame_3d_node(
                &format!("tf3n{index}"),
                index as f64 * dx,
                is_frame_support(index, segments),
                20.0 + 30.0 * index as f64 / segments as f64,
            )
        })
        .collect();
    let elements = (0..segments)
        .map(|index| ThermalFrame3dElementInput {
            id: format!("tf3e{index}"),
            node_i: index,
            node_j: index + 1,
            local_y_axis: None,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 6.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.2e-4,
            thermal_expansion: 12.0e-6,
            section_depth_y: 0.2,
            section_depth_z: 0.15,
            temperature_gradient_y: 30.0,
            temperature_gradient_z: 20.0,
        })
        .collect();

    SolveThermalFrame3dRequest {
        nodes,
        elements,
        directional_springs: Vec::new(),
        directional_rotational_springs: Vec::new(),
        directional_constraints: Vec::new(),
        directional_rotational_constraints: Vec::new(),
    }
}

fn is_frame_support(index: usize, segments: usize) -> bool {
    index == 0 || (index < segments && index.is_multiple_of(FRAME_SUPPORT_INTERVAL))
}

fn frame_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fixed: bool,
    load_x: f64,
    load_y: f64,
    moment_z: f64,
) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x,
        load_y,
        moment_z,
    }
}

fn frame_3d_node(
    id: &str,
    x: f64,
    fixed: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x,
        load_y,
        load_z,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
    }
}

fn thermal_frame_2d_node(
    id: &str,
    x: f64,
    fixed: bool,
    temperature_delta: f64,
) -> ThermalFrame2dNodeInput {
    ThermalFrame2dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

fn thermal_frame_3d_node(
    id: &str,
    x: f64,
    fixed: bool,
    temperature_delta: f64,
) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAME_SUPPORT_INTERVAL, generate_frame_2d_case, generate_thermal_frame_3d_case};

    #[test]
    fn large_frame_fixtures_bound_each_span_without_pinching_the_loaded_tip() {
        let segments = FRAME_SUPPORT_INTERVAL * 2;
        let frame = generate_frame_2d_case(segments, 10.0);
        assert!(frame.nodes[0].fix_x);
        assert!(frame.nodes[FRAME_SUPPORT_INTERVAL].fix_x);
        assert!(!frame.nodes[segments].fix_x);
        assert_eq!(frame.nodes[segments].load_y, -1000.0);

        let thermal = generate_thermal_frame_3d_case(segments, 10.0);
        assert!(thermal.nodes[FRAME_SUPPORT_INTERVAL].fix_x);
        assert!(!thermal.nodes[segments].fix_x);
    }
}
