use crate::heat_plane_2d_element::{
    HeatPlaneQuadComputed, HeatPlaneTriangleComputed, plane_triangle_scalar_gradient,
};
use crate::solver_control::SolverStage;
use crate::solver_postprocess::{fold_results, try_collect_results};
use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneNodeResult, HeatPlaneQuadElementInput, HeatPlaneQuadElementResult,
    HeatPlaneTriangleElementInput, HeatPlaneTriangleElementResult,
};

pub(super) fn recover_nodes(
    nodes: &[HeatPlaneNodeInput],
    relative: &[f64],
    reference: f64,
) -> Result<Vec<HeatPlaneNodeResult>, String> {
    try_collect_results(
        SolverStage::ResultNodes,
        nodes.iter().enumerate().map(|(index, node)| {
            let temperature = if node.fix_temperature {
                node.temperature
            } else {
                relative[index] + reference
            };
            if !temperature.is_finite() {
                return Err(format!(
                    "heat plane node {index} ({}): temperature is not representable",
                    node.id
                ));
            }
            Ok(HeatPlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                temperature,
                heat_load: node.heat_load,
            })
        }),
    )
}

pub(super) fn recover_triangle(
    index: usize,
    element: &HeatPlaneTriangleElementInput,
    computed: &HeatPlaneTriangleComputed,
    relative: &[f64],
    reference: f64,
) -> Result<HeatPlaneTriangleElementResult, String> {
    let values = [element.node_i, element.node_j, element.node_k].map(|i| relative[i]);
    let gradient =
        plane_triangle_scalar_gradient(&computed.gradient_x, &computed.gradient_y, &values);
    let fields = HeatFields::recover(
        &element.id,
        gradient,
        element.conductivity,
        computed.area,
        element.thickness,
    )?;
    Ok(HeatPlaneTriangleElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        area: computed.area,
        average_temperature: average_temperature(&element.id, &values, reference)?,
        temperature_gradient_x: gradient[0],
        temperature_gradient_y: gradient[1],
        heat_flux_x: fields.flux[0],
        heat_flux_y: fields.flux[1],
        heat_flux_magnitude: fields.magnitude,
        heat_flow_rate: fields.flow,
    })
}

pub(super) fn recover_quad(
    index: usize,
    element: &HeatPlaneQuadElementInput,
    computed: &HeatPlaneQuadComputed,
    relative: &[f64],
    reference: f64,
) -> Result<HeatPlaneQuadElementResult, String> {
    let values = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ]
    .map(|i| relative[i]);
    let first = plane_triangle_scalar_gradient(
        &computed.first.gradient_x,
        &computed.first.gradient_y,
        &[values[0], values[1], values[2]],
    );
    let second = plane_triangle_scalar_gradient(
        &computed.second.gradient_x,
        &computed.second.gradient_y,
        &[values[0], values[2], values[3]],
    );
    let area = computed.first.area + computed.second.area;
    let gradient = [0, 1].map(|axis| {
        first[axis] * (computed.first.area / area) + second[axis] * (computed.second.area / area)
    });
    let fields = HeatFields::recover(
        &element.id,
        gradient,
        element.conductivity,
        area,
        element.thickness,
    )?;
    Ok(HeatPlaneQuadElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        node_l: element.node_l,
        area,
        average_temperature: average_temperature(&element.id, &values, reference)?,
        temperature_gradient_x: gradient[0],
        temperature_gradient_y: gradient[1],
        heat_flux_x: fields.flux[0],
        heat_flux_y: fields.flux[1],
        heat_flux_magnitude: fields.magnitude,
        heat_flow_rate: fields.flow,
    })
}

struct HeatFields {
    flux: [f64; 2],
    magnitude: f64,
    flow: f64,
}

impl HeatFields {
    fn recover(
        id: &str,
        gradient: [f64; 2],
        conductivity: f64,
        area: f64,
        thickness: f64,
    ) -> Result<Self, String> {
        let flux = gradient.map(|value| -conductivity * value);
        if gradient.iter().zip(flux).any(|(&gradient, flux)| {
            !gradient.is_finite() || !flux.is_finite() || gradient != 0.0 && flux == 0.0
        }) {
            return Err(format!(
                "heat plane element {id}: gradient or heat flux is not representable"
            ));
        }
        let magnitude = flux[0].hypot(flux[1]);
        if !magnitude.is_finite() {
            return Err(format!(
                "heat plane element {id}: heat flux magnitude is not representable"
            ));
        }
        // Pair extremes before the remaining factor, avoiding an intermediate
        // overflow/underflow when the final positive product is representable.
        let mut factors = [magnitude, area, thickness];
        factors.sort_unstable_by(f64::total_cmp);
        let flow = (factors[0] * factors[2]) * factors[1];
        if !area.is_finite() || !flow.is_finite() || magnitude != 0.0 && flow == 0.0 {
            return Err(format!(
                "heat plane element {id}: heat flow rate is not representable"
            ));
        }
        Ok(Self {
            flux,
            magnitude,
            flow,
        })
    }
}

fn average_temperature(id: &str, values: &[f64], reference: f64) -> Result<f64, String> {
    let scale = values.iter().map(|value| value.abs()).fold(0.0, f64::max);
    let mut sum = 0.0_f64;
    let mut correction = 0.0;
    if scale != 0.0 {
        for value in values {
            let value = value / scale;
            let next = sum + value;
            correction += if sum.abs() >= value.abs() {
                (sum - next) + value
            } else {
                (value - next) + sum
            };
            sum = next;
        }
    }
    let mean = ((sum + correction) / values.len() as f64) * scale + reference;
    if !mean.is_finite() {
        return Err(format!(
            "heat plane element {id}: average temperature is not representable"
        ));
    }
    Ok(mean)
}

pub(super) fn total_heat_flow(flows: impl ExactSizeIterator<Item = f64>) -> Result<f64, String> {
    let total = fold_results(SolverStage::ResultTotals, flows, -0.0, |sum, flow| {
        sum + flow.abs()
    })?;
    if !total.is_finite() {
        return Err("heat plane total heat flow rate is not representable".into());
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_preserves_large_values_tiny_constants_and_cancelling_terms() {
        assert_eq!(
            average_temperature("e", &[f64::MAX; 3], 0.0).unwrap(),
            f64::MAX
        );
        let tiny = f64::from_bits(1);
        assert_eq!(average_temperature("e", &[tiny; 4], 0.0).unwrap(), tiny);
        let mean = average_temperature("e", &[1e308, 1.0, -1e308], 0.0).unwrap();
        assert!((mean - 1.0 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn heat_flow_volume_product_pairs_extremes_before_the_remaining_factor() {
        for (flux, area, thickness) in [(1e200, 1e200, 1e-200), (1e-320, 1e-6, 1e6)] {
            let result = HeatFields::recover("e", [flux, 0.0], 1.0, area, thickness).unwrap();
            assert!(result.flow > 0.0 && (result.flow / flux - 1.0).abs() < 1e-4);
        }
        for (flux, area, thickness) in [(1e200, 1e100, 1e100), (1e-200, 1e-100, 1e-100)] {
            let error = HeatFields::recover("e", [flux, 0.0], 1.0, area, thickness)
                .err()
                .unwrap();
            assert!(error.contains("heat flow rate"), "{error}");
        }
    }

    #[test]
    fn absolute_temperature_overflow_is_rejected_and_clean_replay_succeeds() {
        let nodes = [HeatPlaneNodeInput {
            id: "free".into(),
            x: 0.0,
            y: 0.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        }];
        let error = recover_nodes(&nodes, &[1e308], 1e308).unwrap_err();
        assert!(
            error.contains("free") && error.contains("temperature"),
            "{error}"
        );
        assert_eq!(
            recover_nodes(&nodes, &[1.0], 20.0).unwrap()[0].temperature,
            21.0
        );
    }
}
