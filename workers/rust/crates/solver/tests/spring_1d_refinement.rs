use kyuubiki_protocol::{SolveSpring1dRequest, Spring1dElementInput, Spring1dNodeInput};
use kyuubiki_solver::solve_spring_1d;

const TOL: f64 = 1.0e-10;
const LENGTH: f64 = 3.0;
const EQUIVALENT_STIFFNESS: f64 = 24_000.0;
const TIP_LOAD: f64 = 1_200.0;

#[test]
fn equivalent_spring_chain_is_refinement_invariant() {
    let tip_displacement = TIP_LOAD / EQUIVALENT_STIFFNESS;
    let total_energy = 0.5 * TIP_LOAD * tip_displacement;

    for elements in [1_usize, 2, 4, 8, 16, 32] {
        let result = solve_spring_1d(&mesh(elements)).expect("refined spring chain should solve");
        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(result.max_displacement, tip_displacement);
        assert_close(result.max_force, TIP_LOAD);
        assert_close(result.total_strain_energy, total_energy);

        for node in &result.nodes {
            assert_close(node.ux, tip_displacement * node.x / LENGTH);
        }
        for element in &result.elements {
            let expected_extension = tip_displacement / elements as f64;
            assert_close(element.length, LENGTH / elements as f64);
            assert_close(element.extension, expected_extension);
            assert_close(element.force, TIP_LOAD);
            assert_close(element.strain_energy, 0.5 * TIP_LOAD * expected_extension);
        }
    }
}

fn mesh(count: usize) -> SolveSpring1dRequest {
    let nodes = (0..=count)
        .map(|index| Spring1dNodeInput {
            id: format!("node-{index}"),
            x: LENGTH * index as f64 / count as f64,
            fix_x: index == 0,
            load_x: if index == count { TIP_LOAD } else { 0.0 },
        })
        .collect();
    let elements = (0..count)
        .map(|index| Spring1dElementInput {
            id: format!("spring-{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: EQUIVALENT_STIFFNESS * count as f64,
        })
        .collect();
    SolveSpring1dRequest { nodes, elements }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}",
    );
}
