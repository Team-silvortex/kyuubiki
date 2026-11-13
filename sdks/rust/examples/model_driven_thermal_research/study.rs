use super::{
    checks, field_checks, mechanical_checks,
    model::Case,
    policy::{Design, GRID, MAX_JOBS, TIP_LIMIT, TIP_TOLERANCE, tip_feasibility},
    provider::retain,
};
use kyuubiki_headless_sdk::{ControlPlaneClient, KyuubikiAgentClient, KyuubikiSession};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const LABELS: [&str; 16] = [
    "k2-2", "k2-10", "k2-100", "k2-200", "k10-2", "k10-10", "k10-100", "k10-200", "k100-2",
    "k100-10", "k100-100", "k100-200", "k200-2", "k200-10", "k200-100", "k200-200",
];

pub fn case(d: Design, reference: f64, rise: f64) -> Result<Case, String> {
    let left = GRID
        .iter()
        .position(|v| *v == d.left)
        .ok_or("invalid left conductivity")?;
    let right = GRID
        .iter()
        .position(|v| *v == d.right)
        .ok_or("invalid right conductivity")?;
    if ![1, 2, 4, 8].contains(&d.refinement)
        || ![20.0, 30.0, 293.15].contains(&reference)
        || ![-20.0, 0.0, 20.0, 21.999, 22.0, 22.001, 40.0].contains(&rise)
    {
        return Err("unapproved verification case".into());
    }
    Ok(Case {
        material: LABELS[left * 4 + right],
        conductivity: [d.left, d.right],
        refinement: d.refinement,
        reference,
        rise,
        cold_offset: 0.0,
        expansion: super::model::ALPHA,
        poisson: [0.0; 2],
    })
}

pub fn oracle() -> Value {
    let mut best: Option<Case> = None;
    for left in GRID {
        for right in GRID {
            let c = case(
                Design {
                    left,
                    right,
                    refinement: 1,
                },
                20.0,
                20.0,
            )
            .unwrap();
            if c.displacement(1.0) <= TIP_LIMIT && best.is_none_or(|b| c.flux() > b.flux()) {
                best = Some(c);
            }
        }
    }
    let b = best.unwrap();
    json!({"left":b.conductivity[0],"right":b.conductivity[1],
        "heat_flux_x":b.flux(),"tip_ux":b.displacement(1.0),"grid_size":16})
}

fn payload(result: &Value) -> &Value {
    if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    }
}

fn metrics(result: &Value) -> Result<(f64, f64), String> {
    fn mean(values: Vec<f64>) -> Result<f64, String> {
        if values.is_empty() || values.iter().any(|x| !x.is_finite()) {
            return Err("invalid metric samples".into());
        }
        Ok(values.iter().sum::<f64>() / values.len() as f64)
    }
    let a = &payload(result)["artifacts"];
    let flux = a["heat_out.result"]["elements"]
        .as_array()
        .ok_or("missing heat metrics")?
        .iter()
        .map(|n| n["heat_flux_x"].as_f64().ok_or("invalid flux"))
        .collect::<Result<Vec<_>, _>>()?;
    let tip = a["structure_out.result"]["nodes"]
        .as_array()
        .ok_or("missing structural metrics")?
        .iter()
        .filter(|n| n["x"].as_f64().is_some_and(|x| (x - 1.0).abs() < 1e-12))
        .map(|n| n["ux"].as_f64().ok_or("invalid displacement"))
        .collect::<Result<Vec<_>, _>>()?;
    if tip.is_empty() || tip.iter().any(|v| !v.is_finite()) {
        return Err("invalid tip metric".into());
    }
    Ok((
        mean(flux)?,
        tip.into_iter().fold(f64::NEG_INFINITY, f64::max),
    ))
}

pub fn rejects_tampered_results(c: Case, result: &Value) -> bool {
    [
        ("heat_out.result", "temperature"),
        ("bridge_out.model", "temperature_delta"),
        ("structure_out.result", "ux"),
        ("heat_out.result", "x"),
    ]
    .iter()
    .all(|(artifact, field)| {
        let mut altered = payload(result).clone();
        altered["artifacts"][artifact]["nodes"][0][field] = json!(1e6);
        checks::validate(c, &altered).map_or(true, |v| v["passed"] != true)
    })
}

pub struct Study {
    client: ControlPlaneClient,
    agent: KyuubikiAgentClient,
    root: PathBuf,
    pub rows: Vec<Value>,
    pub jobs: usize,
}

struct Execution {
    result: Value,
    job_id: String,
    request_hash: String,
    digest: String,
}

impl Study {
    pub fn new(root: &Path) -> Result<Self, String> {
        let url = "http://127.0.0.1:4000";
        let client = ControlPlaneClient::new_with_auth(url, None).map_err(|e| e.to_string())?;
        let health = client.health().map_err(|e| e.to_string())?;
        retain(root, "service-health.json", &health)?;
        let session =
            KyuubikiSession::from_control_plane_with_auth(url, None).map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            agent: KyuubikiAgentClient::new(session),
            root: root.into(),
            rows: vec![],
            jobs: 0,
        })
    }

    pub fn evaluate(
        &mut self,
        d: Design,
        reference: f64,
        rise: f64,
        origin: &str,
    ) -> Result<Value, String> {
        self.evaluate_with_offset(d, reference, rise, 0.0, origin)
    }

    pub fn evaluate_with_offset(
        &mut self,
        d: Design,
        reference: f64,
        rise: f64,
        cold_offset: f64,
        origin: &str,
    ) -> Result<Value, String> {
        let c = case(d, reference, rise)?;
        if ![-10.0, 0.0, 10.0, 20.0].contains(&cold_offset) {
            return Err("unapproved cold-boundary offset".into());
        }
        self.evaluate_case(Case { cold_offset, ..c }, origin)
    }

    pub fn evaluate_case(&mut self, c: Case, origin: &str) -> Result<Value, String> {
        let d = Design {
            left: c.conductivity[0],
            right: c.conductivity[1],
            refinement: c.refinement,
        };
        let expected = case(d, c.reference, c.rise)?;
        if c.material != expected.material
            || ![-10.0, 0.0, 10.0, 20.0].contains(&c.cold_offset)
            || ![super::model::ALPHA, [10e-6, 35e-6], [18e-6, 6e-6]].contains(&c.expansion)
            || ![
                [0.0, 0.0],
                [0.3, 0.3],
                [0.2, 0.4],
                [-0.2, 0.3],
                [0.49, 0.25],
            ]
            .contains(&c.poisson)
        {
            return Err("unapproved material verification case".into());
        }
        if let Some(row) = self.rows.iter().find(|r| r["case"] == c.id()) {
            let mut cached = row.clone();
            cached["reused_retained_evidence"] = json!(true);
            return Ok(cached);
        }
        let (graph, input) = c.workflow();
        let execution = self.dispatch(&c.id(), graph, input, origin)?;
        let result = &execution.result;
        let validation = checks::validate(c, result)?;
        let field_validation = field_checks::validate(c, result)?;
        let mechanical_validation = mechanical_checks::validate(c, result)?;
        let tamper_rejected = rejects_tampered_results(c, result);
        retain(
            &self.root,
            &format!("{}-validation.json", c.id()),
            &validation,
        )?;
        retain(
            &self.root,
            &format!("{}-field-validation.json", c.id()),
            &field_validation,
        )?;
        retain(
            &self.root,
            &format!("{}-mechanical-validation.json", c.id()),
            &mechanical_validation,
        )?;
        if validation["passed"] != true
            || field_validation["passed"] != true
            || mechanical_validation["passed"] != true
            || !tamper_rejected
        {
            return Err(format!(
                "physics validation failed for {}; evidence retained",
                c.id()
            ));
        }
        let (flux, tip) = metrics(result)?;
        let feasibility = tip_feasibility(tip)?;
        let row = json!({"case":c.id(),"left":d.left,"right":d.right,"refinement":d.refinement,
            "reference":c.reference,"rise":c.rise,"cold_offset":c.cold_offset,
            "expansion":c.expansion,"poisson":c.poisson,"origin":origin,"job_id":execution.job_id,
            "heat_flux_x":flux,"tip_ux":tip,"feasible":feasibility == "feasible",
            "feasibility":feasibility,"tip_numerical_tolerance":TIP_TOLERANCE,
            "physics_passed":true,"tampered_evidence_rejected":tamper_rejected,
            "reused_retained_evidence":false,"readback_matches":true,"request_sha256":execution.request_hash,
            "result_sha256":execution.digest,"validation":validation,
            "field_validation":field_validation,"mechanical_validation":mechanical_validation});
        retain(&self.root, &format!("{}-evidence.json", c.id()), &row)?;
        self.rows.push(row.clone());
        Ok(row)
    }

    #[allow(dead_code)] // The design-selection example shares dispatch but does not run patches.
    pub fn evaluate_patch(
        &mut self,
        c: super::planar_patch::Case,
        origin: &str,
    ) -> Result<Value, String> {
        if c != super::planar_patch::Case::new(c.name, c.refinement)? {
            return Err("unapproved planar case".into());
        }
        if let Some(row) = self.rows.iter().find(|r| r["case"] == c.id()) {
            let mut row = row.clone();
            row["reused_retained_evidence"] = json!(true);
            return Ok(row);
        }
        let (graph, input) = c.workflow();
        let execution = self.dispatch(&c.id(), graph, input, origin)?;
        let validation = super::planar_checks::validate(c, &execution.result)?;
        retain(
            &self.root,
            &format!("{}-validation.json", c.id()),
            &validation,
        )?;
        if validation["passed"] != true {
            return Err(format!(
                "planar validation failed for {}; evidence retained",
                c.id()
            ));
        }
        let row = json!({"case":c.id(),"scenario":c.name,"refinement":c.refinement,"job_id":execution.job_id,
            "origin":origin,"reused_retained_evidence":false,"physics_passed":true,"readback_matches":true,
            "request_sha256":execution.request_hash,"result_sha256":execution.digest,"planar_validation":validation});
        retain(&self.root, &format!("{}-evidence.json", c.id()), &row)?;
        self.rows.push(row.clone());
        Ok(row)
    }

    #[allow(dead_code)]
    pub fn evaluate_bending(
        &mut self,
        c: super::bending_model::Case,
        origin: &str,
    ) -> Result<Value, String> {
        if c != super::bending_model::Case::new(c.name, c.refinement)? {
            return Err("unapproved bending case".into());
        }
        if let Some(row) = self.rows.iter().find(|r| r["case"] == c.id()) {
            let mut cached = row.clone();
            cached["reused_retained_evidence"] = json!(true);
            return Ok(cached);
        }
        let (graph, input) = c.workflow();
        let execution = self.dispatch(&c.id(), graph, input, origin)?;
        let validation = super::bending_checks::validate(c, &execution.result)?;
        retain(
            &self.root,
            &format!("{}-validation.json", c.id()),
            &validation,
        )?;
        if validation["passed"] != true {
            return Err(format!(
                "bending validation failed for {}; evidence retained",
                c.id()
            ));
        }
        let direct = if super::bending_model::ENERGY_NAMES.contains(&c.name) {
            let v = super::bending_direct::validate(c, &execution.result)?;
            retain(
                &self.root,
                &format!("{}-direct-validation.json", c.id()),
                &v,
            )?;
            if v["passed"] != true {
                return Err("independent bending direct comparison failed".into());
            }
            v
        } else {
            Value::Null
        };
        let row = json!({"case":c.id(),"scenario":c.name,"refinement":c.refinement,"job_id":execution.job_id,
            "origin":origin,"reused_retained_evidence":false,"physics_passed":true,"readback_matches":true,
            "request_sha256":execution.request_hash,"result_sha256":execution.digest,"bending_validation":validation,
            "direct_validation":direct});
        retain(&self.root, &format!("{}-evidence.json", c.id()), &row)?;
        self.rows.push(row.clone());
        Ok(row)
    }

    fn dispatch(
        &mut self,
        id: &str,
        graph: Value,
        input: Value,
        origin: &str,
    ) -> Result<Execution, String> {
        if self.jobs >= MAX_JOBS {
            return Err("workflow job budget reached".into());
        }
        let request_hash = retain(
            &self.root,
            &format!("{id}-request.json"),
            &json!({"graph":graph,"input_artifacts":input,"origin":origin}),
        )?;
        // Count before dispatch; an ambiguous response never authorizes a retry.
        self.jobs += 1;
        eprintln!("workflow {}: {id} ({origin})", self.jobs);
        let outcome = self
            .agent
            .run_workflow_graph(
                &graph,
                &input,
                Duration::from_millis(200),
                Duration::from_secs(120),
                true,
            )
            .map_err(|e| format!("workflow outcome unresolved; no retry: {e}"))?;
        retain(
            &self.root,
            &format!("{id}-terminal.json"),
            &outcome.terminal,
        )?;
        let result = outcome.result.ok_or("workflow result missing")?;
        let digest = retain(&self.root, &format!("{id}-result.json"), &result)?;
        if outcome.terminal["job"]["status"] != "completed" || outcome.validated_outputs.is_none() {
            return Err("workflow terminal/output contract failed".into());
        }
        let job_id = outcome.terminal["job"]["job_id"]
            .as_str()
            .ok_or("missing job identity")?
            .to_owned();
        let readback = self
            .client
            .fetch_result(&job_id)
            .map_err(|e| e.to_string())?;
        if readback != result {
            return Err("independent result readback mismatch".into());
        }
        Ok(Execution {
            result,
            job_id,
            request_hash,
            digest,
        })
    }

    pub fn verify_load_boundary(&mut self, winner: Design) -> Result<Value, String> {
        let mut observed = vec![];
        for (rise, expected) in [
            (21.999, "feasible"),
            (22.0, "boundary"),
            (22.001, "infeasible"),
        ] {
            let row = self.evaluate(winner, 20.0, rise, "load_boundary")?;
            if row["feasibility"] != expected {
                return Err("load-boundary classification failed".into());
            }
            observed.push(
                json!({"case":row["case"],"job_id":row["job_id"],"rise":rise,
                "tip_ux":row["tip_ux"],"feasibility":row["feasibility"]}),
            );
        }
        self.evaluate(winner, 20.0, -20.0, "negative_temperature_rise")?;
        let jobs: std::collections::BTreeSet<_> = observed
            .iter()
            .filter_map(|v| v["job_id"].as_str())
            .collect();
        if jobs.len() != 3 {
            return Err("distinct load cases reused an execution identity".into());
        }
        Ok(
            json!({"passed":true,"tip_limit":TIP_LIMIT,"numerical_tolerance":TIP_TOLERANCE,
            "boundary_is_not_approved_feasible":true,"cases":observed}),
        )
    }

    pub fn validate_selection(&self, selection: &Value) -> Result<Design, String> {
        let left = selection["left"].as_f64().ok_or("selection missing left")?;
        let right = selection["right"]
            .as_f64()
            .ok_or("selection missing right")?;
        let unique: std::collections::BTreeSet<_> = self
            .rows
            .iter()
            .filter(|r| r["origin"] == "model")
            .map(|r| format!("{}:{}", r["left"], r["right"]))
            .collect();
        if unique.len() < 4 {
            return Err("model must evaluate at least four different designs".into());
        }
        let observed = self.rows.iter().any(|r| {
            r["left"] == left
                && r["right"] == right
                && r["rise"] == 20.0
                && r["reference"] == 20.0
                && r["feasible"] == true
                && r["physics_passed"] == true
        });
        let best = oracle();
        if !observed || best["left"] != left || best["right"] != right {
            return Err("selection not observed, infeasible, or not the independently verified grid optimum".into());
        }
        Ok(Design {
            left,
            right,
            refinement: 8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearby_load_cases_cannot_reuse_the_same_cached_evidence() {
        let base = case(
            Design {
                left: 100.0,
                right: 200.0,
                refinement: 8,
            },
            20.0,
            20.0,
        )
        .unwrap();
        let ids: std::collections::BTreeSet<_> = [21.999, 22.0, 22.001]
            .iter()
            .map(|rise| {
                Case {
                    rise: *rise,
                    ..base
                }
                .id()
            })
            .collect();
        assert_eq!(
            ids.len(),
            3,
            "fractional loads must not alias after display rounding"
        );
    }

    #[test]
    fn exact_grid_oracle_rejects_the_naive_max_conductivity_choice() {
        let best = oracle();
        assert_eq!(
            (best["left"].as_f64(), best["right"].as_f64()),
            (Some(100.0), Some(200.0))
        );
        assert!((best["heat_flux_x"].as_f64().unwrap() - 2666.666666666667).abs() < 1e-8);
        assert!(
            case(
                Design {
                    left: 200.0,
                    right: 200.0,
                    refinement: 1
                },
                20.0,
                20.0
            )
            .unwrap()
            .displacement(1.0)
                > TIP_LIMIT
        );
    }

    #[test]
    fn verification_cases_have_distinct_ids_and_bounded_meshes() {
        let mut ids = std::collections::BTreeSet::new();
        for left in GRID {
            for right in GRID {
                assert!(
                    ids.insert(
                        case(
                            Design {
                                left,
                                right,
                                refinement: 2
                            },
                            20.0,
                            20.0
                        )
                        .unwrap()
                        .id()
                    )
                );
            }
        }
        assert!(
            case(
                Design {
                    left: 2.0,
                    right: 10.0,
                    refinement: 1000
                },
                20.0,
                20.0
            )
            .is_err()
        );
    }

    #[test]
    fn numerical_gates_catch_corrupted_artifacts_not_only_failed_jobs() {
        let c = case(
            Design {
                left: 100.0,
                right: 200.0,
                refinement: 1,
            },
            20.0,
            20.0,
        )
        .unwrap();
        let (mut heat, mut structure) = c.models();
        let mut bridge = structure.clone();
        for n in heat["nodes"].as_array_mut().unwrap() {
            n["temperature"] = json!(c.temperature(n["x"].as_f64().unwrap()));
        }
        for n in structure["nodes"].as_array_mut().unwrap() {
            n["ux"] = json!(c.displacement(n["x"].as_f64().unwrap()));
            n["uy"] = json!(0.0);
        }
        for n in bridge["nodes"].as_array_mut().unwrap() {
            n["temperature_delta"] = json!(c.temperature(n["x"].as_f64().unwrap()) - c.reference);
        }
        for e in heat["elements"].as_array_mut().unwrap() {
            e["heat_flux_x"] = json!(c.flux());
            e["heat_flux_y"] = json!(0.0);
        }
        let result = json!({"workflow_id":format!("research.{}",c.id()),"artifacts":{
            "heat_out.result":heat,"bridge_out.model":bridge,"structure_out.result":structure}});
        assert_eq!(checks::validate(c, &result).unwrap()["passed"], true);
        assert!(rejects_tampered_results(c, &result));
        let (flux, tip) = metrics(&result).unwrap();
        assert!((flux - c.flux()).abs() < 1e-8);
        assert!((tip - c.displacement(1.0)).abs() < 1e-12);
        let mut corrupted_constraint = result.clone();
        corrupted_constraint["artifacts"]["structure_out.result"]["nodes"][1]["uy"] = json!(0.1);
        assert_ne!(
            checks::validate(c, &corrupted_constraint).unwrap()["passed"],
            true,
            "a correct axial result must not hide violation of a transverse constraint"
        );
    }
}
