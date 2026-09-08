use serde_json::{Value, json};

#[test]
fn measured_research_values_decode_to_the_correct_ieee_value() {
    for decimal in [
        "1.8181818181817597",
        "30.909090909090878",
        "0.18181818181817602",
    ] {
        let reference: f64 = decimal.parse().unwrap();
        let decoded: Value = serde_json::from_str(decimal).unwrap();
        assert_eq!(
            decoded.as_f64().unwrap().to_bits(),
            reference.to_bits(),
            "{decimal}"
        );
    }
}

#[test]
fn persisted_scientific_json_does_not_drift_on_repeated_readback() {
    for value in [
        1.8181818181817597,
        30.909090909090878,
        0.18181818181817602,
        8.239936510889834e-18,
        -5.539595475043124e-14,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1),
        -0.0,
    ] {
        let expected = json!({"field":value});
        let mut observed = expected.clone();
        for _ in 0..10 {
            observed =
                serde_json::from_slice(&serde_json::to_vec_pretty(&observed).unwrap()).unwrap();
            assert_eq!(
                observed["field"].as_f64().unwrap().to_bits(),
                value.to_bits()
            );
        }
    }
}

#[test]
fn deterministic_finite_bit_patterns_roundtrip_without_a_tolerance() {
    let mut bits = 0x9e3779b97f4a7c15_u64;
    for _ in 0..8192 {
        bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = f64::from_bits(bits);
        if value.is_finite() {
            let encoded = serde_json::to_vec(&value).unwrap();
            let decoded: f64 = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(
                decoded.to_bits(),
                bits,
                "{}",
                String::from_utf8_lossy(&encoded)
            );
        }
    }
}
