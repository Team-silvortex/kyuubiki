use kyuubiki_protocol::{
    ModalFrame2dModeResult, ModalFrame3dModeResult, SolveModalFrame2dRequest,
    compute_operator_task_digest, verify_operator_task_digest,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::io::Cursor;

fn finite_values() -> Vec<f64> {
    let mut values = vec![
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        1.4553085447431156e-7,
        1.236075027143164e-7,
        1.0 / 3.0,
    ];
    // Deterministic mantissas across the binary64 range; these are codec fixtures,
    // not claims that every such value describes a physically solvable model.
    let mut state = 0x6d6f_6461_6c5f_6a73_u64;
    for exponent in [0, 1, 17, 300, 700, 1022, 1023, 1024, 1500, 2000, 2046] {
        for _ in 0..32 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let bits = ((exponent as u64) << 52) | (state & ((1_u64 << 52) - 1));
            values.push(f64::from_bits(bits));
            values.push(f64::from_bits(bits | (1_u64 << 63)));
        }
    }
    values
}

fn assert_bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "JSON changed component {index}: {expected:e} -> {actual:e}"
        );
    }
}

fn round_trip<T: Serialize + DeserializeOwned>(value: &T) -> T {
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}

#[test]
fn modal_json_reader_paths_preserve_finite_binary64_bits() {
    let expected = finite_values();
    for encoded in [
        serde_json::to_string(&expected).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap(),
    ] {
        let text: Vec<f64> = serde_json::from_str(&encoded).unwrap();
        let bytes: Vec<f64> = serde_json::from_slice(encoded.as_bytes()).unwrap();
        let reader: Vec<f64> = serde_json::from_reader(Cursor::new(encoded.as_bytes())).unwrap();
        for actual in [text, bytes, reader] {
            assert_bits(&actual, &expected);
        }
    }
}

#[test]
fn modal_json_mode_contract_preserves_frequencies_and_shape_bits() {
    let mode = ModalFrame2dModeResult {
        index: 0,
        eigenvalue_rad_s_squared: 1.4553085447431156e-7,
        natural_frequency_rad_s: 1.4553085447431156e-7_f64.sqrt(),
        natural_frequency_hz: 1.4553085447431156e-7_f64.sqrt() / std::f64::consts::TAU,
        period_s: std::f64::consts::TAU / 1.4553085447431156e-7_f64.sqrt(),
        participation_norm: 1.0,
        shape: finite_values(),
    };
    let spatial = ModalFrame3dModeResult {
        index: mode.index,
        eigenvalue_rad_s_squared: mode.eigenvalue_rad_s_squared,
        natural_frequency_rad_s: mode.natural_frequency_rad_s,
        natural_frequency_hz: mode.natural_frequency_hz,
        period_s: mode.period_s,
        participation_norm: mode.participation_norm,
        shape: mode.shape.clone(),
    };
    let planar: ModalFrame2dModeResult = round_trip(&mode);
    let decoded: ModalFrame3dModeResult = round_trip(&spatial);
    let original = serde_json::to_value(&mode).unwrap();
    for actual in [&planar.shape, &decoded.shape] {
        assert_bits(actual, &mode.shape);
    }
    for encoded in [
        serde_json::to_value(planar).unwrap(),
        serde_json::to_value(decoded).unwrap(),
    ] {
        for key in [
            "eigenvalue_rad_s_squared",
            "natural_frequency_rad_s",
            "natural_frequency_hz",
            "period_s",
            "participation_norm",
        ] {
            assert_eq!(
                encoded[key].as_f64().unwrap().to_bits(),
                original[key].as_f64().unwrap().to_bits()
            );
        }
    }
}

#[test]
fn modal_json_request_preserves_material_and_geometry_bits() {
    let values = finite_values();
    for sample in values.iter().copied().filter(|v| *v > 0.0) {
        let model: SolveModalFrame2dRequest = serde_json::from_value(json!({
            "nodes":[{"id":"codec-node", "x":sample, "y":sample,
                "fix_x":true, "fix_y":true, "fix_rz":true,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0}], "mode_count":1,
            "elements":[{"id":"codec-fixture", "node_i":0, "node_j":1,
                "area":sample, "youngs_modulus":sample, "moment_of_inertia":sample,
                "section_modulus":sample, "density":sample}]
        }))
        .unwrap();
        let decoded: SolveModalFrame2dRequest = round_trip(&model);
        let element = &decoded.elements[0];
        assert_bits(
            &[
                decoded.nodes[0].x,
                decoded.nodes[0].y,
                element.area,
                element.youngs_modulus,
                element.moment_of_inertia,
                element.section_modulus,
                element.density,
            ],
            &[sample; 7],
        );
    }
}

#[test]
fn finite_numeric_task_digests_survive_json_round_trip() {
    let mut task = json!({
        "schema_version":"kyuubiki.operator-task-ir/v1", "task_id":"modal-codec",
        "operator":{"id":"builtin.modal_frame_2d"},
        "input_artifact":{"payload":{"mode_components":finite_values()}}
    });
    let digest = compute_operator_task_digest(&task).unwrap();
    task["integrity"] = json!({"task_digest":digest});
    let decoded: Value = round_trip(&task);
    assert_eq!(compute_operator_task_digest(&decoded).unwrap(), digest);
    verify_operator_task_digest(&decoded).unwrap();
}

#[test]
fn modal_json_readers_reject_invalid_numbers_and_allow_valid_replay() {
    for token in ["1e309", "-1e309", "NaN", "Infinity", "null"] {
        let encoded = format!("[0.0,{token}]");
        assert!(serde_json::from_str::<Vec<f64>>(&encoded).is_err());
        assert!(serde_json::from_slice::<Vec<f64>>(encoded.as_bytes()).is_err());
        assert!(serde_json::from_reader::<_, Vec<f64>>(Cursor::new(encoded)).is_err());
        assert_bits(&round_trip(&finite_values()), &finite_values());
    }
}
