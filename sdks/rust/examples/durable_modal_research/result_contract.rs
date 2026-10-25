use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Check reported fields, not the eigenproblem. The caller separately validates
/// the supported model, spectrum, constraints, residuals and mass orthogonality.
pub fn validate(input: &Value, output: &Value) -> Result<()> {
    // This example supplies a complete fixed input, without implicit defaults.
    if !input.is_object() || output.get("input") != Some(input) {
        return Err("modal result input echo does not match the submitted reference".into());
    }
    let modes = output["modes"]
        .as_array()
        .filter(|modes| !modes.is_empty())
        .ok_or("modal result requires nonempty modes")?;
    let mut minimum = f64::INFINITY;
    let mut maximum = 0.0_f64;
    for (index, mode) in modes.iter().enumerate() {
        let angular = format!("mode {index} angular frequency");
        let lambda = positive(mode, "eigenvalue_rad_s_squared", &angular)?;
        let omega = positive(mode, "natural_frequency_rad_s", &angular)?;
        close(omega, lambda.sqrt(), &angular)?;
        let frequency = positive(mode, "natural_frequency_hz", &angular)?;
        close(omega, frequency * std::f64::consts::TAU, &angular)?;

        let participation = format!("mode {index} participation norm");
        let declared = positive(mode, "participation_norm", &participation)?;
        let shape = mode["shape"]
            .as_array()
            .ok_or_else(|| format!("{participation}: shape missing"))?;
        let mut norm = 0.0_f64;
        for value in shape {
            let value = value
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("{participation}: invalid shape component"))?;
            norm = norm.hypot(value);
        }
        close(declared, norm, &participation)?;
        minimum = minimum.min(frequency);
        maximum = maximum.max(frequency);
    }
    let low = positive(output, "min_frequency_hz", "frequency summary")?;
    let high = positive(output, "max_frequency_hz", "frequency summary")?;
    if low > high {
        return Err("modal result frequency summary is inverted".into());
    }
    close(low, minimum, "frequency summary minimum")?;
    close(high, maximum, "frequency summary maximum")?;
    Ok(())
}

fn positive(object: &Value, key: &str, context: &str) -> Result<f64> {
    object[key]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| format!("modal result {context}: missing positive finite {key}").into())
}

fn close(actual: f64, expected: f64, context: &str) -> Result<()> {
    if !expected.is_finite() || expected <= 0.0 || (actual / expected - 1.0).abs() > 1e-12 {
        return Err(format!("modal result {context} disagrees with recomputed value").into());
    }
    Ok(())
}
