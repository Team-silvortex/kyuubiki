use super::*;

fn close(got: f64, want: f64, tolerance: f64) {
    assert!(
        got.is_finite() && (got - want).abs() <= tolerance,
        "{got} != {want}"
    );
}

#[test]
fn q9_nodal_basis_and_tensor_polynomial_derivatives() {
    for j in 0..3 {
        for i in 0..3 {
            let (n, _) = shape(i as f64 - 1.0, j as f64 - 1.0);
            for (k, value) in n.into_iter().enumerate() {
                close(value, f64::from(k == 3 * j + i), 1e-14);
            }
        }
    }
    for (x, y) in [(-0.71, 0.23), (0.33, -0.91), (0.0, 0.0)] {
        let (n, g) = shape(x, y);
        close(n.iter().sum(), 1.0, 1e-14);
        for d in g {
            close(d.iter().sum(), 0.0, 1e-14);
        }
        // All nine monomials, not only the bilayer's particular displacement.
        for a in 0_i32..=2 {
            for b in 0_i32..=2 {
                let samples: [f64; 9] = std::array::from_fn(|k| {
                    (k as f64 % 3.0 - 1.0).powi(a) * ((k / 3) as f64 - 1.0).powi(b)
                });
                let dot = |v: [f64; 9]| v.iter().zip(samples).map(|(a, b)| a * b).sum();
                close(dot(n), x.powi(a) * y.powi(b), 1e-14);
                close(
                    dot(g[0]),
                    if a == 0 {
                        0.0
                    } else {
                        a as f64 * x.powi(a - 1) * y.powi(b)
                    },
                    1e-14,
                );
                close(
                    dot(g[1]),
                    if b == 0 {
                        0.0
                    } else {
                        b as f64 * x.powi(a) * y.powi(b - 1)
                    },
                    1e-14,
                );
            }
        }
    }
}

#[test]
fn q9_solves_heat_and_non_affine_bilayer_without_exact_displacement_input() {
    for name in super::super::bending_model::NAMES {
        let report = run(Case::new(name, 1).unwrap(), 4, 2).unwrap();
        assert_eq!(report["passed"], true, "{name}: {report}");
        assert_eq!(report["heat_solved"], true);
        assert_eq!(report["production_operator"], false);
        assert_eq!(report["nodes"], 45);
        close(
            report["max_temperature_error_k"].as_f64().unwrap(),
            0.0,
            1e-9,
        );
        close(
            report["relative_displacement_error"].as_f64().unwrap(),
            0.0,
            1e-8,
        );
        close(report["energy_error_j"].as_f64().unwrap(), 0.0, 1e-8);
    }
}

#[test]
fn q9_rejects_unbounded_or_interface_crossing_meshes() {
    let c = Case::new("bilayer_gradient", 1).unwrap();
    for (nx, ny) in [(0, 2), (2, 0), (2, 3), (33, 2), (2, 18), (usize::MAX, 2)] {
        assert!(run(c, nx, ny).is_err());
    }
    let mut bad = c;
    bad.conductivity[0] = 0.0;
    assert!(run(bad, 2, 2).is_err());
    bad = c;
    bad.poisson[1] = 1.0;
    assert!(run(bad, 2, 2).is_err());
    bad = c;
    bad.young[0] = f64::NAN;
    assert!(run(bad, 2, 2).is_err());
    bad = c;
    bad.poisson[0] = -1.0;
    assert!(Mesh::new(bad, 2, 2).is_err());
}

#[test]
fn q9_full_integration_has_only_three_rigid_modes() {
    let c = Case::new("bilayer_gradient", 1).unwrap();
    let m = Mesh::new(c, 2, 2).unwrap();
    let ids = m.ids(0, 0);
    let (mut k, _) = element(c, m, 0, ids, &vec![0.0; m.nodes()]);
    let scale = k.iter().flatten().map(|v| v.abs()).fold(0.0_f64, f64::max);
    for mode in 0..3 {
        let u: [f64; 18] = std::array::from_fn(|i| {
            let [x, y] = m.xy(ids[i / 2]);
            match (mode, i % 2) {
                (0, 0) | (1, 1) => 1.0,
                (2, 0) => -y,
                (2, 1) => x,
                _ => 0.0,
            }
        });
        for row in k {
            close(
                row.iter().zip(u).map(|(a, b)| a * b).sum::<f64>() / scale,
                0.0,
                1e-14,
            );
        }
    }
    for i in 0..18 {
        for j in 0..18 {
            close((k[i][j] - k[j][i]) / scale, 0.0, 1e-14);
        }
    }
    // Independent row elimination counts the kernel dimension; no diagonal penalty.
    let mut rank = 0;
    for col in 0..18 {
        let pivot = (rank..18)
            .max_by(|a, b| k[*a][col].abs().total_cmp(&k[*b][col].abs()))
            .unwrap();
        if k[pivot][col].abs() <= scale * 1e-11 {
            continue;
        }
        k.swap(rank, pivot);
        for row in rank + 1..18 {
            let factor = k[row][col] / k[rank][col];
            for j in col..18 {
                k[row][j] -= factor * k[rank][j];
            }
        }
        rank += 1;
    }
    assert_eq!(rank, 15);
}

#[test]
fn q9_thermal_load_uses_midside_and_centre_nodes() {
    let c = Case::new("bilayer_gradient", 1).unwrap();
    let m = Mesh::new(c, 2, 2).unwrap();
    let ids = m.ids(0, 0);
    let zero = vec![0.0; m.nodes()];
    assert!(element(c, m, 0, ids, &zero).1.iter().all(|v| *v == 0.0));
    for node in [1, 3, 4, 5, 7] {
        let mut theta = zero.clone();
        theta[ids[node]] = 1.0;
        let f = element(c, m, 0, ids, &theta).1;
        assert!(f.iter().map(|v| v * v).sum::<f64>() > 1.0);
        close(f.iter().step_by(2).sum(), 0.0, 1e-10);
        close(f.iter().skip(1).step_by(2).sum(), 0.0, 1e-10);
    }
}

#[test]
fn q9_consistent_end_loads_match_virtual_work_through_quadratic_degree() {
    let c = Case::new("bilayer_gradient", 1).unwrap();
    let m = Mesh::new(c, 4, 2).unwrap();
    let f = loads(c, m);
    let [e, k] = c.axial();
    for power in 0_i32..=2 {
        let want: f64 = (0..2)
            .map(|layer| {
                let lo = layer as f64 * HEIGHT / 2.0;
                let hi = lo + HEIGHT / 2.0;
                let [a, b] = c.eigenstrain(layer);
                c.young[layer]
                    * THICKNESS
                    * ((e - a) * (hi.powi(power + 1) - lo.powi(power + 1)) / (power + 1) as f64
                        + (k - b) * (hi.powi(power + 2) - lo.powi(power + 2)) / (power + 2) as f64)
            })
            .sum();
        for (col, sign) in [(0, -1.0), (2 * m.nx, 1.0)] {
            let got: f64 = (0..=2 * m.ny)
                .map(|row| {
                    let node = row * (2 * m.nx + 1) + col;
                    f[2 * node] * m.xy(node)[1].powi(power)
                })
                .sum();
            close(got, sign * want, 1e-10);
        }
    }
    assert!(f.iter().skip(1).step_by(2).all(|v| *v == 0.0));
    assert!(
        f[2 * (2 * m.nx + 1)].abs() > 1.0,
        "midside traction cannot be omitted"
    );
}

#[test]
fn q9_audit_rejects_bad_temperature_displacement_and_rigid_motion() {
    let c = Case::new("bilayer_gradient", 1).unwrap();
    let m = Mesh::new(c, 4, 2).unwrap();
    let theta = heat(c, m).unwrap();
    let u = mechanical(c, m, &theta).unwrap();
    let mut bad_t = theta.clone();
    bad_t[m.nodes() / 2] += 0.01;
    assert_eq!(audit(c, m, &bad_t, &u).unwrap()["passed"], false);
    let mut bad_u = u.clone();
    bad_u[u.len() / 2] += 1e-6;
    assert_eq!(audit(c, m, &theta, &bad_u).unwrap()["passed"], false);
    // A rigid translation leaves strains unchanged, but violates prescribed DOFs.
    let shifted: Vec<_> = u.iter().map(|v| v + 1e-6).collect();
    assert_eq!(audit(c, m, &theta, &shifted).unwrap()["passed"], false);
    assert!(audit(c, m, &theta, &u[..u.len() - 1]).is_err());
    bad_t[0] = f64::INFINITY;
    assert!(audit(c, m, &bad_t, &u).is_err());
}
