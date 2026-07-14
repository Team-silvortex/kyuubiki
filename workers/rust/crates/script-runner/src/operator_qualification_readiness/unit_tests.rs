use super::{compare_actions, priority_rank, readiness_rank};

#[test]
fn sort_ranks_match_contract_order() {
    assert!(priority_rank("p0") < priority_rank("p1"));
    assert!(readiness_rank("broken") < readiness_rank("planned"));
}

#[test]
fn compare_actions_orders_priority_first() {
    let left = serde_json::json!({"priority": "p0", "readiness": "blocked", "candidate_id": "z"});
    let right = serde_json::json!({"priority": "p1", "readiness": "broken", "candidate_id": "a"});
    assert!(compare_actions(&left, &right).is_lt());
}
