use kyuubiki_protocol::{
    HeatPlaneQuadElementInput, SolveElectrostaticPlaneQuad2dResult, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest,
};
use serde::{Deserialize, Serialize};

pub const COMPOSITE_ELECTROTHERMAL_LOSS_SCHEMA_VERSION: &str =
    "kyuubiki.composite-electrothermal-loss-projection/v1";
pub const COMPOSITE_HEAT_TO_THERMAL_SCHEMA_VERSION: &str =
    "kyuubiki.composite-heat-to-thermal-projection/v1";

const VACUUM_PERMITTIVITY_F_M: f64 = 8.854_187_812_8e-12;
const COORDINATE_TOLERANCE_M: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeDielectricLossSpec {
    pub source_element_id: String,
    pub frequency_hz: f64,
    pub relative_permittivity: f64,
    pub loss_tangent: f64,
    pub reference_temperature_c: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrothermalLossProjection {
    pub schema_version: String,
    pub model: String,
    pub source_element_id: String,
    pub frequency_hz: f64,
    pub relative_permittivity: f64,
    pub loss_tangent: f64,
    pub effective_conductivity_s_m: f64,
    pub electric_field_rms_v_m: f64,
    pub volumetric_loss_w_m3: f64,
    pub source_volume_m3: f64,
    pub total_loss_w: f64,
    pub distributed_total_heat_load_w: f64,
    pub energy_balance_relative_error: f64,
    pub target_element_count: usize,
    pub target_node_count: usize,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeHeatToThermalProjection {
    pub schema_version: String,
    pub reference_temperature_c: f64,
    pub mapped_node_count: usize,
    pub minimum_temperature_delta_c: f64,
    pub maximum_temperature_delta_c: f64,
    pub maximum_coordinate_error_m: f64,
}

pub fn project_composite_dielectric_loss_to_heat(
    electrostatic: &SolveElectrostaticPlaneQuad2dResult,
    heat_seed: &SolveHeatPlaneQuad2dRequest,
    spec: &CompositeDielectricLossSpec,
) -> Result<
    (
        SolveHeatPlaneQuad2dRequest,
        CompositeElectrothermalLossProjection,
    ),
    String,
> {
    validate_loss_spec(spec)?;
    let field = electrostatic
        .elements
        .iter()
        .find(|element| element.id == spec.source_element_id)
        .ok_or_else(|| {
            format!(
                "electrostatic result is missing source element {}",
                spec.source_element_id
            )
        })?;
    let input = electrostatic
        .input
        .elements
        .iter()
        .find(|element| element.id == spec.source_element_id)
        .ok_or_else(|| {
            format!(
                "electrostatic input is missing source element {}",
                spec.source_element_id
            )
        })?;
    if !field.area.is_finite()
        || field.area <= 0.0
        || !input.thickness.is_finite()
        || input.thickness <= 0.0
    {
        return Err("electrothermal source element must have finite positive geometry".to_string());
    }
    if !field.electric_field_magnitude.is_finite() {
        return Err("electrothermal source field must be finite".to_string());
    }
    validate_matching_element_geometry(electrostatic, heat_seed, input, &spec.source_element_id)?;

    let angular_frequency = 2.0 * std::f64::consts::PI * spec.frequency_hz;
    let effective_conductivity = angular_frequency
        * VACUUM_PERMITTIVITY_F_M
        * spec.relative_permittivity
        * spec.loss_tangent;
    let volumetric_loss = effective_conductivity * field.electric_field_magnitude.powi(2);
    let source_volume = field.area * input.thickness;
    let total_loss = volumetric_loss * source_volume;
    let heat_request = distribute_composite_dielectric_heat_load(heat_seed, total_loss)?;
    let distributed_total = heat_request
        .nodes
        .iter()
        .map(|node| node.heat_load)
        .sum::<f64>();
    let energy_balance_relative_error = relative_error(distributed_total, total_loss);
    let target_elements = dielectric_elements(&heat_request);
    let target_node_count = unique_node_indices(&target_elements).len();
    let target_element_count = target_elements.len();

    Ok((
        heat_request,
        CompositeElectrothermalLossProjection {
            schema_version: COMPOSITE_ELECTROTHERMAL_LOSS_SCHEMA_VERSION.to_string(),
            model: "harmonic_dielectric_loss_sigma_eff_e_rms_squared".to_string(),
            source_element_id: spec.source_element_id.clone(),
            frequency_hz: spec.frequency_hz,
            relative_permittivity: spec.relative_permittivity,
            loss_tangent: spec.loss_tangent,
            effective_conductivity_s_m: effective_conductivity,
            electric_field_rms_v_m: field.electric_field_magnitude,
            volumetric_loss_w_m3: volumetric_loss,
            source_volume_m3: source_volume,
            total_loss_w: total_loss,
            distributed_total_heat_load_w: distributed_total,
            energy_balance_relative_error,
            target_element_count,
            target_node_count,
            assumptions: vec![
                "The solved electrostatic field magnitude is interpreted as an RMS harmonic field."
                    .to_string(),
                "Relative permittivity and loss tangent are scalar, isotropic, and frequency-local screening values."
                    .to_string(),
                "Element dielectric loss is lumped consistently to its four thermal nodes."
                    .to_string(),
            ],
        },
    ))
}

pub fn distribute_composite_dielectric_heat_load(
    heat_seed: &SolveHeatPlaneQuad2dRequest,
    total_heat_load_w: f64,
) -> Result<SolveHeatPlaneQuad2dRequest, String> {
    if !total_heat_load_w.is_finite() || total_heat_load_w < 0.0 {
        return Err("composite dielectric heat load must be finite and non-negative".to_string());
    }
    let targets = dielectric_elements(heat_seed);
    if targets.is_empty() {
        return Err("composite heat model is missing dielectric elements".to_string());
    }
    let mut weighted = Vec::with_capacity(targets.len());
    for element in targets {
        let area = quad_area(heat_seed, element)?;
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err(format!("heat element {} has invalid thickness", element.id));
        }
        weighted.push((element, area * element.thickness));
    }
    let total_volume = weighted.iter().map(|(_, volume)| volume).sum::<f64>();
    if !total_volume.is_finite() || total_volume <= 0.0 {
        return Err("composite dielectric region must have positive volume".to_string());
    }
    let mut request = heat_seed.clone();
    for node in &mut request.nodes {
        node.heat_load = 0.0;
    }
    for (element, volume) in weighted {
        let nodal_load = total_heat_load_w * volume / total_volume / 4.0;
        for index in element_nodes(element) {
            let node = request.nodes.get_mut(index).ok_or_else(|| {
                format!(
                    "heat element {} references unknown node {index}",
                    element.id
                )
            })?;
            node.heat_load += nodal_load;
        }
    }
    let distributed = request.nodes.iter().map(|node| node.heat_load).sum::<f64>();
    if relative_error(distributed, total_heat_load_w) > 1.0e-12 {
        return Err("composite dielectric heat-load distribution lost energy".to_string());
    }
    Ok(request)
}

pub fn project_composite_heat_to_thermal(
    heat: &SolveHeatPlaneQuad2dResult,
    thermal_seed: &SolveThermalPlaneQuad2dRequest,
    reference_temperature_c: f64,
) -> Result<
    (
        SolveThermalPlaneQuad2dRequest,
        CompositeHeatToThermalProjection,
    ),
    String,
> {
    if !reference_temperature_c.is_finite() {
        return Err("thermal projection reference temperature must be finite".to_string());
    }
    if heat.nodes.len() != thermal_seed.nodes.len() {
        return Err("heat and thermal projections require equal node counts".to_string());
    }
    let mut request = thermal_seed.clone();
    let mut minimum_delta = f64::INFINITY;
    let mut maximum_delta = f64::NEG_INFINITY;
    let mut maximum_coordinate_error = 0.0_f64;
    for target in &mut request.nodes {
        let source = heat
            .nodes
            .iter()
            .find(|node| node.id == target.id)
            .ok_or_else(|| format!("heat result is missing thermal node {}", target.id))?;
        let coordinate_error = (source.x - target.x).hypot(source.y - target.y);
        if !coordinate_error.is_finite() || coordinate_error > COORDINATE_TOLERANCE_M {
            return Err(format!(
                "heat and thermal node {} coordinates do not match",
                target.id
            ));
        }
        if !source.temperature.is_finite() {
            return Err(format!("heat node {} temperature is not finite", target.id));
        }
        target.temperature_delta = source.temperature - reference_temperature_c;
        minimum_delta = minimum_delta.min(target.temperature_delta);
        maximum_delta = maximum_delta.max(target.temperature_delta);
        maximum_coordinate_error = maximum_coordinate_error.max(coordinate_error);
    }
    Ok((
        request,
        CompositeHeatToThermalProjection {
            schema_version: COMPOSITE_HEAT_TO_THERMAL_SCHEMA_VERSION.to_string(),
            reference_temperature_c,
            mapped_node_count: heat.nodes.len(),
            minimum_temperature_delta_c: minimum_delta,
            maximum_temperature_delta_c: maximum_delta,
            maximum_coordinate_error_m: maximum_coordinate_error,
        },
    ))
}

fn validate_loss_spec(spec: &CompositeDielectricLossSpec) -> Result<(), String> {
    if spec.source_element_id.trim().is_empty() {
        return Err("electrothermal source element id must not be empty".to_string());
    }
    if !spec.frequency_hz.is_finite() || spec.frequency_hz <= 0.0 {
        return Err("electrothermal frequency must be finite and positive".to_string());
    }
    if !spec.relative_permittivity.is_finite() || spec.relative_permittivity <= 0.0 {
        return Err("electrothermal relative permittivity must be finite and positive".to_string());
    }
    if !spec.loss_tangent.is_finite() || spec.loss_tangent < 0.0 {
        return Err("electrothermal loss tangent must be finite and non-negative".to_string());
    }
    if !spec.reference_temperature_c.is_finite() {
        return Err("electrothermal reference temperature must be finite".to_string());
    }
    Ok(())
}

fn validate_matching_element_geometry(
    electrostatic: &SolveElectrostaticPlaneQuad2dResult,
    heat: &SolveHeatPlaneQuad2dRequest,
    source: &kyuubiki_protocol::ElectrostaticPlaneQuadElementInput,
    target_id: &str,
) -> Result<(), String> {
    let target = heat
        .elements
        .iter()
        .find(|element| element.id == target_id)
        .ok_or_else(|| format!("heat model is missing target element {target_id}"))?;
    let source_coordinates = element_nodes_electrostatic(source)
        .into_iter()
        .map(|index| {
            electrostatic
                .input
                .nodes
                .get(index)
                .map(|node| (node.x, node.y))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("electrostatic element {target_id} references an unknown node"))?;
    let target_coordinates = element_nodes(target)
        .into_iter()
        .map(|index| heat.nodes.get(index).map(|node| (node.x, node.y)))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("heat element {target_id} references an unknown node"))?;
    for coordinate in source_coordinates {
        if !target_coordinates.iter().any(|target| {
            (target.0 - coordinate.0).hypot(target.1 - coordinate.1) <= COORDINATE_TOLERANCE_M
        }) {
            return Err(format!(
                "electrostatic and heat element {target_id} geometry does not match"
            ));
        }
    }
    Ok(())
}

fn dielectric_elements(request: &SolveHeatPlaneQuad2dRequest) -> Vec<&HeatPlaneQuadElementInput> {
    request
        .elements
        .iter()
        .filter(|element| {
            element.id == "dielectric_core" || element.id.starts_with("layer_1_element_")
        })
        .collect()
}

fn quad_area(
    request: &SolveHeatPlaneQuad2dRequest,
    element: &HeatPlaneQuadElementInput,
) -> Result<f64, String> {
    let coordinates = element_nodes(element)
        .into_iter()
        .map(|index| request.nodes.get(index).map(|node| (node.x, node.y)))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("heat element {} references an unknown node", element.id))?;
    let twice_area = coordinates
        .iter()
        .zip(coordinates.iter().cycle().skip(1))
        .take(coordinates.len())
        .map(|((x1, y1), (x2, y2))| x1 * y2 - x2 * y1)
        .sum::<f64>();
    let area = 0.5 * twice_area.abs();
    if !area.is_finite() || area <= 0.0 {
        return Err(format!("heat element {} has invalid area", element.id));
    }
    Ok(area)
}

fn element_nodes(element: &HeatPlaneQuadElementInput) -> [usize; 4] {
    [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ]
}

fn element_nodes_electrostatic(
    element: &kyuubiki_protocol::ElectrostaticPlaneQuadElementInput,
) -> [usize; 4] {
    [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ]
}

fn unique_node_indices(elements: &[&HeatPlaneQuadElementInput]) -> Vec<usize> {
    let mut indices = elements
        .iter()
        .flat_map(|element| element_nodes(element))
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    indices
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    if expected.abs() <= f64::EPSILON {
        actual.abs()
    } else {
        (actual - expected).abs() / expected.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompositeDielectricLossSpec, distribute_composite_dielectric_heat_load,
        project_composite_dielectric_loss_to_heat, project_composite_heat_to_thermal,
    };
    use kyuubiki_protocol::{
        SolveElectrostaticPlaneQuad2dResult, SolveHeatPlaneQuad2dRequest,
        SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest,
    };
    use serde_json::json;

    #[test]
    fn dielectric_loss_projection_conserves_power() {
        let electrostatic = electrostatic_result();
        let heat = heat_request();
        let spec = CompositeDielectricLossSpec {
            source_element_id: "dielectric_core".to_string(),
            frequency_hz: 10.0e6,
            relative_permittivity: 3.4,
            loss_tangent: 0.008,
            reference_temperature_c: 35.0,
        };

        let (request, projection) =
            project_composite_dielectric_loss_to_heat(&electrostatic, &heat, &spec)
                .expect("projection should pass");

        assert!(projection.total_loss_w > 0.0);
        assert_eq!(projection.energy_balance_relative_error, 0.0);
        assert_eq!(projection.target_element_count, 1);
        assert_eq!(projection.target_node_count, 4);
        assert_eq!(
            request.nodes.iter().map(|node| node.heat_load).sum::<f64>(),
            projection.total_loss_w
        );
    }

    #[test]
    fn distributed_refinement_load_is_volume_weighted_and_conservative() {
        let mut heat = heat_request();
        let second = heat.elements[0].clone();
        heat.elements[0].id = "layer_1_element_1".to_string();
        heat.elements
            .push(kyuubiki_protocol::HeatPlaneQuadElementInput {
                id: "layer_1_element_2".to_string(),
                ..second
            });

        let request = distribute_composite_dielectric_heat_load(&heat, 0.25)
            .expect("distribution should pass");

        assert!(
            (request.nodes.iter().map(|node| node.heat_load).sum::<f64>() - 0.25).abs() < 1.0e-15
        );
    }

    #[test]
    fn heat_projection_requires_matching_thermal_coordinates() {
        let heat: SolveHeatPlaneQuad2dResult = serde_json::from_value(json!({
            "input": {"nodes": [], "elements": []},
            "nodes": [{
                "index": 0, "id": "n0", "x": 0.0, "y": 0.0,
                "temperature": 42.5, "heat_load": 0.1
            }],
            "elements": [], "max_temperature": 42.5, "max_heat_flux": 0.0,
            "total_abs_heat_flow_rate": 0.0
        }))
        .expect("heat result");
        let mut thermal: SolveThermalPlaneQuad2dRequest = serde_json::from_value(json!({
            "nodes": [{
                "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true,
                "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0
            }],
            "elements": []
        }))
        .expect("thermal request");

        let (projected, evidence) =
            project_composite_heat_to_thermal(&heat, &thermal, 35.0).expect("projection");
        assert_eq!(projected.nodes[0].temperature_delta, 7.5);
        assert_eq!(evidence.mapped_node_count, 1);

        thermal.nodes[0].x = 0.1;
        assert!(project_composite_heat_to_thermal(&heat, &thermal, 35.0).is_err());
    }

    fn electrostatic_result() -> SolveElectrostaticPlaneQuad2dResult {
        serde_json::from_value(json!({
            "input": {
                "nodes": electrostatic_nodes(),
                "elements": [{
                    "id": "dielectric_core", "node_i": 0, "node_j": 1,
                    "node_k": 2, "node_l": 3, "thickness": 0.001,
                    "permittivity": 3.4
                }]
            },
            "nodes": [],
            "elements": [{
                "index": 0, "id": "dielectric_core", "node_i": 0, "node_j": 1,
                "node_k": 2, "node_l": 3, "area": 0.0009,
                "average_potential": 0.0, "potential_gradient_x": 0.0,
                "potential_gradient_y": 0.0, "electric_field_x": 1000.0,
                "electric_field_y": 0.0, "electric_field_magnitude": 1000.0,
                "electric_flux_density_x": 0.0, "electric_flux_density_y": 0.0,
                "electric_flux_density_magnitude": 0.0, "electric_energy_density": 0.0,
                "stored_energy": 0.0
            }],
            "max_potential": 0.0, "max_electric_field": 1000.0,
            "max_flux_density": 0.0, "max_electric_energy_density": 0.0,
            "total_stored_energy": 0.0
        }))
        .expect("electrostatic result")
    }

    fn electrostatic_nodes() -> serde_json::Value {
        json!([
            {"id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0},
            {"id": "n1", "x": 0.03, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0},
            {"id": "n2", "x": 0.03, "y": 0.03, "fix_potential": false, "potential": 0.0, "charge_density": 0.0},
            {"id": "n3", "x": 0.0, "y": 0.03, "fix_potential": true, "potential": 0.0, "charge_density": 0.0}
        ])
    }

    fn heat_request() -> SolveHeatPlaneQuad2dRequest {
        serde_json::from_value(json!({
            "nodes": [
                {"id": "n0", "x": 0.0, "y": 0.0, "fix_temperature": false, "temperature": 35.0, "heat_load": 1.0},
                {"id": "n1", "x": 0.03, "y": 0.0, "fix_temperature": false, "temperature": 35.0, "heat_load": 1.0},
                {"id": "n2", "x": 0.03, "y": 0.03, "fix_temperature": true, "temperature": 35.0, "heat_load": 1.0},
                {"id": "n3", "x": 0.0, "y": 0.03, "fix_temperature": false, "temperature": 35.0, "heat_load": 1.0}
            ],
            "elements": [{
                "id": "dielectric_core", "node_i": 0, "node_j": 1,
                "node_k": 2, "node_l": 3, "thickness": 0.001,
                "conductivity": 0.25
            }]
        }))
        .expect("heat request")
    }
}
