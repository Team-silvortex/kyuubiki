use kyuubiki_protocol::SolveHeatPlaneQuad2dRequest;

pub(crate) fn validate_seed_loads(seed: &SolveHeatPlaneQuad2dRequest) -> Result<(), String> {
    if seed.nodes.iter().any(|node| !node.heat_load.is_finite()) {
        return Err("composite heat request contains non-finite nodal load".into());
    }
    Ok(())
}

pub(crate) fn power_relative_error(actual: f64, expected: f64) -> f64 {
    if !actual.is_finite() || !expected.is_finite() || actual < 0.0 || expected < 0.0 {
        f64::INFINITY
    } else if expected == 0.0 {
        if actual == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        ((actual - expected) / expected).abs()
    }
}

pub(crate) fn sum_power(values: impl IntoIterator<Item = f64>) -> Result<f64, String> {
    let mut total = 0.0;
    let mut correction = 0.0;
    for value in values {
        if !value.is_finite() || value < 0.0 {
            return Err("composite heat power must be finite and non-negative".into());
        }
        let corrected = value - correction;
        let next = total + corrected;
        if !next.is_finite() {
            return Err("composite heat power sum is not representable".into());
        }
        correction = (next - total) - corrected;
        total = next;
    }
    Ok(total)
}

pub(crate) fn add_uniform_quad_power(
    request: &mut SolveHeatPlaneQuad2dRequest,
    indices: [usize; 4],
    power: f64,
) -> Result<f64, String> {
    if !power.is_finite() || power < 0.0 {
        return Err("composite heat power must be finite and non-negative".into());
    }
    for (position, index) in indices.iter().enumerate() {
        if indices[..position].contains(index) || *index >= request.nodes.len() {
            return Err("composite heat element requires four distinct known nodes".into());
        }
    }
    let share = power / 4.0;
    let mut added = [0.0; 4];
    for (position, index) in indices.into_iter().enumerate() {
        let node = &mut request.nodes[index];
        let after = node.heat_load + share;
        added[position] = after - node.heat_load;
        if !after.is_finite() || power_relative_error(added[position], share) > 1.0e-12 {
            return Err(format!(
                "composite heat node {} cannot represent the added power",
                node.id
            ));
        }
        node.heat_load = after;
    }
    let distributed = sum_power(added)?;
    if power_relative_error(distributed, power) > 1.0e-12 {
        return Err("composite heat-load distribution lost energy".into());
    }
    Ok(distributed)
}

pub(crate) fn added_power(
    seed: &SolveHeatPlaneQuad2dRequest,
    projected: &SolveHeatPlaneQuad2dRequest,
) -> Result<f64, String> {
    if seed.nodes.len() != projected.nodes.len() {
        return Err("composite heat projection changed node count".into());
    }
    // Subtract at each node before summation. An unrelated large pre-existing
    // load must not round away the new power in two whole-model totals.
    sum_power(
        seed.nodes
            .iter()
            .zip(&projected.nodes)
            .map(|(before, after)| after.heat_load - before.heat_load),
    )
}
