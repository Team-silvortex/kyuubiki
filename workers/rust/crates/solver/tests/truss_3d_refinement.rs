use kyuubiki_protocol::{SolveTruss3dRequest, Truss3dElementInput, Truss3dNodeInput};
use kyuubiki_solver::solve_truss_3d;

const TOL: f64 = 1.0e-8;
const RADIUS: f64 = 0.6;
const HEIGHT: f64 = 0.9;
const LOAD_Z: f64 = -1_500.0;
const AREA: f64 = 0.012;
const YOUNGS_MODULUS: f64 = 70.0e9;

#[test]
fn symmetric_tripod_area_partition_is_refinement_invariant() {
    let length = (RADIUS * RADIUS + HEIGHT * HEIGHT).sqrt();
    let vertical_direction = HEIGHT / length;
    let member_force = LOAD_Z / (3.0 * vertical_direction);
    let apex_uz =
        LOAD_Z * length / (3.0 * YOUNGS_MODULUS * AREA * vertical_direction * vertical_direction);
    let stress = member_force / AREA;
    let strain = stress / YOUNGS_MODULUS;
    let energy_density = 0.5 * stress * strain;
    let total_energy = 3.0 * energy_density * AREA * length;

    for partitions in [1_usize, 2, 4, 8, 16] {
        let result = solve_truss_3d(&mesh(partitions)).expect("partitioned 3D truss should solve");
        assert_eq!(result.elements.len(), partitions * 3);
        for support in &result.nodes[..3] {
            assert_close(support.ux, 0.0);
            assert_close(support.uy, 0.0);
            assert_close(support.uz, 0.0);
        }
        let apex = &result.nodes[3];
        assert_close(apex.ux, 0.0);
        assert_close(apex.uy, 0.0);
        assert_close(apex.uz, apex_uz);
        assert_close(result.max_displacement, apex_uz.abs());
        assert_close(result.max_stress, stress.abs());
        assert_close(result.max_strain_energy_density, energy_density.abs());
        assert_close(result.total_strain_energy, total_energy);
        assert_close(result.total_strain_energy, 0.5 * LOAD_Z * apex_uz);

        let mut force_sums = [0.0_f64; 3];
        for (index, element) in result.elements.iter().enumerate() {
            assert_close(element.length, length);
            assert_close(element.axial_force, member_force / partitions as f64);
            assert_close(element.stress, stress);
            assert_close(element.strain, strain);
            assert_close(element.strain_energy_density, energy_density);
            force_sums[index / partitions] += element.axial_force;
        }
        for force_sum in force_sums {
            assert_close(force_sum, member_force);
        }
    }
}

fn mesh(partitions: usize) -> SolveTruss3dRequest {
    let root_three_over_two = 3.0_f64.sqrt() * 0.5;
    let nodes = vec![
        node("base-a", RADIUS, 0.0, 0.0, true, 0.0),
        node(
            "base-b",
            -0.5 * RADIUS,
            root_three_over_two * RADIUS,
            0.0,
            true,
            0.0,
        ),
        node(
            "base-c",
            -0.5 * RADIUS,
            -root_three_over_two * RADIUS,
            0.0,
            true,
            0.0,
        ),
        node("apex", 0.0, 0.0, HEIGHT, false, LOAD_Z),
    ];
    let partition_area = AREA / partitions as f64;
    let mut elements = Vec::with_capacity(partitions * 3);
    for index in 0..partitions {
        elements.push(element(format!("leg-a-{index}"), 0, 3, partition_area));
    }
    for index in 0..partitions {
        elements.push(element(format!("leg-b-{index}"), 1, 3, partition_area));
    }
    for index in 0..partitions {
        elements.push(element(format!("leg-c-{index}"), 2, 3, partition_area));
    }
    SolveTruss3dRequest { nodes, elements }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> Truss3dNodeInput {
    Truss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
    }
}

fn element(id: String, node_i: usize, node_j: usize, area: f64) -> Truss3dElementInput {
    Truss3dElementInput {
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
