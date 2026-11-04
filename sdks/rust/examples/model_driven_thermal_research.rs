//! Opt-in, bounded model-driven research using the official Rust headless SDK.
#[path = "layered_thermal_research/checks.rs"]
mod checks;
#[path = "model_driven_thermal_research/field_checks.rs"]
mod field_checks;
#[allow(dead_code)]
#[path = "layered_thermal_research/model.rs"]
mod model;
#[path = "model_driven_thermal_research/policy.rs"]
mod policy;
#[path = "model_driven_thermal_research/provider.rs"]
mod provider;
#[path = "model_driven_thermal_research/study.rs"]
mod study;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};

const BRIEF: &str = "You are running a bounded SYNTHETIC engineering experiment using Kyuubiki's official Rust headless SDK. \
Choose conductivities (W/m/K) for two bonded equal-length layers along x in a 1 m by 0.1 m strip, thickness 0.01 m. \
Each layer's conductivity is independently one of [2,10,100,200]. Thermal expansion coefficients are [10e-6,20e-6] /K, \
Young's moduli [70e9,3e9] Pa. Poisson ratio is zero, all y displacement restrained, x fixed only at left. \
Steady heat: left boundary 40, right 20, insulated sides, zero heat sources. Thermal strain reference is 20. \
The tool executes heat Q4 -> temperature-field bridge -> thermoelastic Q4 with actual Agents, validates against analytic fields, \
and reads back retained results. Objective: MAXIMIZE heat_flux_x subject to tip_ux <= 110e-6 m. \
Choose at least four DISTINCT conductivity pairs using evaluate_designs, observing results before deciding further experiments. \
Prefer refinement 2 for exploration; the caller independently verifies the winner at refinements 1,2,4,8 and temperature transformations. \
Do not invent results, assume highest conductivity is feasible, or claim these synthetic coefficients are certified real materials. \
At most 24 exploration jobs; batches 1..4. Finish within 10 model turns. \
Finish with ONLY a JSON object {\"left\":number,\"right\":number,\"rationale\":string,\"limitations\":[string]} \
selecting an evaluated feasible design. No markdown fences. The caller, not you, decides whether numerical gates pass.";

fn tools() -> Value {
    json!([{"type":"function","name":"evaluate_designs",
        "description":"Run up to four approved synthetic layered heat-to-structure designs. Returns real SDK evidence and independently checked metrics. No arbitrary code, files, network, materials, or approval changes.",
        "strict":true,"parameters":{"type":"object","additionalProperties":false,
            "properties":{"designs":{"type":"array","minItems":1,"maxItems":4,"items":{
                "type":"object","additionalProperties":false,"properties":{
                    "left":{"type":"number","enum":policy::GRID},
                    "right":{"type":"number","enum":policy::GRID},
                    "refinement":{"type":"integer","enum":[1,2,4,8]}},
                "required":["left","right","refinement"]}}},"required":["designs"]}}])
}

fn run(root: &Path, key: &Path) -> Result<(), String> {
    let definition = concat!(
        include_str!("model_driven_thermal_research.rs"),
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
        "scope":"caller_opt_in_synthetic_local_layered_thermal_only","brief":BRIEF,
        "sdk_version":env!("CARGO_PKG_VERSION"),"definition_sha256":format!("{:x}",Sha256::digest(definition.as_bytes())),
        "max_estimated_usd":policy::BUDGET_USD,"max_calls":policy::MAX_CALLS,"max_jobs":policy::MAX_JOBS,
        "model_granted_authority":false,"automatic_retry":false,"production_qualification":false}),
    )?;
    let mut study = study::Study::new(root)?;
    let mut provider = provider::Provider::new(root, key)?;
    let mut conversation = vec![
        json!({"role":"developer","content":BRIEF}),
        json!({"role":"user","content":"Begin the experiment. Use real tool evidence, compare alternatives, and choose a constrained optimum."}),
    ];
    let mut selection = None;
    let mut exploration_jobs = 0;
    let mut seen_calls = std::collections::BTreeSet::new();
    for turn in 1..=10 {
        let output = provider.request("gpt-6.1-sol", &conversation, tools())?;
        let calls: Vec<_> = output
            .iter()
            .filter(|v| v["type"] == "function_call")
            .collect();
        if calls.is_empty() {
            let text = policy::output_text(&output)?;
            let final_value: Value =
                serde_json::from_str(&text).map_err(|_| "planner final output is not JSON")?;
            provider::retain(root, "model-selection.json", &final_value)?;
            selection = Some(final_value);
            break;
        }
        if calls.len() != 1 {
            return Err("parallel tool calls are outside this experiment's approval".into());
        }
        let call = calls[0];
        if call["name"] != "evaluate_designs" {
            return Err("model requested an unapproved tool".into());
        }
        let call_id = call["call_id"]
            .as_str()
            .ok_or("missing model call identity")?
            .to_owned();
        if !seen_calls.insert(call_id.clone()) {
            return Err("duplicate model call identity; no replay".into());
        }
        let arguments = call["arguments"].as_str().ok_or("missing tool arguments")?;
        let batch = policy::designs(arguments)?;
        if exploration_jobs + batch.len() > 24 {
            return Err("exploration job limit reached".into());
        }
        provider::retain(
            root,
            &format!("tool-{turn:02}-request.json"),
            &json!({"call_id":call_id,"name":"evaluate_designs","arguments":serde_json::from_str::<Value>(arguments).map_err(|_| "invalid arguments")?}),
        )?;
        let mut rows = vec![];
        for d in batch {
            exploration_jobs += 1;
            rows.push(study.evaluate(d, 20.0, 20.0, "model")?);
        }
        provider::retain(root, &format!("tool-{turn:02}-result.json"), &json!(rows))?;
        // Stateless Responses continuation: reasoning envelopes stay in memory, not evidence files.
        conversation.extend(output);
        conversation.push(json!({"type":"function_call_output","call_id":call_id,"output":serde_json::to_string(&rows).map_err(|_| "tool output encoding failed")?}));
    }
    let selection = selection.ok_or("planner turn limit reached without a selection")?;
    let winner = study.validate_selection(&selection)?;
    for refinement in [1, 2, 4, 8] {
        study.evaluate(
            policy::Design {
                refinement,
                ..winner
            },
            20.0,
            20.0,
            "independent_verification",
        )?;
    }
    for (reference, rise) in [(293.15, 20.0), (20.0, 0.0), (20.0, 40.0)] {
        study.evaluate(winner, reference, rise, "independent_verification")?;
    }
    let boundary = study.verify_load_boundary(winner)?;
    provider::retain(root, "load-boundary-report.json", &boundary)?;
    let evidence = json!({"scope":"synthetic_constrained_layered_thermal_optimization_not_material_certification",
        "selection":selection,"independent_grid_oracle":study::oracle(),"cases":study.rows,
        "load_boundary":boundary,"numerical_gates_passed":true,"jobs":study.jobs,"sdk_version":env!("CARGO_PKG_VERSION")});
    provider::retain(root, "numerical-report.json", &evidence)?;
    let review = provider.request("gpt-6-astra", &[
        json!({"role":"developer","content":"Independently critique this synthetic engineering experiment, not its writing. Identify invalid physics assumptions, misleading optimization claims, missing checks and unsafe automation. You cannot authorize execution or override deterministic gates. Return ONLY JSON {\"scope_supported\":boolean,\"blocking_issues\":[string],\"limitations\":[string],\"next_experiments\":[string]}. Do not claim general material certification or broad solver qualification."}),
        json!({"role":"user","content":format!("Problem:\n{BRIEF}\nRetained deterministic evidence:\n{}", evidence)})
    ], json!([]))?;
    let review: Value = serde_json::from_str(&policy::output_text(&review)?)
        .map_err(|_| "review output is not JSON")?;
    provider::retain(root, "independent-model-review.json", &review)?;
    let review_accepted = review["scope_supported"] == true
        && review["blocking_issues"]
            .as_array()
            .is_some_and(Vec::is_empty);
    let report = json!({"schema_version":"kyuubiki.model-research-example/v1","complete":true,
        "numerical_gates_passed":true,"independent_review_accepted":review_accepted,
        "qualification":"synthetic_reference_only","jobs":study.jobs,"cases":study.rows,"load_boundary":boundary,
        "selection":selection,"review":review,"usage":provider.usage,
        "estimated_standard_usd":provider.budget.charged,"budget_usd":policy::BUDGET_USD});
    provider::retain(root, "report.json", &report)?;
    println!(
        "workflows={} model_calls={} estimated_usd={:.6} numerical_pass=true review_accepted={review_accepted}",
        study.jobs, provider.budget.calls, provider.budget.charged
    );
    if !review_accepted {
        return Err("independent review raised issues; report retained for human review".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("KYUUBIKI_LIVE_RESEARCH").as_deref() != Ok("1") {
        return Err("paid research is opt-in: set KYUUBIKI_LIVE_RESEARCH=1".into());
    }
    let mut args = env::args().skip(1);
    let out = args
        .next()
        .ok_or("usage: model_driven_thermal_research <new-output-directory>")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let key = env::var("KYUUBIKI_MODEL_API_KEY_FILE")
        .map_err(|_| "set the external credential file path")?;
    let root = Path::new(&out);
    fs::create_dir(root)?;
    if let Err(error) = run(root, Path::new(&key)) {
        let _ = provider::retain(
            root,
            "failure.json",
            &json!({"complete":false,"error":error,"automatic_retry":false}),
        );
        return Err(error.into());
    }
    Ok(())
}
