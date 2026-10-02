use std::collections::BTreeMap;

// Independent unit-length Euler-Bernoulli assembly, shared only by regression tests.
// Do not use the production assembler or round-trip the published shape through sqrt(M).
pub fn unit_bending_residual(segments: usize, value: f64, shape: &[f64], space: bool) -> f64 {
    let stride = if space { 6 } else { 3 };
    assert_eq!(shape.len(), stride * (segments + 1));
    let size = 2 * segments;
    let mut rows = vec![BTreeMap::new(); size];
    let element = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for segment in 0..segments {
        for (i, row) in element.iter().enumerate() {
            for (j, coefficient) in row.iter().enumerate() {
                if 2 * segment + i >= 2 && 2 * segment + j >= 2 {
                    *rows[2 * segment + i - 2]
                        .entry(2 * segment + j - 2)
                        .or_insert(0.0) += coefficient;
                }
            }
        }
    }
    let (mut residual, mut applied, mut target) = (0.0_f64, 0.0_f64, 0.0_f64);
    for plane in 0..if space { 2 } else { 1 } {
        let vector: Vec<_> = shape
            .chunks_exact(stride)
            .skip(1)
            .flat_map(|v| {
                if plane == 0 {
                    [v[1], v[stride - 1]]
                } else {
                    [v[2], -v[4]]
                }
            })
            .collect();
        for (i, row) in rows.iter().enumerate() {
            let mut sum = 0.0_f64;
            let mut low = 0.0;
            for (&j, &a) in row {
                let product = a * vector[j];
                let next = sum + product;
                low += if sum.abs() >= product.abs() {
                    (sum - next) + product
                } else {
                    (product - next) + sum
                };
                low += a.mul_add(vector[j], -product);
                sum = next;
            }
            let mass: f64 =
                (if i % 2 == 0 { 1.0 } else { 1.0 / 12.0 }) * if i >= size - 2 { 0.5 } else { 1.0 };
            let force = (sum + low) / mass.sqrt();
            let inertia = value * mass.sqrt() * vector[i];
            residual = residual.hypot(force - inertia);
            applied = applied.hypot(force);
            target = target.hypot(inertia);
        }
    }
    residual / applied.max(target)
}
