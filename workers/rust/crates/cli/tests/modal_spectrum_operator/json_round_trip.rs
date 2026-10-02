use super::*;

fn encoded_model(model: &Value) -> Value {
    serde_json::from_slice(&serde_json::to_vec(model).unwrap()).unwrap()
}

fn assert_numeric_bits(expected: &Value, actual: &Value) {
    match (expected, actual) {
        (Value::Number(a), Value::Number(b)) => {
            assert_eq!(a.as_f64().unwrap().to_bits(), b.as_f64().unwrap().to_bits());
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                assert_numeric_bits(a, b);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (key, a) in a {
                assert_numeric_bits(a, b.get(key).expect("JSON field must survive"));
            }
        }
        _ => assert_eq!(expected, actual),
    }
}

#[test]
fn headless_json_modal_results_preserve_numeric_bits_and_physical_residuals() {
    for (segments, space) in [(96, false), (100, false), (100, true)] {
        let model = polished_bending::request(segments, 1.0, space);
        let model_after_json = encoded_model(&model);
        assert_numeric_bits(&model, &model_after_json);
        let original = planned_solve(space, model_after_json).unwrap();
        let encoded = serde_json::to_vec(&original).unwrap();
        let decoded: Value = serde_json::from_reader(std::io::Cursor::new(encoded)).unwrap();
        assert_numeric_bits(&original, &decoded);
        polished_bending::check(&decoded, segments, 1.0, space);
    }
}

#[test]
fn headless_json_unresolved_spectrum_returns_error_and_valid_request_replays() {
    let mut unresolved = polished_bending::request(128, 1.0, false);
    unresolved["mode_count"] = json!(6);
    let error = planned_solve(false, encoded_model(&unresolved)).unwrap_err();
    assert!(
        error.contains("refinement did not converge") && error.contains("relative="),
        "{error}"
    );
    let replay = polished_bending::request(100, 1.0, false);
    let result = planned_solve(false, encoded_model(&replay)).unwrap();
    let decoded: Value = encoded_model(&result);
    assert_numeric_bits(&result, &decoded);
    polished_bending::check(&decoded, 100, 1.0, false);
}
