//! Bounded rectangular Q9 comparison, not a registered production operator.
use super::bending_direct::Band;
use super::bending_model::{Case, HEIGHT, THICKNESS};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct Mesh {
    nx: usize,
    ny: usize,
}

impl Mesh {
    fn new(c: Case, nx: usize, ny: usize) -> Result<Self, String> {
        if !(1..=32).contains(&nx) || !(2..=16).contains(&ny) || ny % 2 != 0 {
            return Err(
                "Q9 reference requires nx=1..32, even ny=2..16 with aligned interface".into(),
            );
        }
        let values = [c.reference, c.bottom, c.top, c.alpha[0], c.alpha[1]];
        if values.iter().any(|v| !v.is_finite())
            || c.young
                .iter()
                .chain(&c.conductivity)
                .any(|v| !v.is_finite() || *v <= 0.0)
            || c.poisson
                .iter()
                .any(|v| !v.is_finite() || *v <= -1.0 || *v >= 0.5)
        {
            return Err("invalid Q9 reference material or temperature".into());
        }
        Ok(Self { nx, ny })
    }
    fn nodes(self) -> usize {
        (2 * self.nx + 1) * (2 * self.ny + 1)
    }
    fn xy(self, i: usize) -> [f64; 2] {
        [
            (i % (2 * self.nx + 1)) as f64 / (2 * self.nx) as f64,
            (i / (2 * self.nx + 1)) as f64 * HEIGHT / (2 * self.ny) as f64,
        ]
    }
    fn ids(self, row: usize, col: usize) -> [usize; 9] {
        std::array::from_fn(|i| (2 * row + i / 3) * (2 * self.nx + 1) + 2 * col + i % 3)
    }
    fn fixed(self) -> [usize; 3] {
        [0, 1, 4 * self.nx + 1]
    }
}

fn lagrange(x: f64) -> ([f64; 3], [f64; 3]) {
    (
        [0.5 * x * (x - 1.0), 1.0 - x * x, 0.5 * x * (x + 1.0)],
        [x - 0.5, -2.0 * x, x + 0.5],
    )
}

// Row-major tensor grid: bottom row first, including midsides and centre.
fn shape(xi: f64, eta: f64) -> ([f64; 9], [[f64; 9]; 2]) {
    let (x, gx) = lagrange(xi);
    let (y, gy) = lagrange(eta);
    (
        std::array::from_fn(|i| x[i % 3] * y[i / 3]),
        [
            std::array::from_fn(|i| gx[i % 3] * y[i / 3]),
            std::array::from_fn(|i| x[i % 3] * gy[i / 3]),
        ],
    )
}

fn gauss() -> [(f64, f64); 3] {
    let a = (3.0_f64 / 5.0).sqrt();
    [(-a, 5.0 / 9.0), (0.0, 8.0 / 9.0), (a, 5.0 / 9.0)]
}

struct Point {
    n: [f64; 9],
    g: [[f64; 9]; 2],
    b: [[f64; 18]; 3],
    y: f64,
    w: f64,
}

fn points(m: Mesh, row: usize, mut visit: impl FnMut(Point)) {
    let (dx, dy) = (1.0 / m.nx as f64, HEIGHT / m.ny as f64);
    for (xi, wx) in gauss() {
        for (eta, wy) in gauss() {
            let (n, mut g) = shape(xi, eta);
            for i in 0..9 {
                g[0][i] *= 2.0 / dx;
                g[1][i] *= 2.0 / dy;
            }
            let mut b = [[0.0; 18]; 3];
            for i in 0..9 {
                b[0][2 * i] = g[0][i];
                b[1][2 * i + 1] = g[1][i];
                b[2][2 * i] = g[1][i];
                b[2][2 * i + 1] = g[0][i];
            }
            visit(Point {
                n,
                g,
                b,
                y: (row as f64 + (eta + 1.0) / 2.0) * dy,
                w: wx * wy * dx * dy * THICKNESS / 4.0,
            });
        }
    }
}

fn material(c: Case, layer: usize) -> [[f64; 3]; 3] {
    let (e, nu) = (c.young[layer], c.poisson[layer]);
    let d = e / (1.0 - nu * nu);
    [
        [d, nu * d, 0.0],
        [nu * d, d, 0.0],
        [0.0, 0.0, e / (2.0 * (1.0 + nu))],
    ]
}

fn apply(d: [[f64; 3]; 3], x: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|j| d[i][j] * x[j]).sum())
}

fn assemble<const N: usize>(
    a: &mut Band,
    rhs: &mut [f64],
    fixed: &[Option<f64>],
    ids: [usize; N],
    k: [[f64; N]; N],
    f: [f64; N],
) {
    for i in 0..N {
        if fixed[ids[i]].is_some() {
            continue;
        }
        rhs[ids[i]] += f[i];
        for j in 0..N {
            if let Some(value) = fixed[ids[j]] {
                rhs[ids[i]] -= k[i][j] * value;
            } else if ids[i] >= ids[j] {
                a.add(ids[i], ids[j], k[i][j]);
            }
        }
    }
}

fn finish(mut a: Band, mut rhs: Vec<f64>, fixed: &[Option<f64>]) -> Result<Vec<f64>, String> {
    for (i, value) in fixed.iter().enumerate() {
        if let Some(value) = value {
            a.add(i, i, 1.0);
            rhs[i] = *value;
        }
    }
    a.solve(rhs)
}

fn heat(c: Case, m: Mesh) -> Result<Vec<f64>, String> {
    let mut a = Band::new(m.nodes(), 4 * m.nx + 4);
    let mut rhs = vec![0.0; m.nodes()];
    let fixed: Vec<_> = (0..m.nodes())
        .map(|i| {
            let row = i / (2 * m.nx + 1);
            if row == 0 {
                Some(c.bottom - c.reference)
            } else if row == 2 * m.ny {
                Some(c.top - c.reference)
            } else {
                None
            }
        })
        .collect();
    for row in 0..m.ny {
        let conductivity = c.conductivity[usize::from(row >= m.ny / 2)];
        let mut k = [[0.0; 9]; 9];
        points(m, row, |p| {
            for (i, line) in k.iter_mut().enumerate() {
                for (j, value) in line.iter_mut().enumerate() {
                    *value += conductivity * (p.g[0][i] * p.g[0][j] + p.g[1][i] * p.g[1][j]) * p.w;
                }
            }
        });
        for col in 0..m.nx {
            assemble(&mut a, &mut rhs, &fixed, m.ids(row, col), k, [0.0; 9]);
        }
    }
    finish(a, rhs, &fixed)
}

fn loads(c: Case, m: Mesh) -> Vec<f64> {
    let mut f = vec![0.0; 2 * m.nodes()];
    if c.name.starts_with("gradient_free") {
        return f;
    }
    let dy = HEIGHT / m.ny as f64;
    for row in 0..m.ny {
        let layer = usize::from(row >= m.ny / 2);
        for (eta, w) in gauss() {
            let (n, _) = lagrange(eta);
            let traction =
                c.stress((row as f64 + (eta + 1.0) / 2.0) * dy, layer) * dy * THICKNESS * w / 2.0;
            for (col, sign) in [(0, -1.0), (2 * m.nx, 1.0)] {
                for (j, value) in n.into_iter().enumerate() {
                    f[2 * ((2 * row + j) * (2 * m.nx + 1) + col)] += sign * value * traction;
                }
            }
        }
    }
    f
}

fn element(
    c: Case,
    m: Mesh,
    row: usize,
    ids: [usize; 9],
    theta: &[f64],
) -> ([[f64; 18]; 18], [f64; 18]) {
    let layer = usize::from(row >= m.ny / 2);
    let d = material(c, layer);
    let (mut k, mut f) = ([[0.0; 18]; 18], [0.0; 18]);
    points(m, row, |p| {
        let t = c.alpha[layer] * (0..9).map(|i| p.n[i] * theta[ids[i]]).sum::<f64>();
        let thermal = apply(d, [t, t, 0.0]);
        let db: [[f64; 18]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|h| d[i][h] * p.b[h][j]).sum())
        });
        for i in 0..18 {
            f[i] += (0..3).map(|h| p.b[h][i] * thermal[h]).sum::<f64>() * p.w;
            for j in 0..18 {
                k[i][j] += (0..3).map(|h| p.b[h][i] * db[h][j]).sum::<f64>() * p.w;
            }
        }
    });
    (k, f)
}

fn mechanical(c: Case, m: Mesh, theta: &[f64]) -> Result<Vec<f64>, String> {
    let n = 2 * m.nodes();
    let mut a = Band::new(n, 8 * m.nx + 9);
    let mut rhs = loads(c, m);
    let mut fixed = vec![None; n];
    for i in m.fixed() {
        fixed[i] = Some(0.0);
    }
    for row in 0..m.ny {
        for col in 0..m.nx {
            let ids = m.ids(row, col);
            let (k, f) = element(c, m, row, ids, theta);
            let dofs = std::array::from_fn(|i| 2 * ids[i / 2] + i % 2);
            assemble(&mut a, &mut rhs, &fixed, dofs, k, f);
        }
    }
    finish(a, rhs, &fixed)
}

fn audit(c: Case, m: Mesh, theta: &[f64], u: &[f64]) -> Result<Value, String> {
    if theta.len() != m.nodes()
        || u.len() != 2 * m.nodes()
        || theta.iter().chain(u).any(|v| !v.is_finite())
    {
        return Err("invalid Q9 audit fields".into());
    }
    let (mut te, mut du, mut norm) = (0.0_f64, 0.0, 0.0);
    for i in 0..m.nodes() {
        let [x, y] = m.xy(i);
        te = te.max((theta[i] - (c.temperature(y) - c.reference)).abs());
        for (axis, want) in c.displacement(x, y).into_iter().enumerate() {
            du += (u[2 * i + axis] - want).powi(2);
            norm += want * want;
        }
    }
    let (mut energy, mut error, mut thermal_pair, mut shear) = (0.0, 0.0, 0.0, 0.0);
    let mut heat_force = vec![0.0; m.nodes()];
    let mut internal = vec![0.0; 2 * m.nodes()];
    for row in 0..m.ny {
        let layer = usize::from(row >= m.ny / 2);
        let d = material(c, layer);
        for col in 0..m.nx {
            let ids = m.ids(row, col);
            points(m, row, |p| {
                let t = c.alpha[layer] * (0..9).map(|i| p.n[i] * theta[ids[i]]).sum::<f64>();
                let strain: [f64; 3] = std::array::from_fn(|a| {
                    (0..18).map(|j| p.b[a][j] * u[2 * ids[j / 2] + j % 2]).sum()
                });
                let elastic = [strain[0] - t, strain[1] - t, strain[2]];
                let stress = apply(d, elastic);
                let exact = c.strain(p.y, layer);
                let delta = std::array::from_fn(|i| strain[i] - exact[i]);
                let ds = apply(d, delta);
                energy += (0..3).map(|i| elastic[i] * stress[i]).sum::<f64>() * p.w / 2.0;
                error += (0..3).map(|i| delta[i] * ds[i]).sum::<f64>() * p.w / 2.0;
                shear += delta[2] * ds[2] * p.w / 2.0;
                thermal_pair += t * (stress[0] + stress[1]) * p.w;
                let grad: [f64; 2] =
                    std::array::from_fn(|a| (0..9).map(|j| p.g[a][j] * theta[ids[j]]).sum());
                for i in 0..9 {
                    heat_force[ids[i]] +=
                        c.conductivity[layer] * (p.g[0][i] * grad[0] + p.g[1][i] * grad[1]) * p.w;
                    for axis in 0..2 {
                        internal[2 * ids[i] + axis] += (0..3)
                            .map(|j| p.b[j][2 * i + axis] * stress[j])
                            .sum::<f64>()
                            * p.w;
                    }
                }
            });
        }
    }
    let force = loads(c, m);
    let mut residual = 0.0_f64;
    let mut reaction = 0.0_f64;
    for i in 0..u.len() {
        let r = (internal[i] - force[i]).abs();
        if m.fixed().contains(&i) {
            reaction = reaction.max(r);
        } else {
            residual = residual.max(r);
        }
    }
    let hf = heat_force
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let row = i / (2 * m.nx + 1);
            row > 0 && row < 2 * m.ny
        })
        .map(|(_, v)| v.abs())
        .fold(0.0_f64, f64::max);
    let width = 2 * m.nx + 1;
    let bottom: f64 = heat_force[..width].iter().sum();
    let top: f64 = heat_force[heat_force.len() - width..].iter().sum();
    let flux_gap = (bottom - c.flux() * THICKNESS)
        .abs()
        .max((top + c.flux() * THICKNESS).abs());
    let pairing: f64 = force.iter().zip(u).map(|(f, u)| f * u).sum();
    let balance = (2.0 * energy - pairing + thermal_pair).abs();
    let relative = (du / norm.max(1e-30)).sqrt();
    let energy_gap = (energy - c.energy()).abs();
    let metrics = [
        te, relative, energy_gap, error, residual, reaction, hf, flux_gap, balance,
    ];
    let limits = [1e-8, 1e-8, 1e-8, 1e-14, 1e-4, 1e-4, 1e-8, 1e-8, 1e-8];
    let passed = metrics
        .iter()
        .zip(limits)
        .all(|(v, limit)| v.is_finite() && *v <= limit);
    Ok(
        json!({"passed":passed,"production_operator":false,"heat_solved":true,
        "scope":"local_rectangular_q9_banded_reference_not_agent_execution",
        "mesh":[m.nx,m.ny],"nodes":m.nodes(),"mechanical_free_dofs":u.len()-3,
        "heat_free_dofs":m.nodes()-2*width,"quadrature":"3x3_full_integration",
        "max_temperature_error_k":te,"relative_displacement_error":relative,
        "energy_j":energy,"reference_energy_j":c.energy(),"energy_error_j":energy_gap,
        "ordinary_relative_energy_error":if c.energy()>1e-12 {Some(energy_gap/c.energy())}else{None},
        "strain_error_energy_j":error,"shear_error_energy_j":shear,
        "max_free_force_residual_n":residual,"max_reaction_n":reaction,
        "max_free_heat_residual_w":hf,"boundary_heat_rate_error_w":flux_gap,
        "constitutive_identity_error_j":balance,
        "limits":{"temperature_k":limits[0],"relative_displacement":limits[1],"energy_j":limits[2],
            "strain_error_energy_j":limits[3],"force_residual_n":limits[4],"reaction_n":limits[5],
            "heat_residual_w":limits[6],"heat_rate_w":limits[7],"constitutive_identity_j":limits[8]}}),
    )
}

pub fn run(c: Case, nx: usize, ny: usize) -> Result<Value, String> {
    let m = Mesh::new(c, nx, ny)?;
    let theta = heat(c, m)?;
    let u = mechanical(c, m, &theta)?;
    audit(c, m, &theta, &u)
}

#[cfg(test)]
#[path = "bending_q9_tests.rs"]
mod tests;
