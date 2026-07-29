use crate::linear_algebra::{SparseMatrix, add_at};
use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, strain_energy_density,
};
use crate::plane_2d_quad::{
    PlaneQuadComputed, multiply_matrix_vector_3x8, precompute_plane_quad_element_from_coordinates,
};
use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, ThermalPlaneQuadElementInput, ThermalPlaneQuadElementResult,
};

#[derive(Debug, Clone)]
pub(crate) struct ThermalPlaneQuadComputed {
    plane: PlaneQuadComputed,
    equivalent_load: [f64; 8],
    temperature_deltas: [f64; 4],
    average_temperature_delta: f64,
}

#[derive(Debug, Clone)]
struct ThermalPlaneQuadState {
    total_strain: [f64; 3],
    mechanical_strain: [f64; 3],
    thermal_strain: f64,
    stress: [f64; 3],
    principal_stress_1: f64,
    principal_stress_2: f64,
    max_in_plane_shear: f64,
    von_mises: f64,
    strain_energy_density: f64,
}

pub(crate) fn precompute_thermal_plane_quad_element(
    request: &SolveThermalPlaneQuad2dRequest,
    element: &ThermalPlaneQuadElementInput,
) -> Result<ThermalPlaneQuadComputed, String> {
    let indices = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ];
    let coordinates = std::array::from_fn(|index| {
        [
            request.nodes[indices[index]].x,
            request.nodes[indices[index]].y,
        ]
    });
    let temperature_deltas =
        std::array::from_fn(|index| request.nodes[indices[index]].temperature_delta);
    let plane = precompute_plane_quad_element_from_coordinates(
        coordinates,
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )
    .map_err(|error| error.replacen("plane quad", "thermal plane quad", 1))?;

    let mut equivalent_load = [0.0; 8];
    let mut integrated_temperature_delta = 0.0;
    for point in &plane.gauss_points {
        let temperature_delta = dot_4(&point.shape_functions, &temperature_deltas);
        let thermal_strain = element.thermal_expansion * temperature_delta;
        let thermal_stress =
            multiply_matrix_vector_3x3(&plane.d_matrix, &[thermal_strain, thermal_strain, 0.0]);
        let scale = element.thickness * point.det_jacobian;
        for (row, force) in equivalent_load.iter_mut().enumerate() {
            *force += (0..3)
                .map(|component| point.b_matrix[component][row] * thermal_stress[component])
                .sum::<f64>()
                * scale;
        }
        integrated_temperature_delta += temperature_delta * point.det_jacobian;
    }

    Ok(ThermalPlaneQuadComputed {
        average_temperature_delta: integrated_temperature_delta / plane.area,
        plane,
        equivalent_load,
        temperature_deltas,
    })
}

pub(crate) fn assemble_thermal_plane_quad(
    element: &ThermalPlaneQuadElementInput,
    computed: &ThermalPlaneQuadComputed,
    global_stiffness: &mut SparseMatrix,
    force_vector: &mut [f64],
) {
    let map = quad_dof_map(element);
    for row in 0..8 {
        force_vector[map[row]] += computed.equivalent_load[row];
        for column in 0..8 {
            add_at(
                global_stiffness,
                map[row],
                map[column],
                computed.plane.stiffness[row][column],
            );
        }
    }
}

pub(crate) fn build_thermal_plane_quad_element(
    index: usize,
    element: &ThermalPlaneQuadElementInput,
    computed: &ThermalPlaneQuadComputed,
    displacements: &[f64],
) -> ThermalPlaneQuadElementResult {
    let map = quad_dof_map(element);
    let element_displacements = std::array::from_fn(|local| displacements[map[local]]);
    let state =
        thermal_plane_quad_state(computed, &element_displacements, element.thermal_expansion);

    ThermalPlaneQuadElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        node_l: element.node_l,
        area: computed.plane.area,
        average_temperature_delta: computed.average_temperature_delta,
        thermal_strain: state.thermal_strain,
        mechanical_strain_x: state.mechanical_strain[0],
        mechanical_strain_y: state.mechanical_strain[1],
        total_strain_x: state.total_strain[0],
        total_strain_y: state.total_strain[1],
        gamma_xy: state.total_strain[2],
        stress_x: state.stress[0],
        stress_y: state.stress[1],
        tau_xy: state.stress[2],
        principal_stress_1: state.principal_stress_1,
        principal_stress_2: state.principal_stress_2,
        max_in_plane_shear: state.max_in_plane_shear,
        von_mises: state.von_mises,
        strain_energy_density: state.strain_energy_density,
    }
}

fn thermal_plane_quad_state(
    computed: &ThermalPlaneQuadComputed,
    element_displacements: &[f64; 8],
    thermal_expansion: f64,
) -> ThermalPlaneQuadState {
    let mut total_strain = [0.0; 3];
    let mut mechanical_strain = [0.0; 3];
    let mut stress = [0.0; 3];
    let mut integrated_thermal_strain = 0.0;
    let mut integrated_energy = 0.0;

    for point in &computed.plane.gauss_points {
        let point_total_strain = multiply_matrix_vector_3x8(&point.b_matrix, element_displacements);
        let temperature_delta = dot_4(&point.shape_functions, &computed.temperature_deltas);
        let point_thermal_strain = thermal_expansion * temperature_delta;
        let point_mechanical_strain = [
            point_total_strain[0] - point_thermal_strain,
            point_total_strain[1] - point_thermal_strain,
            point_total_strain[2],
        ];
        let point_stress =
            multiply_matrix_vector_3x3(&computed.plane.d_matrix, &point_mechanical_strain);
        for component in 0..3 {
            total_strain[component] += point_total_strain[component] * point.det_jacobian;
            mechanical_strain[component] += point_mechanical_strain[component] * point.det_jacobian;
            stress[component] += point_stress[component] * point.det_jacobian;
        }
        integrated_thermal_strain += point_thermal_strain * point.det_jacobian;
        integrated_energy +=
            strain_energy_density(&point_stress, &point_mechanical_strain) * point.det_jacobian;
    }

    for component in 0..3 {
        total_strain[component] /= computed.plane.area;
        mechanical_strain[component] /= computed.plane.area;
        stress[component] /= computed.plane.area;
    }
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    ThermalPlaneQuadState {
        total_strain,
        mechanical_strain,
        thermal_strain: integrated_thermal_strain / computed.plane.area,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
        strain_energy_density: integrated_energy / computed.plane.area,
    }
}

fn quad_dof_map(element: &ThermalPlaneQuadElementInput) -> [usize; 8] {
    [
        element.node_i * 2,
        element.node_i * 2 + 1,
        element.node_j * 2,
        element.node_j * 2 + 1,
        element.node_k * 2,
        element.node_k * 2 + 1,
        element.node_l * 2,
        element.node_l * 2 + 1,
    ]
}

fn dot_4(left: &[f64; 4], right: &[f64; 4]) -> f64 {
    (0..4).map(|index| left[index] * right[index]).sum()
}
