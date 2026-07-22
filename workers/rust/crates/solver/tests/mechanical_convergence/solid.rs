use super::common::assert_close;
use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

#[test]
fn solid_tetra_3d_perturbations_match_single_free_node_closed_form() {
    let cases = [
        SolidTetraCase {
            height: 1.0,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            load_z: -1000.0,
        },
        SolidTetraCase {
            height: 1.0,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            load_z: -2500.0,
        },
        SolidTetraCase {
            height: 1.0,
            youngs_modulus: 210.0e9,
            poisson_ratio: 0.29,
            load_z: -1000.0,
        },
        SolidTetraCase {
            height: 1.75,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            load_z: -1000.0,
        },
    ];

    for case in cases {
        let result = solve_solid_tetra_3d(&solid_tetra_request(case))
            .expect("single-free-node solid tetra fixture should solve");
        let expected = solid_tetra_closed_form(case);
        let tip = &result.nodes[3];
        let element = &result.elements[0];

        for index in 0..3 {
            assert_close(result.nodes[index].ux, 0.0, "restrained ux");
            assert_close(result.nodes[index].uy, 0.0, "restrained uy");
            assert_close(result.nodes[index].uz, 0.0, "restrained uz");
        }

        assert_close(tip.ux, 0.0, "solid tip ux");
        assert_close(tip.uy, 0.0, "solid tip uy");
        assert_close(tip.uz, expected.tip_uz, "solid tip uz");
        assert_close(
            tip.displacement_magnitude,
            expected.tip_uz.abs(),
            "solid tip displacement magnitude",
        );
        assert_close(
            result.max_displacement,
            expected.tip_uz.abs(),
            "solid max displacement",
        );
        assert_close(element.volume, expected.volume, "solid element volume");
        assert_close(result.total_volume, expected.volume, "solid total volume");
        assert_close(element.strain_x, 0.0, "solid strain x");
        assert_close(element.strain_y, 0.0, "solid strain y");
        assert_close(element.strain_z, expected.strain_z, "solid strain z");
        assert_close(element.gamma_xy, 0.0, "solid gamma xy");
        assert_close(element.gamma_yz, 0.0, "solid gamma yz");
        assert_close(element.gamma_zx, 0.0, "solid gamma zx");
        assert_close(element.stress_x, expected.lateral_stress, "solid stress x");
        assert_close(element.stress_y, expected.lateral_stress, "solid stress y");
        assert_close(element.stress_z, expected.axial_stress, "solid stress z");
        assert_close(element.shear_xy, 0.0, "solid shear xy");
        assert_close(element.shear_yz, 0.0, "solid shear yz");
        assert_close(element.shear_zx, 0.0, "solid shear zx");
        assert_close(
            element.von_mises_stress,
            expected.von_mises,
            "solid von mises",
        );
        assert_close(
            element.strain_energy_density,
            expected.energy_density,
            "solid strain energy density",
        );
        assert_close(
            result.max_von_mises_stress,
            expected.von_mises,
            "solid max von mises",
        );
        assert_close(
            result.max_strain_energy_density,
            expected.energy_density,
            "solid max energy density",
        );
        assert_close(
            result.total_strain_energy,
            expected.total_energy,
            "solid total strain energy",
        );
    }
}

#[test]
fn solid_tetra_3d_is_objective_under_rigid_rotation() {
    let case = SolidTetraCase {
        height: 1.25,
        youngs_modulus: 110.0e9,
        poisson_ratio: 0.27,
        load_z: -1800.0,
    };
    let baseline = solve_solid_tetra_3d(&solid_tetra_request(case))
        .expect("baseline solid tetra fixture should solve");

    for angle in [std::f64::consts::FRAC_PI_6, -std::f64::consts::FRAC_PI_4] {
        let rotated_request = rotate_request_about_y(solid_tetra_request(case), angle);
        let rotated = solve_solid_tetra_3d(&rotated_request)
            .expect("rigidly rotated solid tetra fixture should solve");
        let sine = angle.sin();
        let cosine = angle.cos();
        let baseline_tip = &baseline.nodes[3];
        let rotated_tip = &rotated.nodes[3];

        assert_close(rotated_tip.ux, sine * baseline_tip.uz, "rotated tip ux");
        assert_close(rotated_tip.uy, baseline_tip.uy, "rotated tip uy");
        assert_close(rotated_tip.uz, cosine * baseline_tip.uz, "rotated tip uz");
        assert_close(
            rotated.max_displacement,
            baseline.max_displacement,
            "rotation-invariant max displacement",
        );
        assert_close(
            rotated.total_volume,
            baseline.total_volume,
            "rotation-invariant volume",
        );
        assert_close(
            rotated.max_von_mises_stress,
            baseline.max_von_mises_stress,
            "rotation-invariant von Mises stress",
        );
        assert_close(
            rotated.max_strain_energy_density,
            baseline.max_strain_energy_density,
            "rotation-invariant energy density",
        );
        assert_close(
            rotated.total_strain_energy,
            baseline.total_strain_energy,
            "rotation-invariant total strain energy",
        );

        let rotated_work = 0.5
            * (rotated_request.nodes[3].load_x * rotated_tip.ux
                + rotated_request.nodes[3].load_y * rotated_tip.uy
                + rotated_request.nodes[3].load_z * rotated_tip.uz);
        assert_close(
            rotated.total_strain_energy,
            rotated_work,
            "rotated force-displacement work",
        );
    }
}

#[derive(Clone, Copy)]
struct SolidTetraCase {
    height: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
    load_z: f64,
}

struct SolidTetraExpected {
    volume: f64,
    tip_uz: f64,
    strain_z: f64,
    lateral_stress: f64,
    axial_stress: f64,
    von_mises: f64,
    energy_density: f64,
    total_energy: f64,
}

fn solid_tetra_closed_form(case: SolidTetraCase) -> SolidTetraExpected {
    let volume = case.height / 6.0;
    let strain_stiffness = case.youngs_modulus * (1.0 - case.poisson_ratio)
        / ((1.0 + case.poisson_ratio) * (1.0 - 2.0 * case.poisson_ratio));
    let lambda = case.youngs_modulus * case.poisson_ratio
        / ((1.0 + case.poisson_ratio) * (1.0 - 2.0 * case.poisson_ratio));
    let tip_uz = case.load_z / (strain_stiffness * volume / case.height.powi(2));
    let strain_z = tip_uz / case.height;
    let lateral_stress = lambda * strain_z;
    let axial_stress = strain_stiffness * strain_z;
    let von_mises = (axial_stress - lateral_stress).abs();
    let energy_density = 0.5 * axial_stress * strain_z;

    SolidTetraExpected {
        volume,
        tip_uz,
        strain_z,
        lateral_stress,
        axial_stress,
        von_mises,
        energy_density,
        total_energy: energy_density * volume,
    }
}

fn solid_tetra_request(case: SolidTetraCase) -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            solid_node("n0", 0.0, 0.0, 0.0, true, 0.0),
            solid_node("n1", 1.0, 0.0, 0.0, true, 0.0),
            solid_node("n2", 0.0, 1.0, 0.0, true, 0.0),
            solid_node("n3", 0.0, 0.0, case.height, false, case.load_z),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "t0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: case.youngs_modulus,
            poisson_ratio: case.poisson_ratio,
        }],
    }
}

fn rotate_request_about_y(
    mut request: SolveSolidTetra3dRequest,
    angle: f64,
) -> SolveSolidTetra3dRequest {
    let sine = angle.sin();
    let cosine = angle.cos();
    for node in &mut request.nodes {
        (node.x, node.z) = (
            cosine * node.x + sine * node.z,
            -sine * node.x + cosine * node.z,
        );
        (node.load_x, node.load_z) = (
            cosine * node.load_x + sine * node.load_z,
            -sine * node.load_x + cosine * node.load_z,
        );
    }
    request
}

fn solid_node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
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
