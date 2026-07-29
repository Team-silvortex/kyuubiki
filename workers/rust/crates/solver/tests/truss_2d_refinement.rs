use kyuubiki_protocol::{SolveTruss2dRequest, TrussElementInput, TrussNodeInput};
use kyuubiki_solver::solve_truss_2d;

const TOL: f64 = 1.0e-8;
const HALF_SPAN: f64 = 0.5;
const HEIGHT: f64 = 0.75;
const LOAD_Y: f64 = -1_000.0;
const AREA: f64 = 0.01;
const YOUNGS_MODULUS: f64 = 70.0e9;

#[test]
fn symmetric_two_bar_area_partition_is_refinement_invariant() {
    let length = (HALF_SPAN * HALF_SPAN + HEIGHT * HEIGHT).sqrt();
    let sin_theta = HEIGHT / length;
    let member_force = LOAD_Y / (2.0 * sin_theta);
    let apex_uy = LOAD_Y * length / (2.0 * YOUNGS_MODULUS * AREA * sin_theta * sin_theta);
    let stress = member_force / AREA;
    let strain = stress / YOUNGS_MODULUS;
    let energy_density = 0.5 * stress * strain;
    let total_energy = 2.0 * energy_density * AREA * length;

    for partitions in [1_usize, 2, 4, 8, 16] {
        let result = solve_truss_2d(&mesh(partitions)).expect("partitioned 2D truss should solve");
        assert_eq!(result.elements.len(), partitions * 2);
        assert_close(result.nodes[0].ux, 0.0);
        assert_close(result.nodes[0].uy, 0.0);
        assert_close(result.nodes[1].ux, 0.0);
        assert_close(result.nodes[1].uy, 0.0);
        assert_close(result.nodes[2].ux, 0.0);
        assert_close(result.nodes[2].uy, apex_uy);
        assert_close(result.max_displacement, apex_uy.abs());
        assert_close(result.max_stress, stress.abs());
        assert_close(result.max_strain_energy_density, energy_density.abs());
        assert_close(result.total_strain_energy, total_energy);
        assert_close(result.total_strain_energy, 0.5 * LOAD_Y * apex_uy);

        let mut left_force_sum = 0.0;
        let mut right_force_sum = 0.0;
        for (index, element) in result.elements.iter().enumerate() {
            assert_close(element.length, length);
            assert_close(element.axial_force, member_force / partitions as f64);
            assert_close(element.stress, stress);
            assert_close(element.strain, strain);
            assert_close(element.strain_energy_density, energy_density);
            if index < partitions {
                left_force_sum += element.axial_force;
            } else {
                right_force_sum += element.axial_force;
            }
        }
        assert_close(left_force_sum, member_force);
        assert_close(right_force_sum, member_force);
    }
}

fn mesh(partitions: usize) -> SolveTruss2dRequest {
    let nodes = vec![
        node("left", -HALF_SPAN, 0.0, true, true, 0.0),
        node("right", HALF_SPAN, 0.0, true, true, 0.0),
        node("apex", 0.0, HEIGHT, false, false, LOAD_Y),
    ];
    let partition_area = AREA / partitions as f64;
    let mut elements = Vec::with_capacity(partitions * 2);
    for index in 0..partitions {
        elements.push(element(format!("left-{index}"), 0, 2, partition_area));
    }
    for index in 0..partitions {
        elements.push(element(format!("right-{index}"), 1, 2, partition_area));
    }
    SolveTruss2dRequest { nodes, elements }
}

fn node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool, load_y: f64) -> TrussNodeInput {
    TrussNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y,
    }
}

fn element(id: String, node_i: usize, node_j: usize, area: f64) -> TrussElementInput {
    TrussElementInput {
        id,
        node_i,
        node_j,
        area,
        youngs_modulus: YOUNGS_MODULUS,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}",
    );
}
