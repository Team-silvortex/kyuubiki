use super::{Complex, element_coefficient, inertia_coefficient};
use crate::{
    linear_binary_scale::{restore, split_finite},
    solver_control::{SolverStage, checkpoint, checkpoint_chunk},
};
use kyuubiki_protocol::SolveHarmonicSpring1dRequest;

pub(super) fn validate_harmonic_equilibrium(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    displacement: &[Complex],
    reduced_index: &[Option<usize>],
) -> Result<(), String> {
    let free_count = validate_state(request, displacement, reduced_index)?;
    checkpoint(SolverStage::HarmonicResidual, 0)?;
    let mut rows = (0..free_count)
        .map(|_| Equation::default())
        .collect::<Vec<_>>();
    for (index, node) in request.nodes.iter().enumerate() {
        if let Some(row) = reduced_index[index] {
            rows[row].observe(&[Product::new(node.load_x, 1.0)?]);
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.nodes.len(),
        )?;
    }
    visit_terms(
        request,
        omega,
        displacement,
        reduced_index,
        |row, products| {
            rows[row].observe(&products);
            Ok(())
        },
    )?;
    for (index, node) in request.nodes.iter().enumerate() {
        if let Some(row) = reduced_index[index] {
            let equation = &mut rows[row];
            if equation.exponent == i32::MIN {
                equation.exponent = 0;
            }
            let (load, tail) = Product::new(node.load_x, 1.0)?.normalized(equation.exponent)?;
            equation.real.add(load);
            equation.real.add(tail);
            equation.scale.add(load.abs());
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.nodes.len(),
        )?;
    }
    visit_terms(
        request,
        omega,
        displacement,
        reduced_index,
        |row, products| {
            rows[row]
                .subtract(products)
                .map_err(|error| format!("harmonic original-system free row {row}: {error}"))
        },
    )?;
    let tolerance = (128.0 * f64::EPSILON * free_count as f64).clamp(1.0e-12, 1.0e-8);
    for (row, equation) in rows.into_iter().enumerate() {
        let residual = equation.real.value().hypot(equation.imaginary.value());
        let scale = equation.scale.value();
        let relative = if scale == 0.0 { 0.0 } else { residual / scale };
        if !residual.is_finite()
            || !scale.is_finite()
            || !relative.is_finite()
            || relative > tolerance
        {
            return Err(format!(
                "harmonic spring 1d dynamic stiffness failed backward-error validation at free row {row} ({relative:.6e})"
            ));
        }
        checkpoint_chunk(SolverStage::HarmonicResidual, row + 1, free_count)?;
    }
    checkpoint(
        SolverStage::ConstraintValidate,
        request.nodes.len().saturating_mul(2),
    )?;
    Ok(())
}

fn validate_state(
    request: &SolveHarmonicSpring1dRequest,
    displacement: &[Complex],
    reduced_index: &[Option<usize>],
) -> Result<usize, String> {
    checkpoint(SolverStage::ConstraintValidate, 0)?;
    let size = request.nodes.len();
    if displacement.len() != size || reduced_index.len() != size {
        return Err("harmonic original-system dimensions must match the complete model".into());
    }
    let mut free = 0;
    for (index, node) in request.nodes.iter().enumerate() {
        let value = displacement[index];
        if !value.is_finite() || !node.load_x.is_finite() {
            return Err(format!(
                "harmonic original-system non-finite state at node {index}"
            ));
        }
        if node.fix_x && (value.re != 0.0 || value.im != 0.0) {
            return Err(format!(
                "harmonic original-system support value changed at node {index}"
            ));
        }
        let expected = if node.fix_x { None } else { Some(free) };
        if reduced_index[index] != expected {
            return Err(format!(
                "harmonic original-system reduction map disagrees at node {index}"
            ));
        }
        free += usize::from(!node.fix_x);
        checkpoint_chunk(SolverStage::ConstraintValidate, index + 1, size)?;
    }
    Ok(free)
}

fn visit_terms(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    displacement: &[Complex],
    reduced_index: &[Option<usize>],
    mut visit: impl FnMut(usize, [Product; 4]) -> Result<(), String>,
) -> Result<(), String> {
    for (index, node) in request.nodes.iter().enumerate() {
        if let Some(row) = reduced_index[index] {
            visit(
                row,
                products(
                    Complex::real(-inertia_coefficient(omega, node.mass, index)?),
                    displacement[index],
                )?,
            )?;
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.nodes.len(),
        )?;
    }
    for (index, element) in request.elements.iter().enumerate() {
        let coefficient = element_coefficient(element, omega, index)?;
        for (node, other) in [
            (element.node_i, element.node_j),
            (element.node_j, element.node_i),
        ] {
            if let Some(row) = reduced_index[node] {
                visit(row, products(coefficient, displacement[node])?)?;
                visit(
                    row,
                    products(
                        Complex {
                            re: -coefficient.re,
                            im: -coefficient.im,
                        },
                        displacement[other],
                    )?,
                )?;
            }
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.elements.len(),
        )?;
    }
    Ok(())
}

// Keep the original element terms, rather than summing a dynamic diagonal first.
// Choose the binary scale from actual products, never unrelated coefficient/unknown maxima.
fn products(coefficient: Complex, value: Complex) -> Result<[Product; 4], String> {
    Ok([
        Product::new(coefficient.re, value.re)?,
        Product::new(coefficient.im, value.im)?,
        Product::new(coefficient.re, value.im)?,
        Product::new(coefficient.im, value.re)?,
    ])
}

struct Product {
    high: f64,
    low: f64,
    exponent: i32,
}

impl Product {
    fn new(coefficient: f64, value: f64) -> Result<Self, String> {
        if !coefficient.is_finite() || !value.is_finite() {
            return Err("harmonic original-system contains a non-finite operand".into());
        }
        let (coefficient, ce) = split_finite(coefficient);
        let (value, ve) = split_finite(value);
        let high = coefficient * value;
        Ok(Self {
            high,
            low: coefficient.mul_add(value, -high),
            exponent: ce + ve,
        })
    }

    fn normalized(&self, exponent: i32) -> Result<(f64, f64), String> {
        let shift = self.exponent - exponent;
        let normalize = |value| -> Result<f64, String> {
            let scaled = restore(value, shift);
            if !scaled.is_finite() || restore(scaled, -shift) != value {
                Err(
                    "harmonic original-system residual normalization would lose numerical range"
                        .into(),
                )
            } else {
                Ok(scaled)
            }
        };
        Ok((normalize(self.high)?, normalize(self.low)?))
    }
}

struct Equation {
    exponent: i32,
    real: CompensatedSum,
    imaginary: CompensatedSum,
    scale: CompensatedSum,
}

impl Default for Equation {
    fn default() -> Self {
        Self {
            exponent: i32::MIN,
            real: CompensatedSum::default(),
            imaginary: CompensatedSum::default(),
            scale: CompensatedSum::default(),
        }
    }
}

impl Equation {
    fn observe(&mut self, products: &[Product]) {
        for product in products {
            if product.high != 0.0 {
                self.exponent = self.exponent.max(product.exponent);
            }
        }
    }

    fn subtract(&mut self, products: [Product; 4]) -> Result<(), String> {
        let mut norm = 0.0_f64;
        for (index, product) in products.iter().enumerate() {
            let (high, low) = product.normalized(self.exponent)?;
            norm = norm.hypot(high + low);
            let sum = if index < 2 {
                &mut self.real
            } else {
                &mut self.imaginary
            };
            let sign = if index == 1 { 1.0 } else { -1.0 };
            sum.add(sign * high);
            sum.add(sign * low);
        }
        self.scale.add(norm);
        Ok(())
    }
}

#[derive(Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - next) + value
        } else {
            (value - next) + self.sum
        };
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum + self.correction
    }
}
