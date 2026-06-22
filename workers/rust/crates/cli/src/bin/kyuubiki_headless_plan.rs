use std::fs;

use kyuubiki_headless_sdk::{HeadlessExecutionPlan, build_execution_plan};

pub(super) fn handle_plan(args: &[String]) -> Result<(), String> {
    let flags = super::Flags::parse(args);
    let input_path = flags.input_path()?;
    let batch = super::load_batch_from_path(&input_path)?;
    let plan = build_execution_plan(&batch);
    if let Some(output_path) = &flags.out {
        let output_bytes = serde_json::to_vec_pretty(&plan).map_err(|error| error.to_string())?;
        fs::write(output_path, output_bytes)
            .map_err(|error| format!("failed to write {}: {error}", output_path))?;
    }
    if flags.json {
        super::print_json(&plan)?;
        return if plan.ok {
            Ok(())
        } else {
            Err("plan generated from invalid batch".to_string())
        };
    }
    print_plan(&plan);
    if let Some(output_path) = &flags.out {
        println!("Plan: {output_path}");
    }
    if plan.ok {
        Ok(())
    } else {
        Err("plan generated from invalid batch".to_string())
    }
}

fn print_plan(plan: &HeadlessExecutionPlan) {
    println!("Headless plan: {}", plan.workflow_id);
    if let Some(policy) = &plan.policy {
        println!("Runtime: {:?}", policy.recommended_runtime);
        println!("Engines: {:?}", policy.required_engines);
    }
    println!("Steps: {}", plan.steps.len());
    println!("Confirmations: {}", plan.confirmation_count);
    println!(
        "Service only: {}",
        if plan.compatibility.service_only_ok {
            "yes"
        } else {
            "no"
        }
    );
    if plan.compatibility.browser_session_required {
        println!("Requires desktop browser session: yes");
    }
    for executor in &plan.executor_matrix {
        println!(
            "executor {}: {} ({} issues)",
            executor.executor,
            if executor.compatible {
                "compatible"
            } else {
                "blocked"
            },
            executor.issue_count,
        );
    }
    for confirmation in &plan.confirmations {
        println!(
            "confirm: step {} {} needs {}",
            confirmation.index, confirmation.action, confirmation.flag
        );
    }
}
