use super::{model::Case, scenarios::Experiment, study};
use serde_json::{Value, json};

pub const NAMES: [&str; 8] = [
    "control",
    "poisson_only",
    "ratio_only",
    "coupled",
    "coupled_hot",
    "coupled_cold",
    "coupled_auxetic",
    "coupled_high_nu",
];

pub fn case(e: &Experiment) -> Result<Case, String> {
    let mut c = study::case(e.design(), 20.0, 20.0)?;
    match e.scenario.as_str() {
        "control" => (),
        "poisson_only" => c.poisson = [0.3, 0.3],
        "ratio_only" => c.expansion = [10e-6, 35e-6],
        "coupled" | "coupled_hot" | "coupled_cold" | "coupled_high_nu" => {
            c.expansion = [10e-6, 35e-6];
            c.poisson = [0.2, 0.4];
            if e.scenario == "coupled_high_nu" {
                c.poisson = [0.49, 0.25];
            }
            if e.scenario == "coupled_hot" || e.scenario == "coupled_cold" {
                c.rise = 0.0;
                c.cold_offset = if e.scenario == "coupled_hot" {
                    10.0
                } else {
                    -10.0
                };
            }
        }
        "coupled_auxetic" => {
            c.expansion = [18e-6, 6e-6];
            c.poisson = [-0.2, 0.3];
        }
        _ => return Err("unknown material coupling scenario".into()),
    }
    Ok(c)
}

pub fn verify(rows: &[Value]) -> Result<Value, String> {
    let mut observations = vec![];
    let mut jobs = std::collections::BTreeSet::new();
    for (name, tip) in NAMES.into_iter().zip([
        100e-6,
        130e-6,
        125e-6,
        485e-6 / 3.0,
        305e-6,
        -305e-6,
        109e-6,
        172.25e-6,
    ]) {
        for refinement in [1, 2, 4, 8] {
            let c = case(&Experiment {
                scenario: name.into(),
                refinement,
            })?;
            let row = rows
                .iter()
                .find(|r| r["case"] == c.id())
                .ok_or("missing coupling scenario/mesh")?;
            let id = row["job_id"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or("missing coupling execution identity")?;
            if !jobs.insert(id)
                || row["physics_passed"] != true
                || row["mechanical_validation"]["passed"] != true
                || row["field_validation"]["passed"] != true
                || row["expansion"] != json!(c.expansion)
                || row["poisson"] != json!(c.poisson)
            {
                return Err("coupling evidence provenance/validation mismatch".into());
            }
            let flux = if name == "coupled_hot" || name == "coupled_cold" {
                0.0
            } else {
                8000.0 / 3.0
            };
            for (field, expected, tol) in [("tip_ux", tip, 1e-9), ("heat_flux_x", flux, 1e-5)] {
                let actual = row[field]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or("invalid coupling metric")?;
                if (actual - expected).abs() > tol {
                    return Err(format!("{name}: independent {field} reference failed"));
                }
            }
            observations.push(super::scenarios::observation(name, row));
        }
    }
    Ok(
        json!({"passed":true,"cases":observations,"scope":"eight_material_coupling_scenarios_four_meshes"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Value> {
        NAMES.iter().flat_map(|name| [1,2,4,8].map(|refinement| {
            let c = case(&Experiment { scenario: (*name).into(), refinement }).unwrap();
            json!({"case":c.id(),"job_id":c.id(),"tip_ux":c.displacement(1.0),"heat_flux_x":c.flux(),
                "expansion":c.expansion,"poisson":c.poisson,"physics_passed":true,
                "mechanical_validation":{"passed":true},"field_validation":{"passed":true}})
        })).collect()
    }

    #[test]
    fn hand_integrated_tip_references_and_all_material_identities_are_distinct() {
        let rows = rows();
        assert_eq!(verify(&rows).unwrap()["passed"], true);
        assert_eq!(
            rows.iter()
                .map(|r| r["case"].as_str().unwrap())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            32
        );
        let c = case(&Experiment {
            scenario: "coupled".into(),
            refinement: 2,
        })
        .unwrap();
        assert!(
            (c.expansion[0] / c.conductivity[0] - c.expansion[1] / c.conductivity[1]).abs() > 1e-8
        );
    }

    #[test]
    fn final_gate_rejects_missing_aliased_or_corrupted_evidence() {
        let good = rows();
        assert!(verify(&good[1..]).is_err());
        for (field, value) in [
            ("tip_ux", json!(0.0)),
            ("heat_flux_x", Value::Null),
            ("job_id", good[1]["job_id"].clone()),
            ("poisson", json!([0.3, 0.3])),
            ("physics_passed", json!(false)),
        ] {
            let mut bad = good.clone();
            bad[0][field] = value;
            assert!(verify(&bad).is_err(), "accepted {field}");
        }
    }
}
