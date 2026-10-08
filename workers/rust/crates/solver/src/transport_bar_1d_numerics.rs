use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementInput, AdvectionDiffusionBar1dScheme,
    AdvectionDiffusionBar1dStabilization, SolveAdvectionDiffusionBar1dRequest,
};

pub(super) fn checked(id: &str, field: &str, value: f64) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "advection-diffusion element {id}: {field} is not representable"
        ))
    }
}

pub(super) fn product_ratio(
    id: &str,
    field: &str,
    a: f64,
    b: f64,
    divisor: f64,
) -> Result<f64, String> {
    if a == 0.0 || b == 0.0 {
        return Ok(0.0);
    }
    let product = a * b;
    let direct = product / divisor;
    if product.is_normal() && direct.is_normal() {
        return Ok(direct);
    }

    // Restore the combined exponent once; neither an intermediate overflow nor
    // early subnormal rounding should destroy a representable physical value.
    let (ma, ea) = normalized(a.abs());
    let (mb, eb) = normalized(b.abs());
    let (md, ed) = normalized(divisor.abs());
    let mut mantissa = ma * mb / md;
    let mut exponent = ea + eb - ed;
    while mantissa >= 2.0 {
        mantissa *= 0.5;
        exponent += 1;
    }
    while mantissa < 1.0 {
        mantissa *= 2.0;
        exponent -= 1;
    }
    let magnitude = if exponent > 1023 {
        f64::INFINITY
    } else if exponent >= -1022 {
        mantissa * power_of_two(exponent)
    } else if exponent >= -1075 {
        (mantissa * power_of_two(exponent + 1022)) * f64::MIN_POSITIVE
    } else {
        0.0
    };
    let value = if a.is_sign_negative() ^ b.is_sign_negative() ^ divisor.is_sign_negative() {
        -magnitude
    } else {
        magnitude
    };
    checked(id, field, value)?;
    if value == 0.0 {
        return Err(format!(
            "advection-diffusion element {id}: {field} is not representable (underflow)"
        ));
    }
    Ok(value)
}

fn normalized(mut value: f64) -> (f64, i32) {
    let mut adjustment = 0;
    if value < f64::MIN_POSITIVE {
        value *= power_of_two(54);
        adjustment = -54;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 + adjustment;
    (
        f64::from_bits((bits & ((1_u64 << 52) - 1)) | (1023_u64 << 52)),
        exponent,
    )
}

fn power_of_two(exponent: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&exponent));
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

pub(super) fn average(left: f64, right: f64) -> f64 {
    let sum = left + right;
    if sum.is_finite() {
        sum * 0.5
    } else {
        left * 0.5 + right * 0.5
    }
}

pub(super) fn gradient(id: &str, left: f64, right: f64, signed_length: f64) -> Result<f64, String> {
    let difference = right - left;
    let value = if difference.is_finite() {
        difference / signed_length
    } else {
        right / signed_length - left / signed_length
    };
    checked(id, "concentration gradient", value)?;
    if right != left && value == 0.0 {
        return Err(format!(
            "advection-diffusion element {id}: concentration gradient is not representable (underflow)"
        ));
    }
    Ok(value)
}

pub(super) fn local_matrix(
    request: &SolveAdvectionDiffusionBar1dRequest,
    element: &AdvectionDiffusionBar1dElementInput,
) -> Result<[[f64; 2]; 2], String> {
    let signed_length = request.nodes[element.node_j].x - request.nodes[element.node_i].x;
    let diffusion = product_ratio(
        &element.id,
        "diffusion coefficient",
        element.diffusivity,
        element.area,
        signed_length.abs(),
    )?;
    if request.scheme == AdvectionDiffusionBar1dScheme::Upwind {
        let flow = product_ratio(
            &element.id,
            "advective flow",
            element.velocity,
            element.area,
            1.0,
        )? * signed_length.signum();
        // Conservative endpoint balance, without subtracting near-equal large
        // artificial-diffusion/advection terms on the downwind coupling.
        let plus = checked(
            &element.id,
            "local upwind matrix",
            diffusion + flow.max(0.0),
        )?;
        let minus = checked(
            &element.id,
            "local upwind matrix",
            diffusion + (-flow).max(0.0),
        )?;
        return Ok([[plus, -minus], [-plus, minus]]);
    }
    let advection = product_ratio(
        &element.id,
        "advection coefficient",
        element.velocity,
        element.area,
        2.0,
    )? * signed_length.signum();
    let minus = checked(&element.id, "local matrix", diffusion - advection)?;
    let plus = checked(&element.id, "local matrix", diffusion + advection)?;
    Ok([[minus, -minus], [-plus, plus]])
}

pub(super) fn upwind_stabilization(
    element: &AdvectionDiffusionBar1dElementInput,
    signed_length: f64,
    concentrations: [f64; 2],
    gradient: f64,
    diffusive_flux: f64,
) -> Result<AdvectionDiffusionBar1dStabilization, String> {
    let artificial_diffusivity = product_ratio(
        &element.id,
        "artificial diffusivity",
        element.velocity.abs(),
        signed_length.abs() * 0.5,
        1.0,
    )?;
    let stabilization_flux = product_ratio(
        &element.id,
        "stabilization flux",
        -artificial_diffusivity,
        gradient,
        1.0,
    )?;
    let upstream = if element.velocity.is_sign_negative() == signed_length.is_sign_negative() {
        concentrations[0]
    } else {
        concentrations[1]
    };
    let advection = product_ratio(
        &element.id,
        "upwind advective flux",
        element.velocity,
        upstream,
        1.0,
    )?;
    let numerical_flux = checked(&element.id, "numerical flux", diffusive_flux + advection)?;
    Ok(AdvectionDiffusionBar1dStabilization {
        artificial_diffusivity,
        stabilization_flux,
        numerical_flux,
    })
}
