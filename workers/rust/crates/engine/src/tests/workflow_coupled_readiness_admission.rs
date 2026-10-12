use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

pub(super) fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-coupled-readiness-admission.json"
    ))
    .unwrap()
}

#[test]
fn generated_coupled_reports_remain_usable_after_json_exchange_and_duplication() {
    for (ready, grade, action) in [
        (true, "good", "stop"),
        (true, "watch", "continue"),
        (false, "block", "replan"),
    ] {
        let report = run_transform_operator(
            "transform.evaluate_coupled_readiness",
            json!({"domains":{"thermal":{"thermal_quality_ready":ready,
                "thermal_quality_grade":grade,"thermal_quality_score":0.25}}}),
            json!({"required_domains":["thermal"]}),
        )
        .unwrap();
        let input = json!({"coupled_readiness":report,
            "ranking":[{"candidate_id":"a","score":1,"ready":true,
                "coupled_readiness":report,"objective":{"coupled_readiness":report}}]});
        let exchanged = serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        let decision = run_transform_operator(
            "transform.prepare_quality_next_round_request",
            exchanged,
            json!({}),
        )
        .unwrap();
        assert_eq!(decision["action"], action);
        assert_eq!(decision["selected_candidate_ready"], ready);
        assert_eq!(decision["selected_coupled_readiness"], report);
    }
}

#[test]
fn shared_coupled_readiness_cannot_be_masked_or_silently_replaced() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let result = run_transform_operator(
            "transform.prepare_quality_next_round_request",
            fixture["payload"].clone(),
            fixture["config"].clone(),
        );
        let valid = if fixture["error"] == true {
            result.is_err()
        } else {
            result.as_ref().is_ok_and(|value| {
                fixture["expected"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .all(|(key, expected)| value.get(key) == Some(expected))
            })
        };
        if !valid {
            failures.push(format!("{}: {result:?}", fixture["id"]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn coupled_readiness_alias_placement_and_json_exchange_preserve_decisions() {
    let cases = fixtures();
    let reports: Vec<Value> = ["all_null", "row_ready", "row_warning", "row_block", "row_false_only"]
        .iter().map(|id| cases.iter().find(|case| case["id"] == *id).unwrap()["payload"]["ranking"][0]["coupled_readiness"].clone())
        .collect();
    for row in 0..reports.len() {
        for objective in 0..reports.len() {
            for payload in 0..reports.len() {
                let indices = [row, objective, payload];
                let distinct = indices
                    .into_iter()
                    .filter(|i| *i != 0)
                    .collect::<std::collections::BTreeSet<_>>();
                let input = json!({
                    "ranking":[{"candidate_id":"a","score":1,"ready":true,
                        "coupled_readiness":reports[row],
                        "objective":{"coupled_readiness":reports[objective]}}],
                    "coupled_readiness":reports[payload]
                });
                let exchanged =
                    serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
                let result = run_transform_operator(
                    "transform.prepare_quality_next_round_request",
                    exchanged,
                    json!({}),
                );
                if distinct.len() > 1 {
                    assert!(result.is_err(), "{indices:?}");
                } else {
                    let decision = result.unwrap();
                    let selected = distinct.first().copied().unwrap_or(0);
                    assert_eq!(
                        decision["action"],
                        match selected {
                            2 => "continue",
                            3 | 4 => "replan",
                            _ => "stop",
                        }
                    );
                    assert_eq!(decision["selected_coupled_readiness"], reports[selected]);
                    assert_eq!(
                        decision["request_payload"]["coupled_readiness"],
                        reports[selected]
                    );
                    if selected >= 2 {
                        assert_eq!(
                            decision["selected_iteration_hint"]["action"],
                            if selected == 2 {
                                "review_coupled_readiness"
                            } else {
                                "fix_coupled_readiness"
                            }
                        );
                    }
                }
            }
        }
    }
}
