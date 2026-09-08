use serde_json::{Value, json};

pub fn execution_failure(case: &str, terminal: &Value) -> Option<Value> {
    let job = &terminal["job"];
    if job["status"] == "completed" {
        return None;
    }
    Some(json!({
        "case":case, "passed":false, "failure_stage":"execution",
        "error":job["message"].as_str().unwrap_or("job did not complete"),
        "status_detail":job["status_detail"]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_jobs_keep_the_execution_cause_instead_of_running_numerical_checks() {
        for status in ["failed", "cancelled"] {
            let terminal = json!({"job":{"status":status, "message":"agent_process_unavailable",
                "status_detail":{"failure_class":"runtime_failure"}}});
            let row = execution_failure("case", &terminal).unwrap();
            assert_eq!(row["error"], "agent_process_unavailable");
            assert_eq!(row["failure_stage"], "execution");
            assert_eq!(row["status_detail"]["failure_class"], "runtime_failure");
        }
        assert!(execution_failure("case", &json!({"job":{"status":"completed"}})).is_none());
        assert!(execution_failure("case", &Value::Null).is_some());
    }
}
