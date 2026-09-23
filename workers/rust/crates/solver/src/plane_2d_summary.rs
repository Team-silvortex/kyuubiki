use crate::solver_control::{SolverStage, checkpoint};
use kyuubiki_protocol::{
    PlaneNodeResult, PlaneQuadElementInput, PlaneQuadElementResult, PlaneTriangleElementInput,
    PlaneTriangleElementResult,
};

pub(super) fn validate_plane_result(id: &str, fields: &[f64]) -> Result<(), String> {
    if fields.iter().any(|value| !value.is_finite()) {
        return Err(format!(
            "plane element {id}: recovered state or energy is not representable"
        ));
    }
    Ok(())
}

pub(super) fn max_plane_displacement(nodes: &[PlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_triangle_stress(elements: &[PlaneTriangleElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_quad_stress(elements: &[PlaneQuadElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn triangle_total_strain_energy(
    elements: &[PlaneTriangleElementResult],
    inputs: &[PlaneTriangleElementInput],
) -> Result<f64, String> {
    total_energy(elements.iter().zip(inputs.iter()).map(|(element, input)| {
        (
            element.id.as_str(),
            element.strain_energy_density,
            element.area,
            input.thickness,
        )
    }))
}

pub(super) fn quad_total_strain_energy(
    elements: &[PlaneQuadElementResult],
    inputs: &[PlaneQuadElementInput],
) -> Result<f64, String> {
    total_energy(elements.iter().zip(inputs.iter()).map(|(element, input)| {
        (
            element.id.as_str(),
            element.strain_energy_density,
            element.area,
            input.thickness,
        )
    }))
}

fn total_energy<'a>(
    elements: impl ExactSizeIterator<Item = (&'a str, f64, f64, f64)>,
) -> Result<f64, String> {
    checkpoint(SolverStage::ResultTotals, 0)?;
    let count = elements.len();
    let mut total = 0.0;
    for (index, (id, density, area, thickness)) in elements.enumerate() {
        let mut factors = [density, area, thickness];
        factors.sort_unstable_by(f64::total_cmp);
        let energy = (factors[0] * factors[2]) * factors[1];
        if !energy.is_finite() || energy < 0.0 || (density != 0.0 && energy == 0.0) {
            return Err(format!(
                "plane element {id}: strain energy is not representable"
            ));
        }
        total += energy;
        if !total.is_finite() {
            return Err("plane total strain energy is not representable".into());
        }
        if (index + 1) % 64 == 0 {
            checkpoint(SolverStage::ResultTotals, index + 1)?;
        }
    }
    checkpoint(SolverStage::ResultTotals, count)?;
    Ok(total)
}

pub(super) fn max_triangle_strain_energy_density(elements: &[PlaneTriangleElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_quad_strain_energy_density(elements: &[PlaneQuadElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::total_energy;

    #[test]
    fn energy_product_avoids_representable_intermediate_range_loss() {
        for scale in [1e-200, 1e200] {
            let energy = total_energy([("bulk", scale, scale, 1.0 / scale)].into_iter()).unwrap();
            assert!((energy / scale - 1.0).abs() < 1e-14);
        }
    }

    #[test]
    fn unrepresentable_element_and_total_energy_are_errors() {
        for factors in [(1e-200, 1e-200, 1e-200), (1e200, 1e200, 1e200)] {
            let error =
                total_energy([("bulk", factors.0, factors.1, factors.2)].into_iter()).unwrap_err();
            assert!(error.contains("bulk") && error.contains("representable"));
        }
        let error =
            total_energy([("a", 1e308, 1.0, 1.0), ("b", 1e308, 1.0, 1.0)].into_iter()).unwrap_err();
        assert!(error.contains("total strain energy"));
    }
}
