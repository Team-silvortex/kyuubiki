use super::bending_model::{Case, NAMES};
use serde_json::{Value, json};

pub fn catalog() -> Value {
    json!(NAMES.map(|name| {
        let c=Case::new(name,1).unwrap();
        json!({"scenario":name,"young_pa":c.young,"poisson":c.poisson,"expansion_per_k":c.alpha,
            "conductivity":c.conductivity,"bottom_temperature":c.bottom,"top_temperature":c.top,
            "strain_reference":c.reference,"end_tractions":if name.starts_with("gradient_free"){"none"}else{"analytical_self_equilibrated_not_traction_free"}})
    }))
}

pub fn observation(row: &Value) -> Value {
    json!({"case":row["case"],"scenario":row["scenario"],"refinement":row["refinement"],
        "job_id":row["job_id"],"physics_passed":row["physics_passed"],"readback_matches":row["readback_matches"],
        "reused_retained_evidence":row["reused_retained_evidence"],"bending_validation":row["bending_validation"],
        "direct_validation":row["direct_validation"]})
}

pub fn verify(rows: &[Value]) -> Result<Value, String> {
    let mut jobs = std::collections::BTreeSet::new();
    let mut cases = vec![];
    for (name, bow, flux) in [
        ("gradient_free", 120e-6, -1600.0),
        ("gradient_free_reverse", -120e-6, 1600.0),
        ("bilayer_gradient", 0.0003595180722891566, -1280.0),
        ("bilayer_gradient_reverse", -0.0003595180722891566, 1280.0),
        ("bilayer_uniform", 0.00013493975903614458, 0.0),
        ("bilayer_shift", 0.0003595180722891566, -1280.0),
    ] {
        let mut previous: Option<[f64; 2]> = None;
        for r in [1, 2, 4, 8] {
            let c = Case::new(name, r)?;
            let row = rows
                .iter()
                .find(|x| x["case"] == c.id())
                .ok_or("missing bending matrix case")?;
            let job = row["job_id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("missing bending job")?;
            let v = &row["bending_validation"];
            if !jobs.insert(job)
                || row["physics_passed"] != true
                || row["readback_matches"] != true
                || row["reused_retained_evidence"] != false
                || v["passed"] != true
            {
                return Err("invalid bending evidence".into());
            }
            let get = |k: &str| {
                v[k].as_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| format!("missing bending metric {k}"))
            };
            if (get("expected_mid_bottom_uy")? - bow).abs() > 1e-12
                || (get("expected_heat_flux_y")? - flux).abs() > 1e-9
            {
                return Err("bending reference drift".into());
            }
            let errors = [
                get("relative_nodal_rms_error")?,
                get("normalized_error_energy")?,
            ];
            if errors.iter().any(|x| *x < 0.0) {
                return Err("negative convergence error".into());
            }
            if let Some(p) = previous {
                if errors
                    .iter()
                    .zip(p)
                    .any(|(new, old)| *new > 0.8 * old + 1e-12)
                {
                    return Err(format!("bending convergence failed: {name} r{r}"));
                }
            }
            if r == 8
                && (errors[0] > 0.02
                    || errors[1] > 0.001
                    || get("normalized_stress_l2_error")? > 0.1
                    || (get("mid_bottom_uy")? - bow).abs() > 0.02 * bow.abs())
            {
                return Err(format!("bending finest mesh accuracy failed: {name}"));
            }
            previous = Some(errors);
            cases.push(observation(row));
        }
    }
    Ok(
        json!({"passed":true,"scope":"six_nonaffine_bending_cases_four_mesh_levels",
        "cases":cases,"error_reduction_limit_per_halving":0.8,"finest_relative_nodal_rms_limit":0.02,
        "finest_normalized_error_energy_limit":0.001,"general_traction_free_bilayer_qualification":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn convergence_requires_distinct_real_cases_and_actual_error_reduction() {
        let rows:Vec<_>=NAMES.iter().flat_map(|name|[1,2,4,8].map(|r| {
            let c=Case::new(name,r).unwrap();
            json!({"case":c.id(),"job_id":c.id(),"physics_passed":true,"readback_matches":true,
                "reused_retained_evidence":false,"bending_validation":{"passed":true,
                "expected_mid_bottom_uy":c.displacement(0.5,0.0)[1],"mid_bottom_uy":c.displacement(0.5,0.0)[1],
                "expected_heat_flux_y":c.flux(),"relative_nodal_rms_error":0.1/(r*r) as f64,
                "normalized_error_energy":0.01/(r*r) as f64,"normalized_stress_l2_error":0.1/r as f64}})
        })).collect();
        assert!(verify(&rows).is_ok());
        assert!(verify(&rows[1..]).is_err());
        for (index, path, value) in [
            (1, "/job_id", rows[0]["job_id"].clone()),
            (0, "/reused_retained_evidence", Value::Null),
            (
                2,
                "/bending_validation/relative_nodal_rms_error",
                json!(0.2),
            ),
            (3, "/bending_validation/mid_bottom_uy", json!(0.0)),
            (
                3,
                "/bending_validation/normalized_stress_l2_error",
                json!(0.5),
            ),
            (0, "/bending_validation/expected_heat_flux_y", json!(0.0)),
        ] {
            let mut bad = rows.clone();
            *bad[index].pointer_mut(path).unwrap() = value;
            assert!(verify(&bad).is_err(), "accepted {path}");
        }
    }
}
