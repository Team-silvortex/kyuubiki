use kyuubiki_protocol::{
    SolveThermalQ9RectRequest, THERMAL_Q9_RECT_SCHEMA, ThermalQ9Element, ThermalQ9Node,
};
use kyuubiki_solver::solve_thermal_q9_rect;

fn patch() -> SolveThermalQ9RectRequest {
    SolveThermalQ9RectRequest {
        schema_version: THERMAL_Q9_RECT_SCHEMA.into(),
        nodes: (0..9)
            .map(|i| ThermalQ9Node {
                id: format!("n{i}"),
                x: (i % 3) as f64 / 2.0,
                y: (i / 3) as f64 / 2.0,
                load_x: 0.0,
                load_y: 0.0,
                fix_x: i == 0,
                fix_y: i == 0 || i == 2,
                temperature_delta: 10.0,
            })
            .collect(),
        elements: vec![ThermalQ9Element {
            id: "e0".into(),
            nodes: [0, 1, 2, 3, 4, 5, 6, 7, 8],
            thickness: 0.01,
            youngs_modulus: 70e9,
            poisson_ratio: 0.3,
            thermal_expansion: 12e-6,
        }],
    }
}

#[test]
fn q9_affine_free_expansion_uses_native_solver() {
    let m = patch();
    let r = solve_thermal_q9_rect(&m).unwrap();
    for n in r.nodes {
        assert!((n.ux - 120e-6 * n.x).abs() < 1e-12);
        assert!((n.uy - 120e-6 * n.y).abs() < 1e-12);
    }
    assert!(r.total_strain_energy_j < 1e-18);
    assert!(r.solver_matrix_non_zero_count > 0);
}

#[test]
fn q9_rejects_bad_geometry_materials_identity_and_constraints() {
    let m = patch();
    let mut bad = m.clone();
    bad.schema_version = "unknown".into();
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.nodes[4].x += 0.1;
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.elements[0].nodes[4] = 0;
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.elements[0].nodes[4] = usize::MAX;
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.elements[0].poisson_ratio = -1.0;
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.nodes[1].temperature_delta = f64::NAN;
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.nodes[1].id = "n0".into();
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.elements.push(m.elements[0].clone());
    assert!(solve_thermal_q9_rect(&bad).is_err());
    let mut bad = m.clone();
    bad.nodes[2].fix_y = false;
    for n in &mut bad.nodes {
        n.temperature_delta = 0.0;
    }
    assert!(
        solve_thermal_q9_rect(&bad).is_err(),
        "zero RHS must not hide rotation mechanism"
    );
    let mut value = serde_json::to_value(m).unwrap();
    value["nodes"][0]["temperature"] = 30.0.into();
    assert!(serde_json::from_value::<SolveThermalQ9RectRequest>(value).is_err());
}

#[test]
fn q9_cancellation_discards_result_and_next_call_recovers() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    let control = SolverControl::default();
    let cancel = control.clone();
    let m = patch();
    let err = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ElementAssembly {
                cancel.request_cancel();
            }
        },
        || solve_thermal_q9_rect(&m),
    )
    .unwrap_err();
    assert!(err.contains("cancel"));
    assert!(control.was_interrupted());
    assert!(solve_thermal_q9_rect(&m).is_ok());
}

#[test]
fn q9_midside_temperature_loads_and_budget_are_not_silently_ignored() {
    let m = patch();
    for i in [1, 3, 4, 5, 7] {
        let mut changed = m.clone();
        changed.nodes[i].temperature_delta += 1.0;
        assert!(
            solve_thermal_q9_rect(&changed)
                .unwrap()
                .total_strain_energy_j
                > 1e-8
        );
    }
    let mut large = m.clone();
    large.nodes.resize(4097, m.nodes[0].clone());
    assert!(
        solve_thermal_q9_rect(&large)
            .unwrap_err()
            .contains("budget")
    );
    let mut bad = m;
    bad.elements[0].youngs_modulus = f64::MAX;
    assert!(solve_thermal_q9_rect(&bad).is_err());
}

#[test]
fn q9_shared_edges_use_original_coordinates_not_rounded_reconstructed_bounds() {
    let mut m = patch();
    let template = m.nodes[0].clone();
    m.nodes = (0..15)
        .map(|i| {
            let mut n = template.clone();
            n.id = format!("n{i}");
            n.x = [-0.3, -0.1, 0.1, 0.3, 0.5][i % 5];
            n.y = (i / 5) as f64 / 2.0;
            n.fix_x = i == 0;
            n.fix_y = i == 0 || i == 4;
            n
        })
        .collect();
    let e = m.elements[0].clone();
    m.elements = (0..2)
        .map(|i| {
            let mut e = e.clone();
            e.id = format!("e{i}");
            e.nodes = std::array::from_fn(|j| 2 * i + (j / 3) * 5 + j % 3);
            e
        })
        .collect();
    let result = solve_thermal_q9_rect(&m).unwrap();
    for n in result.nodes {
        assert!((n.ux - 120e-6 * (n.x + 0.3)).abs() < 1e-12);
        assert!((n.uy - 120e-6 * n.y).abs() < 1e-12);
    }
}

#[test]
fn q9_rejects_overlapping_disconnected_and_hanging_meshes() {
    for (offset, message) in [(0.25, "overlapping"), (2.0, "edge-connected")] {
        let mut m = patch();
        let other = patch();
        m.nodes.extend(other.nodes.into_iter().map(|mut n| {
            n.x += offset;
            n.id.push('b');
            n
        }));
        let mut e = other.elements[0].clone();
        e.id = "e1".into();
        e.nodes = e.nodes.map(|i| i + 9);
        m.elements.push(e);
        assert!(solve_thermal_q9_rect(&m).unwrap_err().contains(message));
    }
    let mut m = patch();
    m.nodes.clear();
    m.elements.clear();
    for (index, (x, y, dx, dy)) in [
        (0.0, 0.0, 1.0, 1.0),
        (1.0, 0.0, 1.0, 0.5),
        (1.0, 0.5, 1.0, 0.5),
    ]
    .into_iter()
    .enumerate()
    {
        let mut ids = [0; 9];
        for (i, id) in ids.iter_mut().enumerate() {
            let (x, y) = (x + (i % 3) as f64 * dx / 2.0, y + (i / 3) as f64 * dy / 2.0);
            *id = if let Some(j) = m.nodes.iter().position(|n| n.x == x && n.y == y) {
                j
            } else {
                let j = m.nodes.len();
                m.nodes.push(ThermalQ9Node {
                    id: format!("n{j}"),
                    x,
                    y,
                    fix_x: x == 0.0 && y == 0.0,
                    fix_y: y == 0.0,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                });
                j
            };
        }
        let mut e = patch().elements[0].clone();
        e.id = format!("e{index}");
        e.nodes = ids;
        m.elements.push(e);
    }
    assert!(solve_thermal_q9_rect(&m).unwrap_err().contains("hanging"));
}
