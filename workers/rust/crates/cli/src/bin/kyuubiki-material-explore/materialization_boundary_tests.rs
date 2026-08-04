use crate::materialization::review_decision_template;
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn chain_summary_returns_a_successful_not_applicable_review_boundary() {
    let path = std::env::temp_dir().join(format!(
        "kyuubiki-chain-review-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::write(
        &path,
        serde_json::to_vec(&json!({
            "schema_version": "kyuubiki.material-exploration-chain/v1",
            "rounds": [],
            "stop_reason": "no_search_space_progress"
        }))
        .expect("serialize chain"),
    )
    .expect("write chain fixture");

    let review = review_decision_template(path.to_str().expect("utf-8 path"))
        .expect("chain review boundary should succeed");
    let _ = fs::remove_file(path);

    assert_eq!(review["status"], "not_applicable");
    assert_eq!(review["terminal"], true);
    assert_eq!(review["next_action"], "plan_next");
    assert_eq!(review["review_policy"]["required"], false);
}
