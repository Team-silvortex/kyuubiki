//! Offline follow-up to the model-reviewed bending study. No provider or Agent calls.
use super::{bending_direct, bending_model::Case, bending_q9, provider};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

fn descriptor(value: &Value, degree: u64) -> Option<(u64, u64, u64)> {
    let mesh = value["mesh"].as_array()?;
    if mesh.len() != 2 {
        return None;
    }
    let (nx, ny) = (mesh[0].as_u64()?, mesh[1].as_u64()?);
    if !(1..=64).contains(&nx) || !(2..=32).contains(&ny) || ny % 2 != 0 {
        return None;
    }
    let nodes = (degree * nx + 1) * (degree * ny + 1);
    (value["nodes"].as_u64()? == nodes && value["mechanical_free_dofs"].as_u64()? == 2 * nodes - 3)
        .then_some((nx, ny, nodes))
}

fn compare(q4: &Value, same_mesh: &Value, same_dofs: &Value) -> bool {
    let Some((nx, ny, nodes)) = descriptor(q4, 1) else {
        return false;
    };
    descriptor(same_mesh, 2).is_some_and(|(x, y, _)| x == nx && y == ny)
        && descriptor(same_dofs, 2)
            .is_some_and(|(x, y, n)| 2 * x == nx && 2 * y == ny && n == nodes)
        && [same_mesh, same_dofs].iter().all(|q9| {
            q9["passed"] == true
                && q9["energy_error_j"].as_f64().is_some_and(|e| {
                    e.is_finite()
                        && e >= 0.0
                        && q4["energy_error_j"]
                            .as_f64()
                            .is_some_and(|old| old.is_finite() && old > 0.0 && e < old)
                })
        })
}

pub fn run(root: &Path) -> Result<(), String> {
    let definition = concat!(
        include_str!("../model_driven_thermal_reference.rs"),
        include_str!("bending_model.rs"),
        include_str!("bending_direct.rs"),
        include_str!("bending_q9.rs"),
        include_str!("bending_q9_tests.rs"),
        include_str!("bending_order.rs")
    );
    let mut rows = vec![];
    let mut controls = vec![];
    for name in ["energy_isotropic", "energy_uniform"] {
        for r in [1, 2] {
            let c = Case::new(name, r)?;
            let (nx, ny) = c.dimensions();
            eprintln!("offline order comparison: {name} Q4 {nx}x{ny}");
            let q4 = bending_direct::local_summary(c)?;
            let same_mesh = bending_q9::run(c, nx, ny)?;
            let same_dofs = bending_q9::run(c, nx / 2, ny / 2)?;
            let row = json!({"case":c.id(),"passed":compare(&q4,&same_mesh,&same_dofs),
                "q4":q4,"q9_same_mesh":same_mesh,"q9_same_mechanical_dofs":same_dofs});
            provider::retain(root, &format!("{}-order.json", c.id()), &row)?;
            rows.push(row);
        }
    }
    for name in [
        "gradient_free",
        "gradient_free_reverse",
        "bilayer_gradient_reverse",
        "bilayer_shift",
    ] {
        controls.push(json!({"case":name,"q9":bending_q9::run(Case::new(name,1)?,4,2)?}));
    }
    let passed = rows.len() == 4
        && rows.iter().all(|r| r["passed"] == true)
        && controls.len() == 4
        && controls.iter().all(|r| r["q9"]["passed"] == true);
    provider::retain(
        root,
        "report.json",
        &json!({"passed":passed,"comparisons":rows,"controls":controls,
        "scope":"bounded_local_rectangular_full_integration_element_order_comparison",
        "production_operator_added":false,"production_gates_changed":false,
        "new_workflows":0,"new_model_calls":0,"model_reviewed":false,
        "q4_solves":4,"q9_heat_and_mechanical_pairs":12,
        "shared_linear_solver":"independent_banded_cholesky_no_regularization",
        "q4_temperature":"analytical_reference_not_a_heat_solve",
        "q9_temperature":"solved_quadratic_heat_then_same_mesh_nodal_transfer",
        "sdk_version":env!("CARGO_PKG_VERSION"),
        "definition_sha256":format!("{:x}",Sha256::digest(definition))}),
    )?;
    if !passed {
        return Err("order comparison gate failed; compact evidence retained".into());
    }
    println!("offline order comparison: 4 Q4 solves, 12 Q9 heat/structure pairs; all gates passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_rejects_dof_mismatch_failed_or_nonfinite_evidence() {
        let q4 = json!({"mesh":[16,8],"nodes":153,"mechanical_free_dofs":303,"energy_error_j":0.1});
        let mesh = json!({"mesh":[16,8],"nodes":561,"mechanical_free_dofs":1119,"passed":true,"energy_error_j":1e-12});
        let dofs = json!({"mesh":[8,4],"nodes":153,"mechanical_free_dofs":303,"passed":true,"energy_error_j":1e-12});
        assert!(compare(&q4, &mesh, &dofs));
        for (key, value) in [
            ("nodes", json!(45)),
            ("mechanical_free_dofs", json!(304)),
            ("passed", json!(false)),
            ("energy_error_j", Value::Null),
            ("energy_error_j", json!(0.2)),
        ] {
            let mut bad = dofs.clone();
            bad[key] = value;
            assert!(!compare(&q4, &mesh, &bad));
        }
        let missing = json!({"energy_error_j":0.1});
        let bogus = json!({"passed":true,"energy_error_j":-1.0});
        assert!(!compare(&missing, &bogus, &bogus));
        let mut negative = dofs;
        negative["energy_error_j"] = json!(-1.0);
        assert!(!compare(&q4, &mesh, &negative));
    }
}
