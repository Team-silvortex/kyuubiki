use super::*;
use sha2::{Digest, Sha256};
use std::io::Write;

fn raw_record(path: &Path, body: &[u8]) {
    let mut file = io::open_private(path, true).unwrap();
    let digest = format!("{:x}\n", Sha256::digest(body));
    file.write_all(digest.as_bytes()).unwrap();
    file.write_all(body).unwrap();
    file.sync_all().unwrap();
}

#[test]
fn checksummed_receipt_cannot_silently_choose_duplicate_result_or_failure_fields() {
    for (index, response) in [
        r#""ok":true,"result":{"value":1,"value":2}"#,
        r#""ok":true,"result":{"nested":[{"value":1,"\u0076alue":2}]}"#,
        r#""ok":false,"error":{"code":"failed","message":"test","details":{"retry":false,"retry":true}}"#,
        r#""ok":true,"result":{},"metadata":{"note":"one","note":"two"}"#,
    ].into_iter().enumerate() {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let task = task('a');
        let record = admit(&mut store, &task);
        let path = scratch.0.join(format!("{}.receipt", record.execution_id));
        let body = format!(
            "{{\"record\":{},\"response\":{{\"rpc_version\":1,\"id\":{},{response}}}}}",
            serde_json::to_string(&record).unwrap(),
            serde_json::to_string(&record.request_id).unwrap()
        );
        raw_record(&path, body.as_bytes());
        let original = fs::read(&path).unwrap();
        let error = store.lookup(&record.execution_id).unwrap_err();
        assert!(error.contains("duplicate JSON object key"), "{index}: {error}");
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        assert!(store.lookup(&record.execution_id).unwrap_err().contains("duplicate JSON object key"));
        assert!(store.admit_with(&task, "restart", "duplicate", || -> Result<(), String> {
            panic!("an ambiguous original receipt must not reserve or reexecute");
        }).is_err());
        assert_eq!(fs::read(path).unwrap(), original, "reader must not repair evidence");
    }
}

#[test]
fn unique_record_json_preserves_numeric_bits_strings_and_sibling_scopes() {
    let scratch = Scratch::new();
    let _store = Store::open(&scratch.0).unwrap();
    let path = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
    let body = br#" {
        "negative_zero": -0.0,
        "tiny": 5e-324,
        "precise": 1.0000000000000002,
        "large": 1e308,
        "maximum": 18446744073709551615,
        "minimum": -9223372036854775808,
        "string": "not an object: {\"key\":1,\"key\":2}",
        "\u006ceft": {"value": 1, "nullable": null},
        "right": [{"value": true}, {"value": false}, {"value": "\u03b1"}]
    } "#;
    raw_record(&path, body);
    let observed: Value = io::read(&path, body.len()).unwrap().unwrap();
    let expected: Value = serde_json::from_slice(body).unwrap();
    assert_eq!(observed, expected);
    for key in ["negative_zero", "tiny", "precise", "large"] {
        assert_eq!(
            observed[key].as_f64().unwrap().to_bits(),
            expected[key].as_f64().unwrap().to_bits()
        );
    }
    assert_eq!(
        observed["negative_zero"].as_f64().unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(observed["tiny"].as_f64().unwrap().to_bits(), 1);
    assert_eq!(
        observed["precise"].as_f64().unwrap().to_bits(),
        1.0000000000000002_f64.to_bits()
    );
    assert_eq!(observed["maximum"].as_u64(), Some(u64::MAX));
    assert_eq!(observed["minimum"].as_i64(), Some(i64::MIN));
    let encoded = serde_json::to_vec(&observed).unwrap();
    let prepared = io::prepare(&observed, encoded.len()).unwrap();
    let copy = format!("{}.receipt", "b".repeat(32));
    io::commit(&scratch.0, &copy, prepared).unwrap();
    let restored: Value = io::read(&scratch.0.join(copy), encoded.len())
        .unwrap()
        .unwrap();
    assert_eq!(restored, observed);
    assert_eq!(
        restored["negative_zero"].as_f64().unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert!(
        io::read::<Value>(&path, body.len() - 1)
            .unwrap_err()
            .contains("byte limit")
    );
}

#[test]
fn native_record_json_rejects_ambiguity_trailing_input_and_unbounded_nesting() {
    let mut cases = vec![
        br#"{"value":1,"value":1}"#.to_vec(),
        br#"[{"nested":{"value":1,"\u0076alue":2}}]"#.to_vec(),
        br#"{"value":1} {}"#.to_vec(),
        br#"{"value":1e999}"#.to_vec(),
        br#"{"value":NaN}"#.to_vec(),
        b"{\"value\":\"\xff\"}".to_vec(),
    ];
    cases.push(format!("{}0{}", "[".repeat(140), "]".repeat(140)).into_bytes());
    for (index, body) in cases.iter().enumerate() {
        let scratch = Scratch::new();
        let _store = Store::open(&scratch.0).unwrap();
        let path = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
        raw_record(&path, body);
        assert!(
            io::read::<Value>(&path, MAX_RECEIPT_BYTES).is_err(),
            "case {index}"
        );
    }
}

struct DuplicateEncoded;

impl Serialize for DuplicateEncoded {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("value", &1)?;
        map.serialize_entry("value", &2)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for DuplicateEncoded {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Value::deserialize(decoder)?;
        Ok(Self)
    }
}

#[test]
fn ambiguous_serialization_is_rejected_before_any_record_file_is_created() {
    let scratch = Scratch::new();
    let _store = Store::open(&scratch.0).unwrap();
    let before = fs::read_dir(&scratch.0).unwrap().count();
    match io::prepare(&DuplicateEncoded, MAX_RECORD_BYTES) {
        Err(io::PreparationError::RestartDecode(message)) => {
            assert!(message.contains("duplicate JSON object key"));
        }
        _ => panic!("prepared bytes must obey the same uniqueness gate as restart reads"),
    }
    assert!(
        io::write(
            &scratch.0,
            &format!("{}.receipt", "a".repeat(32)),
            &DuplicateEncoded,
            MAX_RECORD_BYTES
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), before);
}
