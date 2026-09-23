use kyuubiki_protocol::{
    HeatPlaneNodeResult, SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest,
};
use std::collections::HashMap;

const COORDINATE_TOLERANCE_M: f64 = 1.0e-12;

pub(crate) struct CompositeTemperatureMap<'a> {
    // Source records in target-node order; source indices still refer to the heat input.
    pub nodes: Vec<&'a HeatPlaneNodeResult>,
    pub maximum_coordinate_error_m: f64,
}

pub(crate) fn map_composite_temperatures<'a>(
    heat: &'a SolveHeatPlaneQuad2dResult,
    thermal: &SolveThermalPlaneQuad2dRequest,
) -> Result<CompositeTemperatureMap<'a>, String> {
    let count = heat.nodes.len();
    if count == 0 || count != thermal.nodes.len() || count != heat.input.nodes.len() {
        return Err(
            "heat and thermal projections require complete nonempty equal node sets".into(),
        );
    }
    let mut by_id = HashMap::with_capacity(count);
    let mut seen_indices = vec![false; count];
    for source in &heat.nodes {
        let input =
            heat.input.nodes.get(source.index).ok_or_else(|| {
                format!("heat result node {} has an unknown input index", source.id)
            })?;
        if source.id.trim().is_empty()
            || seen_indices[source.index]
            || by_id.insert(source.id.as_str(), source).is_some()
        {
            return Err("heat projection node IDs and indices must be nonempty and unique".into());
        }
        seen_indices[source.index] = true;
        if source.id != input.id || coordinate_error(source.x, source.y, input.x, input.y).is_err()
        {
            return Err(format!(
                "heat result node {} does not match its input mesh",
                source.id
            ));
        }
        if !source.temperature.is_finite() {
            return Err(format!("heat node {} temperature is not finite", source.id));
        }
    }
    let mut nodes = Vec::with_capacity(count);
    let mut maximum_coordinate_error = 0.0_f64;
    for target in &thermal.nodes {
        // Removing a matched ID enforces a bijection without a second target index.
        let source = by_id.remove(target.id.as_str()).ok_or_else(|| {
            format!(
                "thermal node {} is missing from heat results or repeated",
                target.id
            )
        })?;
        let error = coordinate_error(source.x, source.y, target.x, target.y).map_err(|()| {
            format!(
                "heat and thermal node {} coordinates do not match",
                target.id
            )
        })?;
        maximum_coordinate_error = maximum_coordinate_error.max(error);
        nodes.push(source);
    }
    Ok(CompositeTemperatureMap {
        nodes,
        maximum_coordinate_error_m: maximum_coordinate_error,
    })
}

fn coordinate_error(x: f64, y: f64, target_x: f64, target_y: f64) -> Result<f64, ()> {
    let error = (x - target_x).hypot(y - target_y);
    if error.is_finite() && error <= COORDINATE_TOLERANCE_M {
        Ok(error)
    } else {
        Err(())
    }
}
