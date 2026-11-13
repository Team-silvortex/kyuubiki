use super::super::planar_patch::{ALPHA, HEIGHT, THICKNESS};
use super::*;

#[test]
#[ignore = "requires an explicitly selected retained local research directory"]
fn planar_replay_retained_results() {
    let root = std::env::var("KYUUBIKI_PLANAR_REPLAY_DIR").expect("set replay directory");
    let mut checked = 0;
    for name in super::super::planar_patch::NAMES {
        for r in [1, 2, 4, 8] {
            let c = Case::new(name, r).unwrap();
            let path = std::path::Path::new(&root).join(format!("{}-result.json", c.id()));
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => panic!("{}: {e}", path.display()),
            };
            let result = serde_json::from_slice(&bytes).unwrap();
            let v = validate(c, &result).unwrap();
            assert_eq!(v["passed"], true, "{}: {v}", path.display());
            println!(
                "replay_validation: {}",
                json!({"case":c.id(),"validation":v})
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no retained results found");
    println!("validated {checked} retained real results without a model call");
}

fn fixture(c: Case) -> Value {
    let (mut heat, mut s) = c.models();
    let mut bridge = s.clone();
    for n in bridge["nodes"].as_array_mut().unwrap() {
        n["temperature_delta"] = json!(c.rise);
    }
    for n in s["nodes"].as_array_mut().unwrap() {
        let u = c.displacement(n["x"].as_f64().unwrap(), n["y"].as_f64().unwrap());
        n["ux"] = json!(u[0]);
        n["uy"] = json!(u[1]);
        n["temperature_delta"] = json!(c.rise);
    }
    for e in heat["elements"].as_array_mut().unwrap() {
        e["heat_flux_x"] = json!(0.0);
        e["heat_flux_y"] = json!(0.0);
    }
    let nodes = s["nodes"].clone();
    for e in s["elements"].as_array_mut().unwrap() {
        let ids: [usize; 4] =
            ["node_i", "node_j", "node_k", "node_l"].map(|k| e[k].as_u64().unwrap() as usize);
        let area = (0..4)
            .map(|i| {
                let a = &nodes[ids[i]];
                let b = &nodes[ids[(i + 1) % 4]];
                (a["x"].as_f64().unwrap() * b["y"].as_f64().unwrap()
                    - b["x"].as_f64().unwrap() * a["y"].as_f64().unwrap())
                    / 2.0
            })
            .sum::<f64>();
        let [ex, ey, gamma] = c.strain();
        let [sx, sy, tau] = c.stress;
        let radius = ((sx - sy).powi(2) / 4.0 + tau * tau).sqrt();
        for (k, v) in [
            ("area", area),
            ("total_strain_x", ex),
            ("total_strain_y", ey),
            ("gamma_xy", gamma),
            ("mechanical_strain_x", ex - ALPHA * c.rise),
            ("mechanical_strain_y", ey - ALPHA * c.rise),
            ("thermal_strain", ALPHA * c.rise),
            ("average_temperature_delta", c.rise),
            ("stress_x", sx),
            ("stress_y", sy),
            ("tau_xy", tau),
            ("principal_stress_1", (sx + sy) / 2.0 + radius),
            ("principal_stress_2", (sx + sy) / 2.0 - radius),
            ("max_in_plane_shear", radius),
            (
                "von_mises",
                (sx * sx - sx * sy + sy * sy + 3.0 * tau * tau).sqrt(),
            ),
            ("strain_energy_density", c.energy_density()),
        ] {
            e[k] = json!(v);
        }
    }
    s["total_strain_energy"] = json!(c.energy_density() * HEIGHT * THICKNESS);
    json!({"workflow_id":format!("research.{}",c.id()),"artifacts":{
        "heat_out.result":heat,"bridge_out.model":bridge,"structure_out.result":s}})
}

#[test]
fn planar_affine_fields_pass_on_regular_and_distorted_meshes() {
    for name in super::super::planar_patch::NAMES {
        for r in [1, 2, 4, 8] {
            let c = Case::new(name, r).unwrap();
            let v = validate(c, &fixture(c)).unwrap();
            assert_eq!(v["passed"], true, "{name} r{r}: {v}");
        }
    }
}

#[test]
fn planar_bridge_accepts_equivalent_json_numbers_without_accepting_drift() {
    let c = Case::new("tension", 1).unwrap();
    let mut result = fixture(c);
    let bridge = &mut result["artifacts"]["bridge_out.model"];
    for node in bridge["nodes"].as_array_mut().unwrap() {
        for key in ["load_x", "load_y"] {
            let value = node[key].as_f64().unwrap();
            assert_eq!(value.fract(), 0.0);
            node[key] = json!(value as i64);
        }
    }
    for element in bridge["elements"].as_array_mut().unwrap() {
        element["youngs_modulus"] = json!(70_000_000_000_u64);
    }
    assert_eq!(validate(c, &result).unwrap()["passed"], true);
    for (path, value) in [
        ("/nodes/0/load_x", json!(123.0)),
        ("/nodes/0/load_y", Value::Null),
        ("/nodes/0/fix_x", json!(1)),
        ("/nodes/1/fix_y", Value::Null),
        ("/elements/0/youngs_modulus", json!(70_000_000_001_u64)),
        ("/elements/0/thermal_expansion", json!("0.000012")),
    ] {
        let mut bad = result.clone();
        *bad["artifacts"]["bridge_out.model"]
            .pointer_mut(path)
            .unwrap() = value;
        assert!(validate(c, &bad).is_err(), "accepted drift at {path}");
    }
}

#[test]
fn planar_observations_separate_invariants_and_thermal_traction_pairing() {
    for (name, total, thermal) in [
        ("mixed_hot", 0.32352142857142857, 0.24),
        ("mixed_auxetic", -0.18457142857142856, -0.24),
        ("shear", 0.02321428571428572, 0.0),
    ] {
        let c = Case::new(name, 2).unwrap();
        let result = fixture(c);
        let v = validate(c, &result).unwrap();
        assert!((v["traction_displacement_pairing_j"].as_f64().unwrap() - total).abs() < 1e-12);
        assert!((v["thermal_traction_pairing_j"].as_f64().unwrap() - thermal).abs() < 1e-12);
        assert!(
            (v["elastic_traction_pairing_j"].as_f64().unwrap() - (total - thermal)).abs() < 1e-12
        );
        assert_eq!(v["path_integrated_work_evaluated"], false);
        assert!(v.get("mechanical_external_work_j").is_none());
        assert!(v.get("stress").is_none());
        assert_eq!(v["expected_stress_pa"], json!(c.stress));
        let fields = v["observed_stress_fields_pa"].as_object().unwrap();
        assert_eq!(fields.len(), 7);
        for (key, field) in fields {
            let observed = &result["artifacts"]["structure_out.result"]["elements"][0][key];
            assert_eq!(&field["min"], observed);
            assert_eq!(&field["max"], observed);
            assert_eq!(&field["expected"], observed);
            assert_eq!(field["max_abs_error"], 0.0);
        }
        if name == "shear" {
            assert!(
                (fields["von_mises"]["min"].as_f64().unwrap() - 866025.4037844386).abs() < 1e-8
            );
        }
        let mut changed = result.clone();
        changed["artifacts"]["structure_out.result"]["elements"][0]["von_mises"] = json!(0.0);
        let bad = validate(c, &changed).unwrap();
        assert_eq!(bad["passed"], false);
        assert_eq!(bad["observed_stress_fields_pa"]["von_mises"]["min"], 0.0);
        assert!(
            bad["observed_stress_fields_pa"]["von_mises"]["max_abs_error"]
                .as_f64()
                .unwrap()
                > 1.0
        );
    }
}

#[test]
fn planar_checker_rejects_false_transverse_constraints_and_shear_conventions() {
    let c = Case::new("mixed_hot", 2).unwrap();
    let good = fixture(c);
    for (path, value) in [
        ("/artifacts/structure_out.result/nodes/12/uy", json!(0.0)),
        (
            "/artifacts/structure_out.result/elements/0/gamma_xy",
            json!(c.strain()[2] / 2.0),
        ),
        (
            "/artifacts/structure_out.result/elements/0/tau_xy",
            json!(0.0),
        ),
        (
            "/artifacts/structure_out.result/elements/0/principal_stress_1",
            json!(0.0),
        ),
        (
            "/artifacts/structure_out.result/elements/0/von_mises",
            Value::Null,
        ),
        (
            "/artifacts/structure_out.result/total_strain_energy",
            json!(0.0),
        ),
        ("/artifacts/bridge_out.model/nodes/12/fix_y", json!(true)),
        (
            "/artifacts/bridge_out.model/elements/0/poisson_ratio",
            json!(0.0),
        ),
        (
            "/artifacts/heat_out.result/elements/0/heat_flux_y",
            json!(1.0),
        ),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            validate(c, &bad).map_or(true, |v| v["passed"] != true),
            "accepted {path}"
        );
    }
}

#[test]
fn planar_matrix_requires_complete_unique_and_explicit_execution_evidence() {
    let rows:Vec<_>=super::super::planar_patch::NAMES.iter().flat_map(|name|[1,2,4,8].map(|r| {
        let c=Case::new(name,r).unwrap();
        json!({"case":c.id(),"scenario":name,"refinement":r,"job_id":c.id(),"physics_passed":true,
            "readback_matches":true,"reused_retained_evidence":false,"planar_validation":validate(c,&fixture(c)).unwrap()})
    })).collect();
    assert_eq!(
        super::super::planar_patch::verify(&rows).unwrap()["passed"],
        true
    );
    assert!(super::super::planar_patch::verify(&rows[1..]).is_err());
    for (path, value) in [
        ("/reused_retained_evidence", Value::Null),
        ("/readback_matches", json!(false)),
        ("/job_id", rows[1]["job_id"].clone()),
        ("/planar_validation/corner_uy", json!(0.0)),
        ("/planar_validation/observed_energy_j", json!(1.0)),
    ] {
        let mut bad = rows.clone();
        *bad[0].pointer_mut(path).unwrap() = value;
        assert!(
            super::super::planar_patch::verify(&bad).is_err(),
            "accepted {path}"
        );
    }
}
