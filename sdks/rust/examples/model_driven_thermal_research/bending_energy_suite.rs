use super::bending_model::{Case, ENERGY_NAMES};
use serde_json::{Value, json};

pub const BRIEF: &str = "Investigate the previous bilayer bending total elastic-energy error of 4.919177% at a 32x16 Q4 mesh. \
This is a SYNTHETIC plane-stress 1x0.25m rectangle, thickness .01m, two equal y-layers, E=[70,20]GPa, nu=[.2,.35], alpha=[10,30]e-6/K, k=[10,40]. \
Bottom/top/reference temperatures are 20/40/20 except energy_uniform=30/30/20. Side heat flux is zero; free thermal nodes start at reference, not the answer. \
Heat Q4 -> temperature bridge -> thermal elastic Q4 on real Agents. Only origin ux/uy and bottom-right uy restrained. \
NONZERO analytical self-equilibrated end tractions sigma_x(y)*nx are applied on both ends: not a traction-free bilayer or cantilever. \
Exact continuum ex=e0+k*y, sy=tau=0, zero section force/moment; piecewise quadratic displacement. Gradient continuum energy .30305220883534134J, uniform .12650602409638567J. \
Approved refinements r=[1,2,4]: energy_isotropic and energy_uniform mesh16r x 8r; energy_x mesh16r x16; energy_y mesh32 x8r. \
Choose ALL FOUR scenarios in batches1..4 using evaluate_cases, observe before choosing next batch. Caller completes all12 cases, max10turns. \
Competing hypotheses: length/thickness interpolation including parasitic shear, erroneous energy reporting, iteration tolerance. Each result includes independently reconstructed energy/error components and an independently assembled banded-Cholesky Q4 solve. \
The direct comparison tests the SAME discretization, not an independent continuum model; both could share Q4 limitations. Exact nodal interpolation itself is not the FE equilibrium solution. \
Production source selects dense for <=1024 free DOFs, sparse otherwise, but workflow outputs do not certify which backend ran. No tolerance override is exposed here. \
Caller owns consistency/direct-comparison gates, isotropic mesh energy-error convergence and <=2% final relative energy error for isotropic gradient/uniform, no accuracy target for directional probes. \
Normal-error energy includes coupled x/y terms; shear-error energy is separate. Final f^T*u is not path work. 2U=f^T*u-integral(sigma:thermal_strain). \
Do not claim universal shear-locking diagnosis, nonlinear/material qualification or full solver certification. Do not invent data, run code, alter gates or authorize spending. \
Finish ONLY JSON {\"findings\":[string],\"limitations\":[string],\"next_experiments\":[string]}.";

pub fn catalog() -> Value {
    json!(ENERGY_NAMES.map(|name| {
        let c=Case::new(name,1).unwrap();
        json!({"scenario":name,"meshes":([1,2,4].map(|r|Case::new(name,r).unwrap().dimensions())),
            "continuum_energy_j":c.energy(),"end_traction":"analytical_self_equilibrated_not_traction_free"})
    }))
}

pub fn verify(rows: &[Value]) -> Result<Value, String> {
    let mut jobs = std::collections::BTreeSet::new();
    let mut cases = vec![];
    for name in ENERGY_NAMES {
        let mut previous = None;
        for r in [1, 2, 4] {
            let c = Case::new(name, r)?;
            let matching: Vec<_> = rows.iter().filter(|v| v["case"] == c.id()).collect();
            if matching.len() != 1 {
                return Err("missing or duplicated energy case".into());
            }
            let row = matching[0];
            let job = row["job_id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("energy job missing")?;
            let v = &row["bending_validation"];
            if !jobs.insert(job)
                || row["physics_passed"] != true
                || row["readback_matches"] != true
                || row["reused_retained_evidence"] != false
                || v["passed"] != true
                || row["direct_validation"]["passed"] != true
            {
                return Err("energy evidence/direct comparison failed".into());
            }
            let expected = if name == "energy_uniform" {
                0.12650602409638567
            } else {
                0.30305220883534134
            };
            let exact = v["expected_elastic_energy_j"]
                .as_f64()
                .filter(|x| x.is_finite())
                .ok_or("missing reference energy")?;
            if (exact - expected).abs() > 1e-12 {
                return Err("energy continuum reference drift".into());
            }
            let err = v["relative_continuum_elastic_energy_error"]
                .as_f64()
                .filter(|x| x.is_finite() && *x >= 0.0)
                .ok_or("invalid relative energy error")?;
            if ["energy_isotropic", "energy_uniform"].contains(&name) {
                if previous.is_some_and(|old: f64| err > 0.8 * old + 1e-12)
                    || (r == 4 && err > 0.02)
                {
                    return Err("isotropic energy convergence/accuracy failed".into());
                }
                previous = Some(err);
            }
            cases.push(super::bending_suite::observation(row));
        }
    }
    Ok(
        json!({"passed":true,"cases":cases,"scope":"bounded_directional_bilayer_energy_diagnosis",
        "isotropic_finest_relative_energy_limit":0.02,"directional_accuracy_qualification":false,
        "solver_tolerances_changed":false,"direct_reference_is_same_q4_discretization":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_requires_direct_evidence_and_actual_energy_convergence() {
        let rows: Vec<_> = ENERGY_NAMES
            .iter()
            .flat_map(|name| {
                [1,2,4].map(|r| {
            let c=Case::new(name,r).unwrap();
            json!({"case":c.id(),"job_id":c.id(),"physics_passed":true,"readback_matches":true,
                "reused_retained_evidence":false,"direct_validation":{"passed":true},
                "bending_validation":{"passed":true,"expected_elastic_energy_j":c.energy(),
                    "relative_continuum_elastic_energy_error":0.1/(r*r) as f64}})
        })
            })
            .collect();
        assert!(verify(&rows).is_ok());
        for (i, path, value) in [
            (0, "/direct_validation/passed", json!(false)),
            (0, "/job_id", rows[1]["job_id"].clone()),
            (
                1,
                "/bending_validation/relative_continuum_elastic_energy_error",
                json!(0.11),
            ),
            (
                2,
                "/bending_validation/relative_continuum_elastic_energy_error",
                json!(0.021),
            ),
            (
                0,
                "/bending_validation/expected_elastic_energy_j",
                json!(0.0),
            ),
        ] {
            let mut bad = rows.clone();
            *bad[i].pointer_mut(path).unwrap() = value;
            assert!(verify(&bad).is_err(), "accepted {path}");
        }
        assert!(verify(&rows[1..]).is_err());
        let mut duplicate = rows.clone();
        duplicate.push(rows[0].clone());
        assert!(verify(&duplicate).is_err());
    }
}
