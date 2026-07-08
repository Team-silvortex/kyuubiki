use serde_json::Value;

pub(crate) fn print_summary(exploration: &Value) {
    let report = &exploration["report"];
    println!(
        "Material exploration: {}",
        exploration["study"].as_str().unwrap_or("--")
    );
    println!("Mode: {}", exploration["mode"].as_str().unwrap_or("--"));
    println!(
        "Candidates: {}",
        exploration["candidate_count"].as_u64().unwrap_or(0)
    );
    println!(
        "Winner: {}",
        report
            .get("winner_candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    if let Some(candidates) = report.get("candidates").and_then(Value::as_array) {
        for candidate in candidates {
            println!(
                "#{} {} score={:.6}",
                candidate.get("rank").and_then(Value::as_u64).unwrap_or(0),
                candidate
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or("--"),
                candidate
                    .get("score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            );
        }
    }
    if let Some(next_round) = exploration.get("next_round") {
        println!(
            "Next round: {}",
            next_round
                .get("decision")
                .and_then(Value::as_str)
                .unwrap_or("--")
        );
        if let Some(actions) = next_round.get("actions").and_then(Value::as_array) {
            let action_list = actions
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            if !action_list.is_empty() {
                println!("Actions: {action_list}");
            }
        }
    }
}

pub(crate) fn print_catalog_summary(catalog: &Value) {
    println!(
        "Material study catalog: {} studies",
        catalog
            .get("study_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    if let Some(studies) = catalog.get("studies").and_then(Value::as_array) {
        for study in studies {
            println!(
                "- {} [{}]: {}",
                study.get("id").and_then(Value::as_str).unwrap_or("--"),
                study.get("domain").and_then(Value::as_str).unwrap_or("--"),
                study
                    .get("objective")
                    .and_then(Value::as_str)
                    .unwrap_or("--")
            );
        }
    }
}

pub(crate) fn print_study_summary(study: &Value) {
    println!(
        "Material study: {}",
        study.get("id").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Domain: {}",
        study.get("domain").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Template: {}",
        study
            .get("template_id")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    println!(
        "Schema: {}",
        study
            .get("report_schema_version")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    println!(
        "Metrics: {}",
        study
            .get("metric_specs")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
    );
}

pub(crate) fn print_study_plan_summary(plan: &Value) {
    println!(
        "Material study plan: {}",
        plan.get("study_id").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Steps: {} total, {} solve",
        plan.get("step_count").and_then(Value::as_u64).unwrap_or(0),
        plan.get("solve_step_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    if let Some(candidate_ids) = plan.get("candidate_ids").and_then(Value::as_array) {
        let candidates = candidate_ids
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !candidates.is_empty() {
            println!("Candidates: {candidates}");
        }
    }
    println!(
        "Run: {}",
        plan.get("recommended_command")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
}

pub(crate) fn print_next_round_plan_summary(plan: &Value) {
    println!(
        "Material next round: {}",
        plan.get("study").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Decision: {}",
        plan.get("decision").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Runnable steps: {}",
        plan.get("runnable_step_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    if let Some(focus) = plan.get("focus_candidate_ids").and_then(Value::as_array) {
        let focus_list = focus
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !focus_list.is_empty() {
            println!("Focus: {focus_list}");
        }
    }
    if let Some(objectives) = plan.get("optimization_objectives") {
        println!(
            "Optimization mode: {}",
            objectives
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("--")
        );
        if let Some(metrics) = objectives
            .get("primary_metric_ids")
            .and_then(Value::as_array)
        {
            let metric_list = metrics
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            if !metric_list.is_empty() {
                println!("Primary metrics: {metric_list}");
            }
        }
    }
}

pub(crate) fn print_chain_summary(chain: &Value) {
    println!(
        "Material exploration chain: {} rounds",
        chain
            .get("round_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    println!(
        "Final iteration: {}",
        chain
            .get("final_iteration")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    println!(
        "Final winner: {}",
        chain
            .get("final_winner_candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    if let Some(assessment) = chain.get("convergence_assessment") {
        println!(
            "Convergence: {}",
            assessment
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("--")
        );
        println!(
            "Recommendation: {}",
            assessment
                .get("recommendation")
                .and_then(Value::as_str)
                .unwrap_or("--")
        );
    }
}
