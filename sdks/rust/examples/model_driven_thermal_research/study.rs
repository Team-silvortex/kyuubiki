use super::{
    checks, field_checks,
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
        || ![20.0, 293.15].contains(&reference)
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
        let c = case(d, reference, rise)?;
        if let Some(row) = self.rows.iter().find(|r| r["case"] == c.id()) {
            let mut cached = row.clone();
            cached["reused_retained_evidence"] = json!(true);
            return Ok(cached);
        }
        if self.jobs >= MAX_JOBS {
            return Err("workflow job budget reached".into());
        }
        let (graph, input) = c.workflow();
        let request_hash = retain(
            &self.root,
            &format!("{}-request.json", c.id()),
            &json!({"graph":graph,"input_artifacts":input,"origin":origin}),
        )?;
        // Count before dispatch; an ambiguous response never authorizes a retry.
        self.jobs += 1;
        eprintln!("workflow {}: {} ({origin})", self.jobs, c.id());
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
            &format!("{}-terminal.json", c.id()),
            &outcome.terminal,
        )?;
        let result = outcome.result.ok_or("workflow result missing")?;
        let digest = retain(&self.root, &format!("{}-result.json", c.id()), &result)?;
        if outcome.terminal["job"]["status"] != "completed" || outcome.validated_outputs.is_none() {
            return Err("workflow terminal/output contract failed".into());
        }
        let job_id = outcome.terminal["job"]["job_id"]
            .as_str()
            .ok_or("missing job identity")?;
        let readback = self
            .client
            .fetch_result(job_id)
            .map_err(|e| e.to_string())?;
        if readback != result {
            return Err("independent result readback mismatch".into());
        }
        let validation = checks::validate(c, &result)?;
        let field_validation = field_checks::validate(c, &result)?;
        let tamper_rejected = rejects_tampered_results(c, &result);
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
        if validation["passed"] != true || field_validation["passed"] != true || !tamper_rejected {
            return Err(format!(
                "physics validation failed for {}; evidence retained",
                c.id()
            ));
        }
        let (flux, tip) = metrics(&result)?;
        let feasibility = tip_feasibility(tip)?;
        let row = json!({"case":c.id(),"left":d.left,"right":d.right,"refinement":d.refinement,
            "reference":reference,"rise":rise,"origin":origin,"job_id":job_id,
            "heat_flux_x":flux,"tip_ux":tip,"feasible":feasibility == "feasible",
            "feasibility":feasibility,"tip_numerical_tolerance":TIP_TOLERANCE,
            "physics_passed":true,"tampered_evidence_rejected":tamper_rejected,
            "readback_matches":true,"request_sha256":request_hash,"result_sha256":digest,"validation":validation,
            "field_validation":field_validation});
        retain(&self.root, &format!("{}-evidence.json", c.id()), &row)?;
        self.rows.push(row.clone());
        Ok(row)
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
    }
}
