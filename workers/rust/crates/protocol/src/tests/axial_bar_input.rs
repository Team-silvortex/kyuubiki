use crate::SolveBarRequest;
use serde_json::{Value, json};
use std::io::{Cursor, Read};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.axial-bar-input-normalization.json"
    ))
    .unwrap()
}

fn patched(base: &Value, patch: &Value) -> Value {
    let mut base = base.clone();
    base.as_object_mut()
        .unwrap()
        .extend(patch.as_object().unwrap().clone());
    base
}

#[test]
fn axial_bar_shared_accepted_inputs_normalize_to_canonical_si() {
    let cases = fixture();
    assert_eq!(cases["accepted"].as_array().unwrap().len(), 10);
    for case in cases["accepted"].as_array().unwrap() {
        let input = patched(&cases["base_input"], &case["patch"]);
        let expected = patched(&cases["base_expected"], &case["expected_patch"]);
        let parsed: SolveBarRequest =
            serde_json::from_value(input).unwrap_or_else(|e| panic!("{}: {e}", case["id"]));
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            expected,
            "{}",
            case["id"]
        );
    }
}

#[test]
fn axial_bar_shared_invalid_inputs_never_choose_units_or_round_counts_silently() {
    let cases = fixture();
    assert_eq!(cases["rejected"].as_array().unwrap().len(), 26);
    for case in cases["rejected"].as_array().unwrap() {
        assert!(
            serde_json::from_value::<SolveBarRequest>(patched(
                &cases["base_input"],
                &case["patch"]
            ))
            .is_err(),
            "{}",
            case["id"]
        );
    }
}

#[test]
fn axial_bar_canonical_si_roundtrips_without_compatibility_labels() {
    let input =
        json!({"length":1.0,"area":0.01,"elements":4,"tip_force":1000.0,"youngs_modulus":210e9});
    let parsed: SolveBarRequest = serde_json::from_value(input.clone()).unwrap();
    let serialized = serde_json::to_vec(&parsed).unwrap();
    assert_eq!(
        serde_json::from_slice::<SolveBarRequest>(&serialized).unwrap(),
        parsed
    );
    assert_eq!(serde_json::from_slice::<Value>(&serialized).unwrap(), input);
}

#[test]
fn axial_bar_rejects_duplicate_fields_and_missing_required_parameters() {
    let duplicate = r#"{"length":1,"area":0.01,"elements":4,"tip_force":1000,"youngs_modulus_gpa":210,"youngs_modulus_gpa":200}"#;
    assert!(serde_json::from_str::<SolveBarRequest>(duplicate).is_err());
    let mut base = fixture()["base_input"].clone();
    base["youngs_modulus_gpa"] = json!(210);
    for field in ["length", "area", "elements", "tip_force"] {
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<SolveBarRequest>(missing).is_err(),
            "{field}"
        );
    }
}

#[test]
fn axial_bar_reader_skips_large_metadata_before_late_unit_fields() {
    let prefix = Cursor::new(br#"{"transport_padding":""#);
    let padding = std::io::repeat(b'x').take(8_000_000);
    let suffix = Cursor::new(br#"","length":"1","area":"0.01","elements":"4","tip_force":"1000","youngs_modulus_gpa":"210"}"#);
    let model: SolveBarRequest =
        serde_json::from_reader(prefix.chain(padding).chain(suffix)).unwrap();
    assert_eq!(model.youngs_modulus, 210e9);
    assert_eq!(model.elements, 4);
}

#[test]
fn axial_bar_scalar_rejection_does_not_materialize_nested_field_bodies() {
    struct Unreadable;
    impl Read for Unreadable {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            panic!("decoder tried to materialize an invalid scalar body");
        }
    }
    for prefix in [
        br#"{"length":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,"#.as_slice(),
        br#"{"length":{"nested":[0,0,0,0,0,0,0,0,0,0,0,0,"#.as_slice(),
    ] {
        // Allow error-position lookahead, but never scan the nested body's remainder.
        let reader = Cursor::new(prefix).chain(Unreadable);
        assert!(serde_json::from_reader::<_, SolveBarRequest>(reader).is_err());
    }
}
