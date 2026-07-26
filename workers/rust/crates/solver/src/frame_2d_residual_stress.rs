use kyuubiki_protocol::{Frame2dResidualStressTemplateInput, Frame2dSectionFiberInput};

pub(crate) fn apply_residual_stress_template(
    element_id: &str,
    template: Option<&Frame2dResidualStressTemplateInput>,
    fibers: &mut [Frame2dSectionFiberInput],
) -> Result<(), String> {
    let Some(template) = template else {
        return Ok(());
    };
    if fibers.len() < 3 {
        return Err(format!(
            "frame 2d material '{element_id}' residual_stress_template requires at least 3 section fibers"
        ));
    }
    if fibers.iter().any(|fiber| fiber.initial_axial_stress != 0.0) {
        return Err(format!(
            "frame 2d material '{element_id}' cannot combine residual_stress_template with explicit fiber initial_axial_stress"
        ));
    }
    match template {
        Frame2dResidualStressTemplateInput::SelfEquilibratedQuadratic { peak_stress } => {
            apply_self_equilibrated_quadratic(element_id, *peak_stress, fibers)
        }
    }
}

fn apply_self_equilibrated_quadratic(
    element_id: &str,
    peak_stress: f64,
    fibers: &mut [Frame2dSectionFiberInput],
) -> Result<(), String> {
    if !(peak_stress.is_finite() && peak_stress != 0.0) {
        return Err(format!(
            "frame 2d material '{element_id}' residual_stress_template peak_stress must be finite and nonzero"
        ));
    }
    let max_y = fibers
        .iter()
        .map(|fiber| fiber.y.abs())
        .fold(0.0_f64, f64::max);
    if !(max_y.is_finite() && max_y > 0.0) {
        return Err(format!(
            "frame 2d material '{element_id}' residual_stress_template requires nonzero section depth"
        ));
    }

    let area = weighted_sum(fibers, |_| 1.0);
    let first_moment = weighted_sum(fibers, |fiber| fiber.y);
    let second_moment = weighted_sum(fibers, |fiber| fiber.y * fiber.y);
    let raw_force = weighted_sum(fibers, |fiber| (fiber.y / max_y).powi(2));
    let raw_moment = weighted_sum(fibers, |fiber| fiber.y * (fiber.y / max_y).powi(2));
    let determinant = area * second_moment - first_moment * first_moment;
    let scale = (area * second_moment)
        .abs()
        .max(first_moment.powi(2))
        .max(f64::MIN_POSITIVE);
    if !(determinant.is_finite() && determinant > scale * 1.0e-12) {
        return Err(format!(
            "frame 2d material '{element_id}' residual_stress_template projection is singular"
        ));
    }
    let offset = (raw_force * second_moment - raw_moment * first_moment) / determinant;
    let slope = (raw_moment * area - raw_force * first_moment) / determinant;
    let projected = fibers
        .iter()
        .map(|fiber| (fiber.y / max_y).powi(2) - offset - slope * fiber.y)
        .collect::<Vec<_>>();
    let peak = projected
        .iter()
        .map(|value| value.abs())
        .fold(0.0, f64::max);
    if !(peak.is_finite() && peak > 1.0e-12) {
        return Err(format!(
            "frame 2d material '{element_id}' residual_stress_template collapses on this fiber layout"
        ));
    }
    for (fiber, value) in fibers.iter_mut().zip(projected) {
        fiber.initial_axial_stress = peak_stress * value / peak;
    }
    Ok(())
}

fn weighted_sum(
    fibers: &[Frame2dSectionFiberInput],
    value: impl Fn(&Frame2dSectionFiberInput) -> f64,
) -> f64 {
    fibers.iter().map(|fiber| fiber.area * value(fiber)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratic_template_is_force_and_moment_free_on_an_asymmetric_layout() {
        let mut fibers = [
            fiber(-0.5, 1.0),
            fiber(-0.1, 2.0),
            fiber(0.2, 1.5),
            fiber(0.7, 0.5),
        ];
        apply_self_equilibrated_quadratic("e0", 80.0, &mut fibers).unwrap();

        let force = weighted_sum(&fibers, |fiber| fiber.initial_axial_stress);
        let moment = weighted_sum(&fibers, |fiber| fiber.y * fiber.initial_axial_stress);
        let peak = fibers
            .iter()
            .map(|fiber| fiber.initial_axial_stress.abs())
            .fold(0.0_f64, f64::max);
        assert!(force.abs() < 1.0e-12);
        assert!(moment.abs() < 1.0e-12);
        assert!((peak - 80.0).abs() < 1.0e-12);
    }

    #[test]
    fn template_rejects_ambiguous_or_collapsed_inputs() {
        let template =
            Frame2dResidualStressTemplateInput::SelfEquilibratedQuadratic { peak_stress: 1.0 };
        let mut explicit = [fiber(-1.0, 1.0), fiber(0.0, 1.0), fiber(1.0, 1.0)];
        explicit[0].initial_axial_stress = 1.0;
        assert!(
            apply_residual_stress_template("e0", Some(&template), &mut explicit)
                .unwrap_err()
                .contains("cannot combine")
        );
        let mut two = [fiber(-1.0, 1.0), fiber(1.0, 1.0)];
        assert!(
            apply_residual_stress_template("e0", Some(&template), &mut two)
                .unwrap_err()
                .contains("at least 3")
        );
    }

    fn fiber(y: f64, area: f64) -> Frame2dSectionFiberInput {
        Frame2dSectionFiberInput {
            y,
            area,
            initial_axial_stress: 0.0,
            material_id: None,
        }
    }
}
