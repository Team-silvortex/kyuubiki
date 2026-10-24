use super::{
    Complex, WideComplex, difference, divide, matrix_difference, numerator, row_update,
    wide_numerator,
};
use crate::solver_control::{SolverStage, check_cancellation};

// Bounded exponent headroom for residual and correction products. This selects
// compensation only, never pivot admission or whether the old range path runs.
const MIN_COMPONENT: f64 = f64::from_bits(923_u64 << 52);
const MAX_COMPONENT: f64 = f64::from_bits(1123_u64 << 52);

pub(crate) struct EliminationFactor {
    value: Complex,
    correction: Complex,
}

impl EliminationFactor {
    pub(crate) fn new(
        numerator: Complex,
        pivot: Complex,
        stage: SolverStage,
    ) -> Result<Self, String> {
        let value = divide(numerator, pivot, stage)?;
        let mut result = Self {
            value,
            correction: Complex::default(),
        };
        if ![numerator, pivot, value].into_iter().all(moderate) {
            return Ok(result);
        }
        let residual = if pivot.im == 0.0 {
            Complex {
                re: (-value.re).mul_add(pivot.re, numerator.re),
                im: (-value.im).mul_add(pivot.re, numerator.im),
            }
        } else if pivot.re == 0.0 {
            Complex {
                re: value.im.mul_add(pivot.im, numerator.re),
                im: (-value.re).mul_add(pivot.im, numerator.im),
            }
        } else {
            difference(numerator, value, pivot, stage)?
        };
        if residual.component_scale() != 0.0 {
            result.correction = divide(residual, pivot, stage)?;
        }
        Ok(result)
    }

    pub(crate) fn value(&self) -> Complex {
        self.value
    }

    pub(crate) fn restrict(
        &mut self,
        updates: impl Iterator<Item = (Complex, Complex)>,
        rhs: WideComplex,
        pivot_rhs: WideComplex,
    ) -> Result<(), String> {
        if self.correction.component_scale() == 0.0 {
            return Ok(());
        }
        if !matches!((rhs.finite(),pivot_rhs.finite()),(Some(a),Some(b)) if moderate(a) && moderate(b))
        {
            self.correction = Complex::default();
            return Ok(());
        }
        // Decide before row mutation so coefficients and RHS use one factor.
        for (index, (base, other)) in updates.enumerate() {
            if !moderate(base) || !moderate(other) {
                self.correction = Complex::default();
                return Ok(());
            }
            if (index + 1) % 1024 == 0 {
                check_cancellation()?;
            }
        }
        Ok(())
    }

    fn applies(&self, base: Complex, other: Complex) -> bool {
        self.correction.component_scale() != 0.0 && moderate(base) && moderate(other)
    }

    pub(crate) fn update_matrix(
        &self,
        base: Complex,
        other: Complex,
        stage: SolverStage,
    ) -> Result<Complex, String> {
        if !self.applies(base, other) {
            return matrix_difference(base, self.value, other, stage);
        }
        let terms = [(self.value, other), (self.correction, other)];
        if let Some(value) = row_update::numerator(base, terms.into_iter())? {
            return Ok(value);
        }
        let (real, imaginary) = numerator(base, terms.into_iter(), stage)?;
        Ok(Complex {
            re: real.restore_exact()?,
            im: imaginary.restore_exact()?,
        })
    }

    pub(crate) fn update_rhs(
        &self,
        base: WideComplex,
        other: WideComplex,
        stage: SolverStage,
    ) -> Result<WideComplex, String> {
        if self.correction.component_scale() == 0.0 {
            return base.subtract_product(self.value, other, stage);
        }
        if let (Some(finite), Some(value)) = (base.finite(), other.finite()) {
            if self.applies(finite, value) {
                let terms = [(self.value, value), (self.correction, value)];
                if let Some(value) = row_update::numerator(finite, terms.into_iter())? {
                    return Ok(value.into());
                }
                let (real, imaginary) = wide_numerator(
                    base,
                    [(self.value, other), (self.correction, other)].into_iter(),
                    stage,
                )?;
                return Ok(WideComplex { real, imaginary });
            }
        }
        base.subtract_product(self.value, other, stage)
    }
}

fn moderate(value: Complex) -> bool {
    [value.re, value.im]
        .into_iter()
        .all(|x| x == 0.0 || (x.abs() >= MIN_COMPONENT && x.abs() <= MAX_COMPONENT))
}

#[cfg(test)]
#[path = "harmonic_spring_factor_tests.rs"]
mod tests;
