use kyuubiki_protocol::{
    SolveSpring1dRequest, SolveSpring1dResult, Spring1dElementInput, Spring1dNodeInput,
};
use kyuubiki_solver::{solve_spring_1d, solve_spring_1d_owned};

fn model(free: usize, strong: f64, weak: f64) -> SolveSpring1dRequest {
    let nodes = (0..=free)
        .map(|index| Spring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: match index {
                0 | 3..=5 => 0.0,
                2 => weak,
                _ => strong,
            },
        })
        .collect();
    let mut elements: Vec<_> = (1..=free)
        .map(|index| Spring1dElementInput {
            id: format!("anchor{index}"),
            node_i: 0,
            node_j: index,
            stiffness: if (2..=5).contains(&index) {
                weak
            } else {
                strong
            },
        })
        .collect();
    for leaf in 3..=5 {
        elements.push(Spring1dElementInput {
            id: format!("branch{leaf}"),
            node_i: 2,
            node_j: leaf,
            stiffness: weak,
        });
    }
    SolveSpring1dRequest { nodes, elements }
}

#[test]
fn branched_spring_operator_preserves_weak_equilibrium_beside_strong_loads() {
    for free in [5, 1025] {
        for (strong, weak) in [(1e20, 1e-20), (1e100, 1e60), (1e-60, 1e-100)] {
            let input = model(free, strong, weak);
            check(&solve_spring_1d(&input).unwrap(), strong, weak);
            check(&solve_spring_1d_owned(input).unwrap(), strong, weak);
        }
    }
}

#[test]
fn extreme_mixed_scale_spring_operator_preserves_local_force_balance() {
    for (strong, weak) in [(1e300, 1e-300), (1e-300, 1e300)] {
        let input = model(1025, strong, weak);
        check(&solve_spring_1d(&input).unwrap(), strong, weak);
        check(&solve_spring_1d_owned(input).unwrap(), strong, weak);
    }
}

fn check(result: &SolveSpring1dResult, strong: f64, weak: f64) {
    let mut internal = vec![0.0; result.nodes.len()];
    for element in &result.elements {
        internal[element.node_i] -= element.force;
        internal[element.node_j] += element.force;
    }
    for node in &result.nodes {
        let original = node.id.strip_prefix('n').unwrap().parse::<usize>().unwrap();
        let expected = match original {
            0 => 0.0,
            2 => 0.4,
            3..=5 => 0.2,
            _ => 1.0,
        };
        assert!(
            (node.ux - expected).abs() < 1e-8,
            "{}: {}",
            node.id,
            node.ux
        );
        if original > 0 {
            let unit = if (2..=5).contains(&original) {
                weak
            } else {
                strong
            };
            let load = result.input.nodes[node.index].load_x / unit;
            assert!(
                (internal[node.index] / unit - load).abs() < 1e-7,
                "{} force balance",
                node.id
            );
        }
    }
}

#[test]
fn mixed_scale_spring_numbering_and_element_orientation_preserve_local_balance() {
    let mut input = model(1025, 1e20, 1e-20);
    let last = input.nodes.len() - 1;
    input.nodes.reverse();
    for element in &mut input.elements {
        (element.node_i, element.node_j) = (last - element.node_j, last - element.node_i);
    }
    check(&solve_spring_1d(&input).unwrap(), 1e20, 1e-20);
}

#[test]
fn exactly_representable_subnormal_spring_forces_remain_usable() {
    for stiffness in [f64::from_bits(1), f64::MIN_POSITIVE] {
        for displacement in [-2.0, 0.0, 2.0] {
            let load = stiffness * displacement;
            let input = SolveSpring1dRequest {
                nodes: vec![
                    Spring1dNodeInput {
                        id: "fixed".into(),
                        x: 0.0,
                        fix_x: true,
                        load_x: 0.0,
                    },
                    Spring1dNodeInput {
                        id: "loaded".into(),
                        x: 1.0,
                        fix_x: false,
                        load_x: load,
                    },
                ],
                elements: vec![Spring1dElementInput {
                    id: "spring".into(),
                    node_i: 0,
                    node_j: 1,
                    stiffness,
                }],
            };
            let borrowed = solve_spring_1d(&input).unwrap();
            let owned = solve_spring_1d_owned(input).unwrap();
            for result in [borrowed, owned] {
                assert_eq!(result.nodes[1].ux, displacement);
                assert_eq!(result.elements[0].force, load);
            }
        }
    }
}

#[test]
fn weak_spring_couplings_survive_scaled_products_and_candidate_range_rejection() {
    let strong = 2.0_f64.powi(1000);
    let weak = 2.0_f64.powi(-1000);
    for sign in [1.0, -1.0] {
        for reverse in [false, true] {
            let mut input = SolveSpring1dRequest {
                nodes: (0..=1025)
                    .map(|index| Spring1dNodeInput {
                        id: format!("n{index}"),
                        x: index as f64,
                        fix_x: index == 0,
                        load_x: sign
                            * match index {
                                1 => -3.0,
                                2..=4 => 1.0,
                                _ => 0.0,
                            },
                    })
                    .collect(),
                elements: (1..=1025)
                    .map(|index| Spring1dElementInput {
                        id: format!("spring{index}"),
                        node_i: if (2..=4).contains(&index) { 1 } else { 0 },
                        node_j: index,
                        stiffness: if (2..=4).contains(&index) {
                            weak
                        } else {
                            strong
                        },
                    })
                    .collect(),
            };
            if reverse {
                let last = input.nodes.len() - 1;
                input.nodes.reverse();
                for element in &mut input.elements {
                    (element.node_i, element.node_j) =
                        (last - element.node_j, last - element.node_i);
                }
            }
            let borrowed = solve_spring_1d(&input).unwrap();
            let owned = solve_spring_1d_owned(input).unwrap();
            for result in [borrowed, owned] {
                let mut internal = vec![0.0; result.nodes.len()];
                for element in &result.elements {
                    assert!(element.force.is_finite());
                    internal[element.node_i] -= element.force;
                    internal[element.node_j] += element.force;
                }
                for node in &result.nodes {
                    let id = node.id.strip_prefix('n').unwrap().parse::<usize>().unwrap();
                    if (2..=4).contains(&id) {
                        assert!((node.ux / strong - sign).abs() < 1e-12);
                    } else {
                        assert_eq!(node.ux, 0.0, "strong or unloaded node {}", node.id);
                    }
                    assert!(
                        (internal[node.index] - result.input.nodes[node.index].load_x).abs()
                            < 1e-12
                    );
                }
                assert!((result.total_strain_energy / strong - 1.5).abs() < 1e-12);
            }
        }
    }
}
