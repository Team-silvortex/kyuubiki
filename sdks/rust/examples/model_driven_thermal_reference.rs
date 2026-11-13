//! Opt-in follow-up: model-selected reference-temperature experiments on real Agents.
#[path = "model_driven_thermal_research/bending_checks.rs"]
#[allow(dead_code)]
mod bending_checks;
#[path = "model_driven_thermal_research/bending_direct.rs"]
#[allow(dead_code)]
mod bending_direct;
#[path = "model_driven_thermal_research/bending_energy_suite.rs"]
mod bending_energy_suite;
#[path = "model_driven_thermal_research/bending_model.rs"]
#[allow(dead_code)]
mod bending_model;
#[path = "model_driven_thermal_research/bending_order.rs"]
mod bending_order;
#[path = "model_driven_thermal_research/bending_q9.rs"]
#[allow(dead_code)]
mod bending_q9;
#[path = "model_driven_thermal_research/bending_suite.rs"]
#[allow(dead_code)]
mod bending_suite;
#[path = "layered_thermal_research/checks.rs"]
mod checks;
#[path = "model_driven_thermal_research/coupling_scenarios.rs"]
mod coupling_scenarios;
#[path = "model_driven_thermal_research/field_checks.rs"]
mod field_checks;
#[path = "model_driven_thermal_research/mechanical_checks.rs"]
mod mechanical_checks;
#[path = "layered_thermal_research/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "model_driven_thermal_research/planar_checks.rs"]
#[allow(dead_code)]
mod planar_checks;
#[path = "model_driven_thermal_research/planar_patch.rs"]
#[allow(dead_code)]
mod planar_patch;
#[path = "model_driven_thermal_research/policy.rs"]
#[allow(dead_code)]
mod policy;
#[path = "model_driven_thermal_research/provider.rs"]
mod provider;
#[path = "model_driven_thermal_research/reference_scenarios.rs"]
mod scenarios;
#[path = "model_driven_thermal_research/study.rs"]
#[allow(dead_code)]
mod study;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::Path};

const BRIEF: &str = "Investigate deeper bugs in a SYNTHETIC Kyuubiki heat-to-thermoelastic workflow. \
Use only evaluate_cases; its approved scenarios vary boundary temperatures INDEPENDENTLY of strain reference. \
Geometry: x in [0,1] m, y in [0,0.1] m, thickness 0.01 m; two equal x-layers, conductivities [100,200] W/(m K), \
thermal expansion [10e-6,20e-6]/K, Young moduli [70e9,3e9] Pa, Poisson ratio zero. \
All y displacement is restrained; x fixed only at left, right axially free; zero mechanical loads and heat sources. \
Sides insulated. Operators: steady Q4 heat -> node-to-node temperature/reference bridge -> plane-stress thermoelastic Q4. \
Choose and execute at least FOUR different scenarios, observe actual tool results before further experiments. \
Test competing hypotheses, not just nominal displacement. Batches 1..4, refinements [1,2,4,8], at most 10 turns. \
Caller completes a fixed 20-case minimum matrix and decides numerical gates, independently of your opinion. \
Returned element stresses are means, not pointwise peaks. Reactions are independently reconstructed, NOT solver-returned fields. \
The 110 micrometre expansion feasibility limit is one-sided and does not certify contraction or stress safety. \
Finish with ONLY JSON {\"findings\":[string],\"limitations\":[string],\"next_experiments\":[string]}. \
Do not invent results or claim broad solver qualification. You cannot execute code, change budgets or grant approval.";

const COUPLING_BRIEF: &str = "Investigate nonzero Poisson coupling and unequal thermal-expansion/conductivity ratios in a SYNTHETIC Kyuubiki workflow. \
Use only evaluate_cases with the approved material catalog. Geometry: two equal x-layers of a 1 x 0.1 m strip, thickness 0.01 m, conductivities [100,200] W/(m K), Young moduli [70e9,3e9] Pa. \
Steady Q4 heat -> node-to-node temperature/reference bridge -> plane-STRESS thermoelastic Q4, zero mechanical loads and heat sources, insulated sides. \
ALL y displacement is restrained; x fixed only at left, right axially free. Boundary temperatures, strain reference, alpha and nu are given by the catalog. \
Choose and execute at least FOUR distinct scenarios; use observations to test hypotheses of ignored nu, wrong material assignment, hidden curvature and incorrect energy. \
Batches 1..4, refinements [1,2,4,8], at most 10 turns. Caller completes all 32 scenario/mesh combinations and owns numerical gates. \
Reaction resultants and residuals are independently reconstructed, NOT solver-returned reactions; element stresses are MEANS, not peaks. \
The inherited 110 micrometre expansion classifier is one-sided, not a stress or contraction safety criterion, and is not an optimization objective here. \
A nu of 0.49 in this PLANE-STRESS study does not qualify a nearly incompressible 3D solver. No general shear or transverse-free expansion claims. \
Finish with ONLY JSON {\"findings\":[string],\"limitations\":[string],\"next_experiments\":[string]}. \
Do not invent observations, execute code, change budgets or grant approval.";

const PLANAR_BRIEF: &str = "Investigate transverse-free Poisson coupling, engineering shear and distorted Q4 elements in a SYNTHETIC workflow. \
Only evaluate_cases is allowed; scenario parameters are fixed by the catalog. Homogeneous 1 x 0.25 m rectangle, thickness 0.01 m, E=70e9 Pa, alpha=12e-6/K, k=20 W/(m K), plane STRESS. \
Heat Q4 -> node-to-node temperature bridge -> thermoelastic Q4. Uniform boundary temperature; interior temperature DOFs are solved. \
Only origin ux/uy and bottom-right uy are fixed: exactly THREE rigid-mode restraints, not an all-uy clamp. \
Mechanical loads are consistent boundary nodal tractions t=sigma*n on ALL outer edges, with the catalog constant stress [sigma_x,sigma_y,tau_xy]. \
Distorted cases perturb interior nodes, preserving the rectangular boundary and positive Jacobians. Refinement r has (2r+1)^2 nodes. \
Choose at least FOUR scenarios through observation-driven batches of 1..4, refinements [1,2,4,8], max 10 turns. Caller completes 32 cases and owns all gates. \
Test hypotheses: unwanted transverse restraints, ignored Poisson contraction, tensor-vs-engineering shear factor, distorted Jacobian errors, wrong energy or stress invariants. \
These are AFFINE displacement/constant-stress patch tests. Do not claim non-affine convergence, nonlinear behavior, heterogeneous free-expansion validity, or general stress-peak qualification. \
Force residuals are independently reconstructed, NOT solver-returned reactions. There is no expansion-limit feasibility objective in this suite. \
Observed stress/invariant fields report element minima/maxima and errors separately from expected values. \
Traction-displacement pairing is f^T*u, NOT loading-path work. Its thermal and thermal-subtracted elastic parts are separate; only the elastic part equals twice strain energy. \
Finish with ONLY JSON {\"findings\":[string],\"limitations\":[string],\"next_experiments\":[string]}. Do not execute code, change budgets or grant approval.";

const BENDING_BRIEF: &str = "Investigate NON-AFFINE thermal bending and material jumps, not constant-stress patches. \
Homogeneous free cases: 1 x 0.25 m rectangle, thickness .01 m, E=70 GPa, nu=.3, alpha=12e-6/K, k=20. \
Bilayer cases have two equal layers along y, with properties and temperatures in the catalog. Heat has bottom/top Dirichlet boundaries and insulated x sides. \
Heat Q4 -> node-to-node temperature bridge -> plane-stress thermoelastic Q4. Only origin ux/uy and bottom-right uy fixed (three rigid modes). \
IMPORTANT: homogeneous gradient_free cases have ZERO mechanical loads; bilayer cases use analytical SELF-EQUILIBRATED END TRACTIONS sigma_x(y)*n_x on both ends. \
They are NOT fully traction-free bilayer strips or clamped cantilevers. Exact solution: ex=e0+kappa*y, sigma_x=E*(ex-alpha*theta), sy=tau=0. \
e0,kappa follow zero section axial force and moment. uy integrates ey=alpha*theta-nu*sigma_x/E plus kappa*x*(1-x)/2. \
Displacement is piecewise quadratic: Q4 is NOT exact, finite energy/stress/displacement errors must converge. Refinements [1,2,4,8] use (4r+1)*(2r+1) nodes. \
Only evaluate_cases with approved names. Test at least FOUR scenarios in batches 1..4, at most 10 turns. Caller completes all SIX scenarios at four levels (24 real cases). \
Gates distinguish exact heat/bridge and reconstructed equilibrium/postprocessing from continuum discretization error. \
Caller requires error reduction by at least 20% per mesh doubling; finest relative nodal RMS and bow error <=2%, normalized energy error <=.001 and normalized stress L2 <=.1. \
These are catalog acceptance limits, not universal error bounds. Force residuals are reconstructed, not solver-returned reactions. No nonlinear, transient, arbitrary-mesh or general free-bilayer qualification. \
Each observation gives the normalization definitions/scales. Error-energy is a quadratic strain-error measure, not the relative total elastic-energy error; the latter is null when the continuum energy is zero. \
Constitutive balance uses 2U=f^T*u-integral(sigma:thermal_strain); f^T*u is final traction-displacement pairing, NOT path-integrated work. \
Finish ONLY JSON {\"findings\":[string],\"limitations\":[string],\"next_experiments\":[string]}. Do not invent data, run code or alter gates/budgets.";

fn tools(suite: scenarios::Suite) -> Value {
    json!([{"type":"function","name":"evaluate_cases","strict":true,
        "description":"Run bounded real SDK heat/bridge/structure scenarios. Returns validated observations; no arbitrary actions.",
        "parameters":{"type":"object","additionalProperties":false,"required":["experiments"],
            "properties":{"experiments":{"type":"array","minItems":1,"maxItems":4,
                "items":{"type":"object","additionalProperties":false,"required":["scenario","refinement"],
                    "properties":{"scenario":{"type":"string","enum":suite.names()},
                        "refinement":{"type":"integer","enum":suite.refinements()}}}}}}}])
}

fn evaluate(
    study: &mut study::Study,
    e: &scenarios::Experiment,
    origin: &str,
    suite: scenarios::Suite,
) -> Result<Value, String> {
    if matches!(
        suite,
        scenarios::Suite::Bending | scenarios::Suite::BendingEnergy
    ) {
        let c = bending_model::Case::new(&e.scenario, e.refinement)?;
        return Ok(bending_suite::observation(
            &study.evaluate_bending(c, origin)?,
        ));
    }
    if suite == scenarios::Suite::Planar {
        let c = planar_patch::Case::new(&e.scenario, e.refinement)?;
        return Ok(planar_patch::observation(&study.evaluate_patch(c, origin)?));
    }
    let c = e.case()?;
    let row = study.evaluate_case(c, origin)?;
    Ok(scenarios::observation(&e.scenario, &row))
}

fn run(root: &Path, key: &Path, suite: scenarios::Suite) -> Result<(), String> {
    let brief = match suite {
        scenarios::Suite::Coupling => COUPLING_BRIEF,
        scenarios::Suite::Reference => BRIEF,
        scenarios::Suite::Planar => PLANAR_BRIEF,
        scenarios::Suite::Bending => BENDING_BRIEF,
        scenarios::Suite::BendingEnergy => bending_energy_suite::BRIEF,
    };
    let definition = concat!(
        include_str!("model_driven_thermal_reference.rs"),
        include_str!("model_driven_thermal_research/reference_scenarios.rs"),
        include_str!("model_driven_thermal_research/bending_model.rs"),
        include_str!("model_driven_thermal_research/bending_direct.rs"),
        include_str!("model_driven_thermal_research/bending_energy_suite.rs"),
        include_str!("model_driven_thermal_research/bending_checks.rs"),
        include_str!("model_driven_thermal_research/bending_tests.rs"),
        include_str!("model_driven_thermal_research/bending_suite.rs"),
        include_str!("model_driven_thermal_research/coupling_scenarios.rs"),
        include_str!("model_driven_thermal_research/planar_patch.rs"),
        include_str!("model_driven_thermal_research/planar_checks.rs"),
        include_str!("model_driven_thermal_research/planar_tests.rs"),
        include_str!("model_driven_thermal_research/mechanical_checks.rs"),
        include_str!("model_driven_thermal_research/policy.rs"),
        include_str!("model_driven_thermal_research/provider.rs"),
        include_str!("model_driven_thermal_research/study.rs"),
        include_str!("model_driven_thermal_research/field_checks.rs"),
        include_str!("layered_thermal_research/model.rs"),
        include_str!("layered_thermal_research/checks.rs")
    );
    provider::retain(
        root,
        "approval.json",
        &json!({"schema_version":"kyuubiki.model-research-example/v1",
        "scope":"caller_opt_in_local_synthetic","suite":suite.name(),"brief":brief,"scenarios":scenarios::catalog(suite),
        "definition_sha256":format!("{:x}",Sha256::digest(definition.as_bytes())),"sdk_version":env!("CARGO_PKG_VERSION"),
        "max_estimated_usd":policy::BUDGET_USD,"max_model_calls":policy::MAX_CALLS,"max_jobs":policy::MAX_JOBS,
        "automatic_retry":false,"model_granted_authority":false,"production_qualification":false}),
    )?;
    let mut study = study::Study::new(root)?;
    let mut api = provider::Provider::new(root, key)?;
    let mut conversation = vec![
        json!({"role":"developer","content":brief}),
        json!({"role":"user","content":format!("Approved case catalog: {}. Begin testing your hypotheses.",scenarios::catalog(suite))}),
    ];
    let mut call_ids = BTreeSet::new();
    let mut tested = BTreeSet::new();
    let mut model_findings = None;
    let mut tool_audit = vec![];
    for turn in 1..=10 {
        let output = api.request("gpt-6.1-sol", &conversation, tools(suite))?;
        let calls: Vec<_> = output
            .iter()
            .filter(|v| v["type"] == "function_call")
            .collect();
        if calls.is_empty() {
            let findings: Value = serde_json::from_str(&policy::output_text(&output)?)
                .map_err(|_| "model findings are not JSON")?;
            if tested.len() < 4 {
                return Err("model did not test four distinct scenarios".into());
            }
            for key in ["findings", "limitations", "next_experiments"] {
                if !findings[key]
                    .as_array()
                    .is_some_and(|v| !v.is_empty() && v.iter().all(Value::is_string))
                {
                    return Err("model findings contract failed".into());
                }
            }
            provider::retain(root, "model-findings.json", &findings)?;
            model_findings = Some(findings);
            break;
        }
        if calls.len() != 1 || calls[0]["name"] != "evaluate_cases" {
            return Err("unapproved or parallel tool call".into());
        }
        let call = calls[0];
        let id = call["call_id"]
            .as_str()
            .ok_or("missing call identity")?
            .to_owned();
        if !call_ids.insert(id.clone()) {
            return Err("duplicate tool call; no replay".into());
        }
        let batch = scenarios::parse(
            suite,
            call["arguments"].as_str().ok_or("missing tool arguments")?,
        )?;
        provider::retain(root, &format!("tool-{turn:02}-request.json"), call)?;
        let requested: Vec<_> = batch
            .iter()
            .map(|e| json!({"scenario":e.scenario,"refinement":e.refinement}))
            .collect();
        let mut observations = vec![];
        for experiment in batch {
            observations.push(evaluate(
                &mut study,
                &experiment,
                "model_reference_experiment",
                suite,
            )?);
            tested.insert(experiment.scenario);
        }
        provider::retain(
            root,
            &format!("tool-{turn:02}-result.json"),
            &json!(observations),
        )?;
        tool_audit.push(json!({"turn":turn,"call_id":id,"requested":requested,
            "observed":observations.iter().map(|v|json!({"case":v["case"],"job_id":v["job_id"],
                "reused_retained_evidence":v["reused_retained_evidence"]})).collect::<Vec<_>>()}));
        conversation.extend(output);
        conversation.push(json!({"type":"function_call_output","call_id":id,
            "output":serde_json::to_string(&observations).map_err(|_|"observation encoding failed")?}));
    }
    let findings = model_findings.ok_or("model turn budget exhausted")?;
    let mut coverage = vec![];
    for experiment in scenarios::coverage(suite) {
        let observed = evaluate(&mut study, &experiment, "caller_reference_matrix", suite)?;
        coverage.push(
            json!({"scenario":experiment.scenario,"refinement":experiment.refinement,
            "case":observed["case"],"job_id":observed["job_id"],
            "reused_retained_evidence":observed["reused_retained_evidence"]}),
        );
    }
    let reference = scenarios::verify(suite, &study.rows)?;
    let evidence = json!({"numerical_gates_passed":true,"reference_checks":reference,
        "minimum_coverage":coverage,"jobs":study.jobs,"model_findings":findings,"tool_audit":tool_audit,
        "qualification":"synthetic_reference_only","suite":suite.name(),"sdk_version":env!("CARGO_PKG_VERSION")});
    provider::retain(root, "numerical-report.json", &evidence)?;
    let review_output = api.request("gpt-6-astra",&[
        json!({"role":"developer","content":"Independently critique physics, numerical validation and automation scope, not prose. Reactions are independently reconstructed, not returned by the solver. Distinguish exact nodal values, discrete energy and continuum/pointwise errors. You cannot override gates. Return ONLY JSON {\"scope_supported\":boolean,\"blocking_issues\":[string],\"limitations\":[string],\"next_experiments\":[string]}."}),
        json!({"role":"user","content":format!("{brief}\nEvidence: {evidence}")})],json!([]))?;
    let review: Value = serde_json::from_str(&policy::output_text(&review_output)?)
        .map_err(|_| "review is not JSON")?;
    provider::retain(root, "independent-model-review.json", &review)?;
    let accepted = review["scope_supported"] == true
        && review["blocking_issues"]
            .as_array()
            .is_some_and(Vec::is_empty);
    provider::retain(
        root,
        "report.json",
        &json!({"complete":true,"numerical_gates_passed":true,
        "independent_review_accepted":accepted,"jobs":study.jobs,"cases":study.rows,"evidence":evidence,
        "review":review,"usage":api.usage,"estimated_standard_usd":api.budget.charged,
        "budget_usd":policy::BUDGET_USD,"qualification":"synthetic_reference_only"}),
    )?;
    println!(
        "workflows={} model_calls={} estimated_usd={:.6} numerical_pass=true review_accepted={accepted}",
        study.jobs, api.budget.calls, api.budget.charged
    );
    if !accepted {
        return Err("independent review raised issues; evidence retained".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("--offline-bending-order") {
        let args: Vec<_> = env::args().skip(2).collect();
        if args.len() != 1 {
            return Err("usage: --offline-bending-order <new-output-directory>".into());
        }
        let root = Path::new(&args[0]);
        fs::create_dir(root)?;
        return bending_order::run(root).map_err(Into::into);
    }
    if env::var("KYUUBIKI_LIVE_RESEARCH").as_deref() != Ok("1") {
        return Err("paid research requires KYUUBIKI_LIVE_RESEARCH=1".into());
    }
    let mut args = env::args().skip(1);
    let out = args.next().ok_or(
        "usage: model_driven_thermal_reference <new-output-directory> [coupling|planar|bending|bending-energy]",
    )?;
    let suite = match args.next().as_deref() {
        None => scenarios::Suite::Reference,
        Some("coupling") => scenarios::Suite::Coupling,
        Some("planar") => scenarios::Suite::Planar,
        Some("bending") => scenarios::Suite::Bending,
        Some("bending-energy") => scenarios::Suite::BendingEnergy,
        _ => return Err("unknown experiment suite".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let key =
        env::var("KYUUBIKI_MODEL_API_KEY_FILE").map_err(|_| "set external credential file path")?;
    let root = Path::new(&out);
    fs::create_dir(root)?;
    if let Err(error) = run(root, Path::new(&key), suite) {
        let _ = provider::retain(
            root,
            "failure.json",
            &json!({"complete":false,"error":error,"automatic_retry":false}),
        );
        return Err(error.into());
    }
    Ok(())
}
