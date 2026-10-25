#[path = "../examples/durable_modal_research/result_contract.rs"]
mod contract;
#[path = "support/modal_metadata_faults.rs"]
mod faults;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;

use serde_json::{Value, json};

// Manufactured metadata only, not a solution of this physical input. The real
// Agent suites independently check complete results through checks::verify.
fn fixture(reversed: bool) -> (Value, Value) {
    let input = model::model(reversed);
    let modes: Vec<_> = (1..=4)
        .map(|i| {
            let omega = i as f64;
            json!({"eigenvalue_rad_s_squared":omega * omega,
            "natural_frequency_rad_s":omega,
            "natural_frequency_hz":omega / std::f64::consts::TAU,
            "participation_norm":1.0, "shape":[0.0, 0.6, 0.8]})
        })
        .collect();
    let output = json!({"input":input, "modes":modes,
        "min_frequency_hz":1.0 / std::f64::consts::TAU,
        "max_frequency_hz":4.0 / std::f64::consts::TAU});
    (input, output)
}

#[test]
fn modal_metadata_accepts_complete_reordered_input_and_rounding() {
    for reversed in [false, true] {
        let (input, output) = fixture(reversed);
        contract::validate(&input, &output).unwrap();
        let mut rounded: Value =
            serde_json::from_slice(&serde_json::to_vec(&output).unwrap()).unwrap();
        rounded["modes"][0]["natural_frequency_rad_s"] = json!(1.0 + 2.0 * f64::EPSILON);
        rounded["modes"][0]["participation_norm"] = json!(1.0 - f64::EPSILON);
        contract::validate(&input, &rounded).unwrap();
    }
}

#[test]
fn modal_metadata_rejects_missing_malformed_or_contradictory_reports() {
    for reversed in [false, true] {
        let (input, output) = fixture(reversed);
        let cases = faults::cases(&output);
        assert_eq!(cases.len(), 38);
        for (name, corrupt, cause) in cases {
            let error = contract::validate(&input, &corrupt)
                .unwrap_err()
                .to_string();
            assert!(error.contains(cause), "{name}: {error}");
        }
        let mut reordered = output;
        reordered["input"] = model::model(!reversed);
        assert!(
            contract::validate(&input, &reordered)
                .unwrap_err()
                .to_string()
                .contains("input echo")
        );
    }
}

#[test]
fn modal_metadata_checks_every_mode_and_derived_norm() {
    let (input, output) = fixture(false);
    for index in 0..4 {
        for field in ["natural_frequency_rad_s", "participation_norm"] {
            for value in [
                json!(0.0),
                json!(-0.0),
                json!(f64::MAX),
                json!(f64::MIN_POSITIVE),
                json!(false),
            ] {
                let mut corrupt = output.clone();
                corrupt["modes"][index][field] = value;
                assert!(
                    contract::validate(&input, &corrupt).is_err(),
                    "mode {index}: {field}"
                );
            }
        }
        let mut corrupt = output.clone();
        corrupt["modes"][index]["shape"] = json!([0.0, 0.3, 0.4]);
        assert!(
            contract::validate(&input, &corrupt)
                .unwrap_err()
                .to_string()
                .contains("participation norm")
        );
    }
}

#[test]
fn modal_metadata_rejects_unrepresentable_and_empty_derived_values() {
    let (input, output) = fixture(false);
    for shape in [
        json!([]),
        json!(vec![0.0; 3]),
        json!([f64::MAX, f64::MAX]),
        json!([null]),
        json!(["1.0"]),
        json!({}),
    ] {
        let mut corrupt = output.clone();
        corrupt["modes"][0]["shape"] = shape;
        assert!(
            contract::validate(&input, &corrupt)
                .unwrap_err()
                .to_string()
                .contains("participation norm")
        );
    }
    for modes in [json!([]), Value::Null, json!({})] {
        let mut corrupt = output.clone();
        corrupt["modes"] = modes;
        assert!(contract::validate(&input, &corrupt).is_err());
    }
    let mut corrupt = output;
    corrupt["modes"][0]["natural_frequency_hz"] = json!(f64::MAX);
    assert!(
        contract::validate(&input, &corrupt)
            .unwrap_err()
            .to_string()
            .contains("angular frequency")
    );
}

#[test]
fn modal_frequency_summary_matches_returned_modes_not_an_unchecked_bound() {
    let (input, output) = fixture(false);
    for (field, factor) in [("min_frequency_hz", 0.5), ("max_frequency_hz", 2.0)] {
        let mut corrupt = output.clone();
        corrupt[field] = json!(output[field].as_f64().unwrap() * factor);
        assert!(
            contract::validate(&input, &corrupt)
                .unwrap_err()
                .to_string()
                .contains("frequency summary")
        );
    }
    let mut reordered = output;
    reordered["modes"].as_array_mut().unwrap().reverse();
    // Summary extrema are recomputed, not guessed from first/last positions.
    // Spectral ordering remains the separate independent checker's responsibility.
    contract::validate(&input, &reordered).unwrap();
}
