use super::thermal_q9_validation::Rect;
use crate::plane_2d_math::{
    multiply_matrix_vector_3x3, plane_stress_d_matrix, strain_energy_density,
};
use kyuubiki_protocol::{SolveThermalQ9RectRequest, ThermalQ9Element, ThermalQ9ElementResult};

pub(super) struct Point {
    pub b: [[f64; 18]; 3],
    pub thermal: f64,
    pub weight: f64,
}

fn basis(x: f64) -> ([f64; 3], [f64; 3]) {
    (
        [x * (x - 1.0) / 2.0, 1.0 - x * x, x * (x + 1.0) / 2.0],
        [x - 0.5, -2.0 * x, x + 0.5],
    )
}

pub(super) fn points(
    r: &SolveThermalQ9RectRequest,
    e: &ThermalQ9Element,
    rect: Rect,
) -> Result<Vec<Point>, String> {
    let q = (3.0_f64 / 5.0).sqrt();
    let gauss = [(-q, 5.0 / 9.0), (0.0, 8.0 / 9.0), (q, 5.0 / 9.0)];
    let mut out = Vec::with_capacity(9);
    for (x, wx) in gauss {
        for (y, wy) in gauss {
            let (nx, gx) = basis(x);
            let (ny, gy) = basis(y);
            let mut b = [[0.0; 18]; 3];
            let mut thermal = 0.0;
            for i in 0..9 {
                let dx = gx[i % 3] * ny[i / 3] * 2.0 / rect.dx;
                let dy = nx[i % 3] * gy[i / 3] * 2.0 / rect.dy;
                b[0][2 * i] = dx;
                b[1][2 * i + 1] = dy;
                b[2][2 * i] = dy;
                b[2][2 * i + 1] = dx;
                thermal += nx[i % 3] * ny[i / 3] * r.nodes[e.nodes[i]].temperature_delta;
            }
            thermal *= e.thermal_expansion;
            let weight = wx * wy * rect.dx * rect.dy * e.thickness / 4.0;
            if !thermal.is_finite()
                || !weight.is_finite()
                || weight <= 0.0
                || b.iter().flatten().any(|v| !v.is_finite())
            {
                return Err("Q9 quadrature or thermal strain is not representable".into());
            }
            out.push(Point { b, thermal, weight });
        }
    }
    Ok(out)
}

pub(super) fn stiffness(
    e: &ThermalQ9Element,
    points: &[Point],
) -> Result<([[f64; 18]; 18], [f64; 18]), String> {
    let d = plane_stress_d_matrix(e.youngs_modulus, e.poisson_ratio);
    let (mut k, mut f) = ([[0.0; 18]; 18], [0.0; 18]);
    for p in points {
        let dt = multiply_matrix_vector_3x3(&d, &[p.thermal, p.thermal, 0.0]);
        let db: [[f64; 18]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|s| d[i][s] * p.b[s][j]).sum())
        });
        for i in 0..18 {
            f[i] += (0..3).map(|s| p.b[s][i] * dt[s]).sum::<f64>() * p.weight;
            for j in 0..18 {
                k[i][j] += (0..3).map(|s| p.b[s][i] * db[s][j]).sum::<f64>() * p.weight;
            }
        }
    }
    if d.iter()
        .flatten()
        .chain(k.iter().flatten())
        .chain(f.iter())
        .any(|v| !v.is_finite())
    {
        return Err("Q9 stiffness or equivalent load is not representable".into());
    }
    Ok((k, f))
}

pub(super) fn result(
    e: &ThermalQ9Element,
    rect: Rect,
    points: &[Point],
    u: &[f64],
) -> Result<ThermalQ9ElementResult, String> {
    let d = plane_stress_d_matrix(e.youngs_modulus, e.poisson_ratio);
    let (mut stress, mut strain, mut energy) = ([0.0; 3], [0.0; 3], 0.0);
    let volume = rect.dx * rect.dy * e.thickness;
    if !volume.is_finite() || volume <= 0.0 {
        return Err("Q9 volume is not representable".into());
    }
    for p in points {
        let elastic = std::array::from_fn(|s| {
            (0..18)
                .map(|i| p.b[s][i] * u[2 * e.nodes[i / 2] + i % 2])
                .sum::<f64>()
                - if s < 2 { p.thermal } else { 0.0 }
        });
        let sigma = multiply_matrix_vector_3x3(&d, &elastic);
        let density = strain_energy_density(&sigma, &elastic);
        if !density.is_finite() || density < 0.0 {
            return Err("Q9 strain energy density is not representable".into());
        }
        energy += density * p.weight;
        for s in 0..3 {
            stress[s] += sigma[s] * (p.weight / volume);
            strain[s] += elastic[s] * (p.weight / volume);
        }
    }
    if stress
        .iter()
        .chain(&strain)
        .chain(std::iter::once(&energy))
        .any(|v| !v.is_finite())
    {
        return Err("Q9 result is not representable".into());
    }
    Ok(ThermalQ9ElementResult {
        id: e.id.clone(),
        nodes: e.nodes,
        area: rect.dx * rect.dy,
        mean_stress: stress,
        mean_mechanical_strain: strain,
        strain_energy_j: energy,
    })
}
