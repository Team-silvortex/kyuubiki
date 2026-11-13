use kyuubiki_protocol::{
    SolveThermalQ9RectRequest, THERMAL_Q9_RECT_SCHEMA, ThermalQ9Element, ThermalQ9Node,
};
use kyuubiki_solver::solve_thermal_q9_rect;

fn reference(uniform: bool) -> ([f64; 2], [[f64; 2]; 2]) {
    let eigen = if uniform {
        [[1e-4, 0.0], [3e-4, 0.0]]
    } else {
        [[0.0, 0.00128], [0.00036, 0.00096]]
    };
    let (mut a, mut b, mut d, mut n, mut m) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, e) in [70e9, 20e9].into_iter().enumerate() {
        let lo = i as f64 * 0.125;
        let hi = lo + 0.125;
        let s = [
            hi - lo,
            (hi * hi - lo * lo) / 2.0,
            (hi.powi(3) - lo.powi(3)) / 3.0,
        ];
        a += e * s[0];
        b += e * s[1];
        d += e * s[2];
        n += e * (eigen[i][0] * s[0] + eigen[i][1] * s[1]);
        m += e * (eigen[i][0] * s[1] + eigen[i][1] * s[2]);
    }
    (
        [
            (n * d - m * b) / (a * d - b * b),
            (m * a - n * b) / (a * d - b * b),
        ],
        eigen,
    )
}

fn model(nx: usize, ny: usize, uniform: bool) -> SolveThermalQ9RectRequest {
    let mut nodes = vec![];
    let mut elements = vec![];
    let width = 2 * nx + 1;
    for j in 0..=2 * ny {
        for i in 0..=2 * nx {
            let y = 0.25 * j as f64 / (2 * ny) as f64;
            nodes.push(ThermalQ9Node {
                id: format!("n{i}_{j}"),
                x: i as f64 / (2 * nx) as f64,
                y,
                load_x: 0.0,
                load_y: 0.0,
                fix_x: i == 0 && j == 0,
                fix_y: j == 0 && (i == 0 || i == 2 * nx),
                temperature_delta: if uniform {
                    10.0
                } else {
                    128.0 * y.min(0.125) + 32.0 * (y - 0.125).max(0.0)
                },
            });
        }
    }
    let ([e0, k], eigen) = reference(uniform);
    for j in 0..ny {
        let layer = usize::from(j >= ny / 2);
        let young = [70e9, 20e9][layer];
        for i in 0..nx {
            elements.push(ThermalQ9Element {
                id: format!("e{i}_{j}"),
                nodes: std::array::from_fn(|s| (2 * j + s / 3) * width + 2 * i + s % 3),
                thickness: 0.01,
                youngs_modulus: young,
                poisson_ratio: [0.2, 0.35][layer],
                thermal_expansion: [10e-6, 30e-6][layer],
            });
        }
        let dy = 0.25 / ny as f64;
        let stress = |y: f64| young * (e0 + k * y - eigen[layer][0] - eigen[layer][1] * y);
        let (s0, s1) = (stress(j as f64 * dy), stress((j + 1) as f64 * dy));
        // Closed-form quadratic-edge integrals of linear traction, independent of kernel Gauss code.
        for (col, sign) in [(0, -1.0), (2 * nx, 1.0)] {
            for (offset, f) in [s0 / 6.0, (s0 + s1) / 3.0, s1 / 6.0]
                .into_iter()
                .enumerate()
            {
                nodes[(2 * j + offset) * width + col].load_x += sign * f * dy * 0.01;
            }
        }
    }
    SolveThermalQ9RectRequest {
        schema_version: THERMAL_Q9_RECT_SCHEMA.into(),
        nodes,
        elements,
    }
}

#[test]
fn q9_bilayer_quadratic_solution_survives_dense_and_sparse_native_backends() {
    for uniform in [false, true] {
        for (nx, ny) in [(2, 2), (8, 4), (16, 8)] {
            let request = model(nx, ny, uniform);
            let r = solve_thermal_q9_rect(&request).unwrap();
            let target = if uniform {
                0.12650602409638567
            } else {
                0.30305220883534134
            };
            println!(
                "uniform={uniform} mesh={nx}x{ny} energy_error_j={:.12e} iterations={} residual={:.12e}",
                (r.total_strain_energy_j - target).abs(),
                r.solver_iterations,
                r.solver_residual_norm
            );
            assert!(
                (r.total_strain_energy_j - target).abs() < 1e-8,
                "{nx}x{ny}: {}",
                r.total_strain_energy_j
            );
            let ([e0, k], eigen) = reference(uniform);
            for n in r.nodes {
                let want_x = (e0 + k * n.y) * n.x - k * n.y / 2.0;
                let mut want_y = k * n.x * (1.0 - n.x) / 2.0;
                for (i, nu) in [0.2, 0.35].into_iter().enumerate() {
                    let lo = i as f64 * 0.125;
                    let hi = n.y.min(lo + 0.125);
                    if hi > lo {
                        want_y += ((1.0 + nu) * eigen[i][0] - nu * e0) * (hi - lo)
                            + ((1.0 + nu) * eigen[i][1] - nu * k) * (hi * hi - lo * lo) / 2.0;
                    }
                }
                assert!((n.ux - want_x).abs() < 1e-10 && (n.uy - want_y).abs() < 1e-10);
            }
            assert!(r.solver_residual_norm.is_finite());
            if nx == 16 {
                assert!(
                    r.solver_iterations > 0,
                    "exercise sparse solve, not only element patch"
                );
            }
        }
    }
}
