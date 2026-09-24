use kyuubiki_protocol::{SolveContactGap1dResult, SolveNonlinearSpring1dResult};
use serde_json::{Value, json};

fn legacy_result(contact: bool, converged: bool) -> Value {
    // Minimal serialization fixture; physical validity is tested by the solver suites.
    let mut value = json!({
        "input":{"nodes":[], "elements":[]}, "nodes":[], "elements":[], "steps":[],
        "converged":converged, "residual_norm":0.0, "max_displacement":0.0, "max_force":0.0
    });
    if contact {
        value["input"]["contacts"] = json!([]);
        value["contacts"] = json!([]);
        value["max_contact_force"] = json!(0.0);
        value["active_contact_count"] = json!(0);
    }
    value
}

fn round_trip(contact: bool, value: Value) -> (Option<f64>, Value) {
    if contact {
        let result: SolveContactGap1dResult = serde_json::from_value(value).unwrap();
        (
            result.achieved_load_factor,
            serde_json::to_value(result).unwrap(),
        )
    } else {
        let result: SolveNonlinearSpring1dResult = serde_json::from_value(value).unwrap();
        (
            result.achieved_load_factor,
            serde_json::to_value(result).unwrap(),
        )
    }
}

#[test]
fn legacy_spring_results_do_not_invent_a_committed_load_factor() {
    for contact in [false, true] {
        for converged in [false, true] {
            let (factor, encoded) = round_trip(contact, legacy_result(contact, converged));
            assert_eq!(factor, None);
            assert!(encoded.get("achieved_load_factor").is_none());
        }
    }
}

#[test]
fn zero_partial_and_full_achieved_factors_survive_protocol_round_trips() {
    for contact in [false, true] {
        for factor in [0.0, 0.5, 1.0] {
            let mut value = legacy_result(contact, factor == 1.0);
            value["achieved_load_factor"] = json!(factor);
            let (decoded, encoded) = round_trip(contact, value);
            assert_eq!(decoded, Some(factor));
            assert_eq!(encoded["achieved_load_factor"], factor);
            assert_eq!(encoded["converged"], factor == 1.0);
        }
    }
}
