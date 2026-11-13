use super::{model::Case, policy::Design, study};
use serde::Deserialize;
use serde_json::{Value, json};

pub const SCENARIOS: [&str; 8] = [
    "baseline",
    "uniform_hot",
    "uniform_cold",
    "reference_only",
    "absolute_shift",
    "uniform_shifted",
    "reversed_gradient",
    "zero",
];

#[derive(Clone, Copy, PartialEq)]
pub enum Suite {
    Reference,
    Coupling,
    Planar,
    Bending,
    BendingEnergy,
}

impl Suite {
    pub fn names(self) -> &'static [&'static str] {
        match self {
            Self::Reference => &SCENARIOS,
            Self::Coupling => &super::coupling_scenarios::NAMES,
            Self::Planar => &super::planar_patch::NAMES,
            Self::Bending => &super::bending_model::NAMES,
            Self::BendingEnergy => &super::bending_model::ENERGY_NAMES,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Reference => "reference_decoupling",
            Self::Coupling => "material_coupling",
            Self::Planar => "planar_affine_patches",
            Self::Bending => "nonaffine_thermal_bending",
            Self::BendingEnergy => "bilayer_energy_diagnosis",
        }
    }
    pub fn refinements(self) -> &'static [usize] {
        if self == Self::BendingEnergy {
            &[1, 2, 4]
        } else {
            &[1, 2, 4, 8]
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    pub scenario: String,
    pub refinement: usize,
}

impl Experiment {
    pub fn design(&self) -> Design {
        Design {
            left: 100.0,
            right: 200.0,
            refinement: self.refinement,
        }
    }

    pub fn case(&self) -> Result<Case, String> {
        let (reference, rise, cold_offset) = match self.scenario.as_str() {
            "baseline" => (20.0, 20.0, 0.0),
            "uniform_hot" => (20.0, 0.0, 10.0),
            "uniform_cold" => (20.0, 0.0, -10.0),
            "reference_only" => (30.0, 20.0, -10.0),
            "absolute_shift" => (293.15, 20.0, 0.0),
            "uniform_shifted" => (293.15, 0.0, 10.0),
            "reversed_gradient" => (20.0, -20.0, 20.0),
            "zero" => (20.0, 0.0, 0.0),
            _ => return super::coupling_scenarios::case(self),
        };
        Ok(Case {
            cold_offset,
            ..study::case(self.design(), reference, rise)?
        })
    }
}

pub fn parse(suite: Suite, arguments: &str) -> Result<Vec<Experiment>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Batch {
        experiments: Vec<Experiment>,
    }
    if arguments.len() > 4096 {
        return Err("oversized experiment arguments".into());
    }
    let batch: Batch =
        serde_json::from_str(arguments).map_err(|_| "invalid experiment arguments")?;
    if !(1..=4).contains(&batch.experiments.len()) {
        return Err("batch must contain 1..4 experiments".into());
    }
    for e in &batch.experiments {
        if !suite.names().contains(&e.scenario.as_str()) {
            return Err("scenario outside selected suite approval".into());
        }
        if matches!(suite, Suite::Bending | Suite::BendingEnergy) {
            super::bending_model::Case::new(&e.scenario, e.refinement)?;
        } else if suite == Suite::Planar {
            super::planar_patch::Case::new(&e.scenario, e.refinement)?;
        } else {
            e.case()?;
        }
    }
    Ok(batch.experiments)
}

pub fn catalog(suite: Suite) -> Value {
    if suite == Suite::BendingEnergy {
        return super::bending_energy_suite::catalog();
    }
    if suite == Suite::Bending {
        return super::bending_suite::catalog();
    }
    if suite == Suite::Planar {
        return super::planar_patch::catalog();
    }
    json!(
        suite.names()
            .iter()
            .map(|name| {
                let e = Experiment {
                    scenario: (*name).into(),
                    refinement: 2,
                };
                let c = e.case().unwrap();
                json!({"scenario":name,"strain_reference":c.reference,"expansion":c.expansion,"poisson":c.poisson,
            "conductivity":c.conductivity,
            "left_temperature":c.temperature(0.0),"right_temperature":c.temperature(1.0)})
            })
            .collect::<Vec<_>>()
    )
}

pub fn coverage(suite: Suite) -> Vec<Experiment> {
    suite
        .names()
        .iter()
        .flat_map(|name| {
            let refinements: &[usize] = if suite != Suite::Reference
                || [
                    "baseline",
                    "uniform_hot",
                    "reference_only",
                    "reversed_gradient",
                ]
                .contains(name)
            {
                suite.refinements()
            } else {
                &[2]
            };
            refinements.iter().map(move |r| Experiment {
                scenario: (*name).into(),
                refinement: *r,
            })
        })
        .collect()
}

pub fn observation(scenario: &str, row: &Value) -> Value {
    if row.get("planar_validation").is_some() {
        return super::planar_patch::observation(row);
    }
    json!({"scenario":scenario,"case":row["case"],"refinement":row["refinement"],"job_id":row["job_id"],
        "expansion":row["expansion"],"poisson":row["poisson"],
        "heat_flux_x":row["heat_flux_x"],"tip_ux":row["tip_ux"],"feasibility":row["feasibility"],
        "reused_retained_evidence":row["reused_retained_evidence"],"physics_passed":row["physics_passed"],
        "mechanical_validation":row["mechanical_validation"],"field_validation":row["field_validation"]})
}

pub fn verify(suite: Suite, rows: &[Value]) -> Result<Value, String> {
    if suite == Suite::BendingEnergy {
        return super::bending_energy_suite::verify(rows);
    }
    if suite == Suite::Bending {
        return super::bending_suite::verify(rows);
    }
    if suite == Suite::Planar {
        return super::planar_patch::verify(rows);
    }
    if suite == Suite::Coupling {
        return super::coupling_scenarios::verify(rows);
    }
    let mut observed = vec![];
    for (name, tip, flux) in [
        ("baseline", 100e-6, 8000.0 / 3.0),
        ("uniform_hot", 150e-6, 0.0),
        ("uniform_cold", -150e-6, 0.0),
        ("reference_only", -50e-6, 8000.0 / 3.0),
        ("absolute_shift", 100e-6, 8000.0 / 3.0),
        ("uniform_shifted", 150e-6, 0.0),
        ("reversed_gradient", 200e-6, -8000.0 / 3.0),
        ("zero", 0.0, 0.0),
    ] {
        let case = Experiment {
            scenario: name.into(),
            refinement: 2,
        }
        .case()?;
        let row = rows
            .iter()
            .find(|r| r["case"] == case.id())
            .ok_or("missing required scenario")?;
        for (field, expected, tolerance) in [("tip_ux", tip, 1e-9), ("heat_flux_x", flux, 1e-5)] {
            let actual = row[field]
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or("invalid reference metric")?;
            if (actual - expected).abs() > tolerance {
                return Err(format!("{name}: independent {field} check failed"));
            }
        }
        if row["physics_passed"] != true || row["mechanical_validation"]["passed"] != true {
            return Err("scenario lacks mechanical evidence".into());
        }
        observed.push(observation(name, row));
    }
    let jobs: std::collections::BTreeSet<_> = observed
        .iter()
        .filter_map(|r| r["job_id"].as_str())
        .collect();
    if jobs.len() != SCENARIOS.len() {
        return Err("different thermal scenarios reused an execution identity".into());
    }
    Ok(
        json!({"passed":true,"cases":observed,"scope":"eight_independent_temperature_reference_scenarios"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_suite_has_twelve_bounded_cases_and_no_unapproved_refinement() {
        let suite = Suite::BendingEnergy;
        assert_eq!(coverage(suite).len(), 12);
        assert_eq!(suite.refinements(), &[1, 2, 4]);
        assert!(
            parse(
                suite,
                r#"{"experiments":[{"scenario":"energy_x","refinement":4}]}"#
            )
            .is_ok()
        );
        assert!(
            parse(
                suite,
                r#"{"experiments":[{"scenario":"energy_x","refinement":8}]}"#
            )
            .is_err()
        );
        assert!(
            parse(
                Suite::Bending,
                r#"{"experiments":[{"scenario":"energy_x","refinement":2}]}"#
            )
            .is_err()
        );
    }
    #[test]
    fn temperature_boundaries_and_strain_reference_are_independent() {
        let nominal = Experiment {
            scenario: "baseline".into(),
            refinement: 2,
        }
        .case()
        .unwrap();
        let reference_only = Experiment {
            scenario: "reference_only".into(),
            refinement: 2,
        }
        .case()
        .unwrap();
        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((nominal.temperature(x) - reference_only.temperature(x)).abs() < 1e-12);
        }
        assert!(
            (nominal.displacement(1.0) - reference_only.displacement(1.0) - 150e-6).abs() < 1e-14
        );
        let ids: std::collections::BTreeSet<_> = coverage(Suite::Reference)
            .iter()
            .map(|e| e.case().unwrap().id())
            .collect();
        assert_eq!(ids.len(), 20);
    }
    #[test]
    fn unapproved_scenarios_meshes_and_fields_fail_before_execution() {
        assert!(
            parse(
                Suite::Reference,
                r#"{"experiments":[{"scenario":"uniform_hot","refinement":2}]}"#
            )
            .is_ok()
        );
        for args in [
            r#"{"experiments":[]}"#,
            r#"{"experiments":[{"scenario":"shell","refinement":2}]}"#,
            r#"{"experiments":[{"scenario":"uniform_hot","refinement":200}]}"#,
            r#"{"experiments":[{"scenario":"uniform_hot","refinement":2,"url":"https://example.com"}]}"#,
        ] {
            assert!(parse(Suite::Reference, args).is_err());
        }
    }

    #[test]
    fn suite_scope_is_validated_for_the_entire_batch() {
        let mixed = r#"{"experiments":[{"scenario":"coupled","refinement":2},{"scenario":"uniform_hot","refinement":2}]}"#;
        assert!(parse(Suite::Reference, mixed).is_err());
        assert!(parse(Suite::Coupling, mixed).is_err());
        assert_eq!(coverage(Suite::Coupling).len(), 32);
        assert_eq!(coverage(Suite::Planar).len(), 32);
        assert!(parse(Suite::Planar, mixed).is_err());
        assert!(
            parse(
                Suite::Planar,
                r#"{"experiments":[{"scenario":"shear","refinement":8}]}"#
            )
            .is_ok()
        );
    }
}
