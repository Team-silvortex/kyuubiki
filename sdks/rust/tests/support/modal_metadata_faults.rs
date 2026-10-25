use serde_json::{Value, json};

// Mutate only test-owned result copies, never the Agent's actual ledger.
pub fn cases(original: &Value) -> Vec<(String, Value, &'static str)> {
    let mut cases = vec![];
    for (pointer, cause) in [
        ("/input", "input echo"),
        ("/min_frequency_hz", "frequency summary"),
        ("/max_frequency_hz", "frequency summary"),
        ("/modes/0/natural_frequency_rad_s", "angular frequency"),
        ("/modes/3/natural_frequency_rad_s", "angular frequency"),
        ("/modes/0/participation_norm", "participation norm"),
        ("/modes/3/participation_norm", "participation norm"),
    ] {
        for (kind, replacement) in [
            ("missing", None),
            ("null", Some(Value::Null)),
            ("text", Some(json!("1.0"))),
            ("negative", Some(json!(-1.0))),
            ("wrong", Some(json!(999.0))),
        ] {
            let mut result = original.clone();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            let object = result.pointer_mut(parent).unwrap().as_object_mut().unwrap();
            match replacement {
                Some(value) => {
                    object.insert(key.into(), value);
                }
                None => {
                    object.remove(key);
                }
            }
            cases.push((
                format!(
                    "{}-{kind}",
                    pointer.trim_start_matches('/').replace('/', "-")
                ),
                result,
                cause,
            ));
        }
    }
    let mut different_input = original.clone();
    different_input["input"]["elements"][0]["id"] = json!("other-material");
    cases.push(("wrong-material-label".into(), different_input, "input echo"));
    let mut wrong_geometry = original.clone();
    wrong_geometry["input"]["nodes"][1]["x"] = json!(42.0);
    cases.push(("wrong-input-geometry".into(), wrong_geometry, "input echo"));
    let mut inverted = original.clone();
    inverted["min_frequency_hz"] = original["max_frequency_hz"].clone();
    inverted["max_frequency_hz"] = original["min_frequency_hz"].clone();
    cases.push((
        "inverted-frequency-range".into(),
        inverted,
        "frequency summary",
    ));
    cases
}
