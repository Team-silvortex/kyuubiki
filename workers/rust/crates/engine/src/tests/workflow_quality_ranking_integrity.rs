use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn next_round_checks_shared_ranking_order_and_nested_score_contract() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-quality-ranking-integrity.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for fixture in fixtures {
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
                    .all(|(key, expected)| {
                        value.get(key).is_some_and(|actual| {
                            if expected.is_number() {
                                expected.as_f64() == actual.as_f64()
                            } else {
                                expected == actual
                            }
                        })
                    })
            })
        };
        if !valid {
            failures.push(format!(
                "{}: expected {}, got {}",
                fixture["id"],
                if fixture["error"] == true {
                    "rejection"
                } else {
                    "accepted control"
                },
                result
                    .as_ref()
                    .map(|v| v["action"].to_string())
                    .unwrap_or_else(|e| e.clone())
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn generated_rankings_round_trip_and_only_canonical_permutations_are_admitted() {
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for (scores, readiness, expected) in [
        ([1.125, 1.125, 1.125], [true, true, true], ["a", "b", "z"]),
        ([2.25, 1.125, 0.0], [true, true, true], ["b", "a", "z"]),
        ([0.0, 2.25, 1.125], [false, true, true], ["b", "a", "z"]),
    ] {
        let candidates: Vec<Value> = ["z","a","b"].iter().enumerate().map(|(index, id)| json!({
            "id":id,"qualities":{"thermal":{
                "thermal_quality_score":scores[index],"thermal_quality_ready":readiness[index]
            }}
        })).collect();
        for permutation in permutations {
            let ranked = run_transform_operator(
                "transform.rank_quality_candidates",
                json!({
                    "candidates":permutation.map(|i| candidates[i].clone())
                }),
                json!({"not_ready_penalty":0}),
            )
            .unwrap();
            assert_eq!(
                ranked["ranking"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| row["candidate_id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            let exchanged: Value =
                serde_json::from_slice(&serde_json::to_vec(&ranked).unwrap()).unwrap();
            assert!(
                run_transform_operator(
                    "transform.prepare_quality_next_round_request",
                    exchanged.clone(),
                    json!({})
                )
                .is_ok()
            );

            let mut shuffled = exchanged.clone();
            shuffled["ranking"] = json!(permutation.map(|i| exchanged["ranking"][i].clone()));
            for (index, row) in shuffled["ranking"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                row["rank"] = json!(index + 1);
            }
            for (outer, inner) in [
                ("best_candidate_id", "candidate_id"),
                ("best_candidate_score", "score"),
                ("best_candidate_ready", "ready"),
            ] {
                shuffled[outer] = shuffled["ranking"][0][inner].clone();
            }
            let result = run_transform_operator(
                "transform.prepare_quality_next_round_request",
                shuffled,
                json!({}),
            );
            assert_eq!(
                result.is_ok(),
                permutation == [0, 1, 2],
                "{permutation:?}: {result:?}"
            );
        }
    }
}
