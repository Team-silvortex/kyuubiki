#[derive(Debug, Clone)]
pub(super) struct ScalarTriangle {
    pub stiffness: [[f64; 3]; 3],
    pub area: f64,
    pub gradient_x: [f64; 3],
    pub gradient_y: [f64; 3],
}

#[derive(Debug, Clone)]
pub(super) struct ScalarQuad {
    pub first: ScalarTriangle,
    pub second: ScalarTriangle,
}

pub(super) fn triangle(
    [a, b, c]: [[f64; 2]; 3],
    thickness: f64,
    coefficient: f64,
    domain: &str,
) -> Result<ScalarTriangle, String> {
    let twice_area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    let area = 0.5 * twice_area.abs();
    if area <= 1.0e-12 {
        return Err(format!("{domain} triangle element area must be positive"));
    }
    if !area.is_finite() {
        return Err(format!("{domain} triangle element area must be finite"));
    }
    let gradient_x = [
        (b[1] - c[1]) / twice_area,
        (c[1] - a[1]) / twice_area,
        (a[1] - b[1]) / twice_area,
    ];
    let gradient_y = [
        (c[0] - b[0]) / twice_area,
        (a[0] - c[0]) / twice_area,
        (b[0] - a[0]) / twice_area,
    ];
    let scale = coefficient * thickness * area;
    let mut stiffness = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            let value = scale
                * (gradient_x[row] * gradient_x[column] + gradient_y[row] * gradient_y[column]);
            if !value.is_finite() {
                return Err(format!(
                    "{domain} triangle element stiffness must be finite"
                ));
            }
            stiffness[row][column] = value;
        }
    }
    Ok(ScalarTriangle {
        stiffness,
        area,
        gradient_x,
        gradient_y,
    })
}

pub(super) fn quad(
    points: [[f64; 2]; 4],
    thickness: f64,
    coefficient: f64,
    domain: &str,
) -> Result<ScalarQuad, String> {
    Ok(ScalarQuad {
        first: triangle(
            [points[0], points[1], points[2]],
            thickness,
            coefficient,
            domain,
        )?,
        second: triangle(
            [points[0], points[2], points[3]],
            thickness,
            coefficient,
            domain,
        )?,
    })
}

pub(super) fn scalar_gradient(
    gradient_x: &[f64; 3],
    gradient_y: &[f64; 3],
    values: &[f64; 3],
) -> [f64; 2] {
    // Partition of unity annihilates a constant field. Subtract it before
    // multiplication instead of cancelling large weighted absolute values.
    let first = values[1] - values[0];
    let second = values[2] - values[0];
    [
        gradient_x[1] * first + gradient_x[2] * second,
        gradient_y[1] * first + gradient_y[2] * second,
    ]
}

pub(super) fn shift_prescribed_reference(prescribed: &mut [(usize, f64)]) -> Result<f64, String> {
    // Solve K(u - reference) = f and retain that relative field through
    // postprocessing, so a large gauge cannot dominate the solver tolerance.
    crate::solver_control::check_cancellation()?;
    let reference = prescribed.first().map_or(0.0, |(_, value)| *value);
    for (index, (_, value)) in prescribed.iter_mut().enumerate() {
        if index % 256 == 0 {
            crate::solver_control::check_cancellation()?;
        }
        *value -= reference;
        if !value.is_finite() {
            return Err("scalar plane prescribed field range must be finite".into());
        }
    }
    Ok(reference)
}

pub(super) fn mean_squared_gradient(
    first: [f64; 2],
    second: [f64; 2],
    first_area: f64,
    second_area: f64,
) -> f64 {
    // Energy is quadratic: integrate each half before averaging. Squaring
    // the average gradient would erase opposing fields inside a split quad.
    let first_squared = first[0] * first[0] + first[1] * first[1];
    let second_squared = second[0] * second[0] + second[1] * second[1];
    let total_area = first_area + second_area;
    (first_area / total_area) * first_squared + (second_area / total_area) * second_squared
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver_control::{SolverControl, with_solver_control};

    #[test]
    fn tiny_subtriangle_retains_its_nonzero_energy_weight() {
        let mean = mean_squared_gradient([0.0, 0.0], [1.0e10, 0.0], 1.0e12, 1.0e-6);
        assert!((mean - 100.0).abs() < 1.0e-12);
        let reversed = mean_squared_gradient([1.0e10, 0.0], [0.0, 0.0], 1.0e-6, 1.0e12);
        assert_eq!(mean, reversed);
    }

    #[test]
    fn nonfinite_geometry_and_stiffness_are_rejected_before_assembly() {
        let huge = [[0.0, 0.0], [1.0e308, 0.0], [0.0, 1.0e308]];
        assert!(
            triangle(huge, 1.0, 1.0, "test")
                .unwrap_err()
                .contains("area must be finite")
        );
        let unit = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        assert!(
            triangle(unit, 1.0e308, 1.0e308, "test")
                .unwrap_err()
                .contains("stiffness must be finite")
        );
    }

    #[test]
    fn prescribed_field_range_overflow_is_rejected() {
        let mut prescribed = [(0, -f64::MAX), (1, f64::MAX)];
        assert!(
            shift_prescribed_reference(&mut prescribed)
                .unwrap_err()
                .contains("range must be finite")
        );
    }

    #[test]
    fn reference_shift_obeys_cancellation_and_fresh_replay() {
        let control = SolverControl::default();
        let mut prescribed = [(0, 123.0), (1, 125.0)];
        let result = with_solver_control(&control, || {
            control.request_cancel();
            shift_prescribed_reference(&mut prescribed)
        });
        assert!(result.unwrap_err().contains("cancelled"));
        assert_eq!(prescribed, [(0, 123.0), (1, 125.0)]);
        assert_eq!(shift_prescribed_reference(&mut prescribed).unwrap(), 123.0);
        assert_eq!(prescribed, [(0, 0.0), (1, 2.0)]);
    }
}
