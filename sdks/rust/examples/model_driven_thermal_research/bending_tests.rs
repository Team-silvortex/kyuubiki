use super::*;

#[test]
fn elastic_energy_subtracts_thermal_eigenstrain_pairing() {
    // Unit-volume bar: E=1000, strain=.003, eigenstrain=.001, stress=2.
    assert!(constitutive_balance_error(0.002, 0.006, 0.002) < 1e-15);
    // A restrained unit-volume bar has strain=0, eigenstrain=.001, stress=-1.
    assert!(constitutive_balance_error(0.0005, 0.0, -0.001) < 1e-15);
    assert!(constitutive_balance_error(0.0005, 0.0, 0.001) > 1e-4);
}

#[test]
fn bending_observation_exposes_normalizers_and_zero_reference_energy() {
    let c = Case::new("gradient_free", 1).unwrap();
    let v = validate(c, &fixture(c)).unwrap();
    assert!((v["normalization"]["stress_scale_pa"].as_f64().unwrap() - 16.8e6).abs() < 1e-8);
    assert!((v["normalization"]["energy_scale_j"].as_f64().unwrap() - 10.08).abs() < 1e-10);
    assert!(v["normalization"]["reference_nodal_l2_m"].as_f64().unwrap() > 0.0);
    assert!(
        v.get("relative_continuum_elastic_energy_error")
            .unwrap()
            .is_null()
    );
    assert_eq!(
        v["acceptance_stage"],
        "per_case_checks_matrix_convergence_still_required"
    );
    let free = v["free_dof_force_residual_n"].as_f64().unwrap();
    let fixed = v["constrained_dof_force_residual_n"].as_f64().unwrap();
    assert_eq!(
        v["all_dof_force_residual_n"].as_f64().unwrap(),
        free.max(fixed)
    );
}

#[test]
fn energy_error_components_include_parasitic_shear() {
    let c = Case::new("bilayer_gradient", 2).unwrap();
    let v = validate(c, &fixture(c)).unwrap();
    let shear = v["shear_error_energy_j"].as_f64().unwrap();
    let normal = v["normal_error_energy_j"].as_f64().unwrap();
    assert!(shear > 0.0 && normal > 0.0);
    assert!((normal + shear - v["error_energy_j"].as_f64().unwrap()).abs() < 1e-12);
    assert!(v["shear_strain_rms"].as_f64().unwrap() > 0.0);
}

fn fixture(c: Case) -> Value {
    let (mut h, mut s) = c.models();
    for n in h["nodes"].as_array_mut().unwrap() {
        n["temperature"] = json!(c.temperature(n["y"].as_f64().unwrap()));
    }
    let mut b = s.clone();
    for n in b["nodes"].as_array_mut().unwrap() {
        n["temperature_delta"] = json!(c.temperature(n["y"].as_f64().unwrap()) - c.reference);
    }
    for n in s["nodes"].as_array_mut().unwrap() {
        let x = n["x"].as_f64().unwrap();
        let y = n["y"].as_f64().unwrap();
        let u = c.displacement(x, y);
        n["ux"] = json!(u[0]);
        n["uy"] = json!(u[1]);
        n["temperature_delta"] = json!(c.temperature(y) - c.reference);
    }
    let (nx, ny) = c.dimensions();
    for e in h["elements"].as_array_mut().unwrap() {
        e["heat_flux_x"] = json!(0.0);
        e["heat_flux_y"] = json!(c.flux());
    }
    for (i, e) in s["elements"].as_array_mut().unwrap().iter_mut().enumerate() {
        let y = (i / nx) as f64 * HEIGHT / ny as f64 + HEIGHT / ny as f64 / 2.0;
        let layer = usize::from(i / nx >= ny / 2);
        let strain = c.strain(y, layer);
        let sx = c.stress(y, layer);
        let thermal = c.alpha[layer] * (c.temperature(y) - c.reference);
        for (k, v) in [
            ("area", HEIGHT / (nx * ny) as f64),
            ("total_strain_x", strain[0]),
            ("total_strain_y", strain[1]),
            ("gamma_xy", 0.0),
            ("mechanical_strain_x", strain[0] - thermal),
            ("mechanical_strain_y", strain[1] - thermal),
            ("thermal_strain", thermal),
            ("stress_x", sx),
            ("stress_y", 0.0),
            ("tau_xy", 0.0),
            ("principal_stress_1", sx.max(0.0)),
            ("principal_stress_2", sx.min(0.0)),
            ("max_in_plane_shear", sx.abs() / 2.0),
            ("von_mises", sx.abs()),
            ("strain_energy_density", sx * sx / (2.0 * c.young[layer])),
        ] {
            e[k] = json!(v);
        }
    }
    s["total_strain_energy"] = json!(c.energy());
    json!({"workflow_id":format!("research.{}",c.id()),"artifacts":{
        "heat_out.result":h,"bridge_out.model":b,"structure_out.result":s}})
}

#[test]
fn bending_checker_accepts_affine_limit_but_not_fake_exact_quadratic_output() {
    let mut c = Case::new("bilayer_uniform", 2).unwrap();
    c.alpha = [12e-6; 2];
    let good = fixture(c);
    assert_eq!(validate(c, &good).unwrap()["passed"], true);
    for (path, value) in [
        ("/artifacts/structure_out.result/nodes/3/uy", json!(1e-3)),
        (
            "/artifacts/structure_out.result/elements/0/tau_xy",
            json!(1e5),
        ),
        (
            "/artifacts/structure_out.result/total_strain_energy",
            json!(1.0),
        ),
        (
            "/artifacts/heat_out.result/elements/0/heat_flux_y",
            json!(1.0),
        ),
        ("/artifacts/bridge_out.model/nodes/1/fix_y", json!(true)),
        (
            "/artifacts/bridge_out.model/elements/0/poisson_ratio",
            json!(0.0),
        ),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            validate(c, &bad).map_or(true, |v| v["passed"] != true),
            "accepted {path}"
        );
    }
    let c = Case::new("gradient_free", 1).unwrap();
    assert_ne!(
        validate(c, &fixture(c)).unwrap()["passed"],
        true,
        "exact quadratic nodal samples are not an equilibrium Q4 solve"
    );
}

#[test]
#[ignore = "requires explicitly selected real retained bending results"]
fn bending_replay_retained_results() {
    let root = std::env::var("KYUUBIKI_BENDING_REPLAY_DIR").expect("set replay directory");
    let mut count = 0;
    for name in super::super::bending_model::NAMES {
        for r in [1, 2, 4, 8] {
            let c = Case::new(name, r).unwrap();
            let p = std::path::Path::new(&root).join(format!("{}-result.json", c.id()));
            let data = match std::fs::read(&p) {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => panic!("{e}"),
            };
            let result = serde_json::from_slice(&data).unwrap();
            let v = validate(c, &result).unwrap();
            assert_eq!(v["passed"], true, "{}: {v}", c.id());
            println!(
                "replay_validation: {}",
                json!({"case":c.id(),"validation":v})
            );
            count += 1;
        }
    }
    assert!(count > 0);
    println!("validated {count} retained bending results");
}

#[test]
#[ignore = "requires explicit opt-in and running local Orchestra/Agents; no model API"]
fn bending_live_unseeded_temperature_probe() {
    use sha2::{Digest, Sha256};
    assert_eq!(std::env::var("KYUUBIKI_LIVE_RESEARCH").as_deref(), Ok("1"));
    let dir = std::env::var("KYUUBIKI_BENDING_PROBE_DIR").expect("set a new probe directory");
    let root = std::path::Path::new(&dir);
    std::fs::create_dir(root).unwrap();
    let definition = concat!(
        include_str!("bending_model.rs"),
        include_str!("bending_checks.rs"),
        include_str!("bending_tests.rs"),
        include_str!("study.rs"),
        include_str!("provider.rs"),
        include_str!("policy.rs"),
        include_str!("../layered_thermal_research/model.rs")
    );
    super::super::provider::retain(
        root,
        "approval.json",
        &json!({"scope":"four_finest_mesh_unseeded_temperature_controls",
        "max_jobs":4,"model_calls":0,"automatic_retry":false,
        "definition_sha256":format!("{:x}",Sha256::digest(definition.as_bytes()))}),
    )
    .unwrap();
    let mut study = super::super::study::Study::new(root).unwrap();
    for name in [
        "gradient_free",
        "bilayer_gradient",
        "bilayer_uniform",
        "bilayer_shift",
    ] {
        let c = Case::new(name, 8).unwrap();
        study
            .evaluate_bending(c, "caller_unseeded_temperature_control")
            .unwrap();
    }
    assert_eq!(study.jobs, 4);
    super::super::provider::retain(
        root,
        "report.json",
        &json!({"complete":true,"api_calls":0,
        "jobs":study.jobs,"cases":study.rows,"full_matrix_repeated":false}),
    )
    .unwrap();
}
