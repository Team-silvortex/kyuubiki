use crate::linear_algebra::{SparseMatrix, add_at};
use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, strain_energy_density,
};
use crate::plane_2d_quad::{
    PlaneQuadComputed, multiply_matrix_vector_3x8, precompute_plane_quad_element_from_coordinates,
};
use crate::thermal_plane_2d_results::{ThermalPlaneState, weighted_average};
use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, ThermalPlaneQuadElementInput, ThermalPlaneQuadElementResult,
};

#[derive(Debug, Clone)]
pub(crate) struct ThermalPlaneQuadComputed {
    plane: PlaneQuadComputed,
    equivalent_load: [f64; 8],
    gauss_temperature_deltas: [f64; 4],
    average_temperature_delta: f64,
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
    let nodal_temperature_deltas =
        std::array::from_fn(|index| request.nodes[indices[index]].temperature_delta);
    let plane = precompute_plane_quad_element_from_coordinates(
        coordinates,
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )
    .map_err(|error| {
        format!(
            "{}: {}",
            element.id,
            error.replacen("plane quad", "thermal plane quad", 1)
        )
    })?;

    let mut equivalent_load = [0.0; 8];
    let gauss_temperature_deltas = std::array::from_fn(|i| {
        weighted_average(
            nodal_temperature_deltas,
            plane.gauss_points[i].shape_functions,
        )
    });
    for (point, temperature_delta) in plane.gauss_points.iter().zip(gauss_temperature_deltas) {
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
    }
    let average_temperature_delta = weighted_average(
        gauss_temperature_deltas,
        plane
            .gauss_points
            .each_ref()
            .map(|point| point.det_jacobian / plane.area),
    );
    if !average_temperature_delta.is_finite()
        || equivalent_load.iter().any(|value| !value.is_finite())
    {
        return Err(format!(
            "thermal plane quad element {}: temperature or equivalent thermal load is not representable",
            element.id
        ));
    }

    Ok(ThermalPlaneQuadComputed {
        average_temperature_delta,
        plane,
        equivalent_load,
        gauss_temperature_deltas,
    })
}

pub(crate) fn assemble_thermal_plane_quad(
    element: &ThermalPlaneQuadElementInput,
    computed: &ThermalPlaneQuadComputed,
    global_stiffness: &mut SparseMatrix,
    force_vector: &mut [f64],
) -> Result<(), String> {
    let map = quad_dof_map(element);
    for row in 0..8 {
        force_vector[map[row]] += computed.equivalent_load[row];
        if !force_vector[map[row]].is_finite() {
            return Err(format!(
                "thermal plane quad element {}: assembled thermal force is not representable",
                element.id
            ));
        }
        for column in 0..8 {
            add_at(
                global_stiffness,
                map[row],
                map[column],
                computed.plane.stiffness[row][column],
            );
        }
    }
    Ok(())
}

pub(crate) fn build_thermal_plane_quad_element(
    index: usize,
    element: &ThermalPlaneQuadElementInput,
    computed: &ThermalPlaneQuadComputed,
    displacements: &[f64],
) -> Result<ThermalPlaneQuadElementResult, String> {
    let map = quad_dof_map(element);
    let element_displacements = std::array::from_fn(|local| displacements[map[local]]);
    let state =
        thermal_plane_quad_state(computed, &element_displacements, element.thermal_expansion);
    state.validate(&element.id)?;

    Ok(ThermalPlaneQuadElementResult {
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
    })
}

fn thermal_plane_quad_state(
    computed: &ThermalPlaneQuadComputed,
    element_displacements: &[f64; 8],
    thermal_expansion: f64,
) -> ThermalPlaneState {
    let fields: [_; 4] = std::array::from_fn(|i| {
        let point = &computed.plane.gauss_points[i];
        let point_total_strain = multiply_matrix_vector_3x8(&point.b_matrix, element_displacements);
        let temperature_delta = computed.gauss_temperature_deltas[i];
        let point_thermal_strain = thermal_expansion * temperature_delta;
        let point_mechanical_strain = [
            point_total_strain[0] - point_thermal_strain,
            point_total_strain[1] - point_thermal_strain,
            point_total_strain[2],
        ];
        let point_stress =
            multiply_matrix_vector_3x3(&computed.plane.d_matrix, &point_mechanical_strain);
        (
            point_total_strain,
            point_mechanical_strain,
            point_stress,
            point_thermal_strain,
            strain_energy_density(&point_stress, &point_mechanical_strain),
        )
    });
    let weights = computed
        .plane
        .gauss_points
        .each_ref()
        .map(|point| point.det_jacobian / computed.plane.area);
    let total_strain =
        std::array::from_fn(|component| weighted_average(fields.map(|f| f.0[component]), weights));
    let mechanical_strain =
        std::array::from_fn(|component| weighted_average(fields.map(|f| f.1[component]), weights));
    let stress =
        std::array::from_fn(|component| weighted_average(fields.map(|f| f.2[component]), weights));
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    ThermalPlaneState {
        total_strain,
        mechanical_strain,
        thermal_strain: weighted_average(fields.map(|f| f.3), weights),
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
        strain_energy_density: weighted_average(fields.map(|f| f.4), weights),
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
