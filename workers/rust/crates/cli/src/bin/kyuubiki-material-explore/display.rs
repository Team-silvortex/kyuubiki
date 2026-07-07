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
}
