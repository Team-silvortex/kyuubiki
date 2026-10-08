use super::MaterialResearchBundle;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../../../../../schemas/examples.material-research-bundle.json");

const ARTIFACTS: &[(&str, &str)] = &[
    ("initial_exploration", "initial_exploration_sha256"),
    (
        "next_round_execution_plan",
        "next_round_execution_plan_sha256",
    ),
    ("next_exploration", "next_exploration_sha256"),
    ("chain", "chain_sha256"),
];

#[test]
fn standalone_and_native_rust_importers_share_the_exact_v1_digest_contract() {
    assert_eq!(
        include_str!("material_research_bundle_integrity.rs"),
        include_str!("../../../../../sdks/rust/src/material_research_bundle_integrity.rs")
    );
}

#[test]
fn verified_import_admits_original_fixture_and_formatting_changes() {
    let bundle = MaterialResearchBundle::from_json_verified(FIXTURE).unwrap();
    assert_eq!(bundle.study, "heat-spreader");
    let reformatted = FIXTURE.replace('\n', "\r\n").replace("  ", "\t");
    assert_eq!(
        MaterialResearchBundle::from_json_verified(&reformatted).unwrap(),
        bundle
    );
    let with_outer_space = format!(" \t\r\n{FIXTURE}\r\n ");
    assert_eq!(
        MaterialResearchBundle::from_json_verified(&with_outer_space).unwrap(),
        bundle
    );
}

#[test]
fn verified_import_rejects_each_stale_artifact_digest() {
    for &(artifact, checksum) in ARTIFACTS {
        let original = raw_field(FIXTURE, artifact);
        let changed = format!(
            "{},\"integrity_probe\":true}}",
            original.trim_end().strip_suffix('}').unwrap()
        );
        let text = replace_field(FIXTURE, artifact, &changed);
        let error = rejection(&text);
        assert!(error.contains(checksum), "{artifact}: {error}");
        assert!(error.contains("checksum mismatch"), "{artifact}: {error}");

        let text = replace_digest(FIXTURE, checksum, &"0".repeat(64));
        let error = rejection(&text);
        assert!(error.contains(checksum), "{artifact}: {error}");
        assert!(error.contains("checksum mismatch"), "{artifact}: {error}");
    }
}

#[test]
fn verified_import_preserves_numeric_tokens_key_order_and_string_escapes() {
    let chain = r#"{"schema_version":"kyuubiki.material-exploration-chain/v1","round_count":1,"stop_reason":"risk_mitigation_required","probe":{"z":1.0,"a":-0.0,"tiny":1e-30,"large":9007199254740993},"text":"\u4f60\u597d"}"#;
    let text = signed_chain(chain);
    MaterialResearchBundle::from_json_verified(&text).unwrap();
    for changed in [
        chain.replace("1.0", "1"),
        chain.replace("-0.0", "0"),
        chain.replace("1e-30", "1.0e-30"),
        chain.replace(r#""z":1.0,"a":-0.0"#, r#""a":-0.0,"z":1.0"#),
        chain.replace(r"\u4f60\u597d", "\u{4f60}\u{597d}"),
        chain.replace("9007199254740993", "9007199254740992"),
    ] {
        let changed = replace_field(&text, "chain", &changed);
        assert!(rejection(&changed).contains("chain_sha256"));
    }
}

#[test]
fn verified_import_preserves_quoted_whitespace_unicode_and_escape_parity() {
    let chain = r#"{"schema_version":"kyuubiki.material-exploration-chain/v1","round_count":1,"stop_reason":"risk_mitigation_required","text":"材料 A  B\tC\nD \"Q\" \\ end","nested":[{"escaped":"backslash \\\\ then quote","empty":""},true,null]}"#;
    let text = signed_chain(chain);
    let bundle = MaterialResearchBundle::from_json_verified(&text).unwrap();
    assert_eq!(bundle.chain["text"], "材料 A  B\tC\nD \"Q\" \\ end");
    let formatted = chain
        .replacen("{", "{ \r\n\t", 1)
        .replace(",\"nested\"", ",\n\t\"nested\"");
    let formatted = replace_field(&text, "chain", &formatted);
    assert_eq!(
        MaterialResearchBundle::from_json_verified(&formatted).unwrap(),
        bundle
    );
    for changed in [
        chain.replace("A  B", "A B"),
        chain.replace(r"\tC", " C"),
        chain.replace("材料", "材质"),
        chain.replace(r"\\ end", r"\\\\ end"),
    ] {
        assert!(rejection(&replace_field(&text, "chain", &changed)).contains("chain_sha256"));
    }
}

#[test]
fn verified_import_accepts_correctly_rehashed_changes_without_claiming_authenticity() {
    let chain = r#"{"schema_version":"kyuubiki.material-exploration-chain/v1","round_count":1,"stop_reason":"risk_mitigation_required","note":"caller edited this artifact"}"#;
    let text = signed_chain(chain);
    let bundle = MaterialResearchBundle::from_json_verified(&text).unwrap();
    assert_eq!(bundle.chain["note"], "caller edited this artifact");
    assert_eq!(bundle.posture, "screening_research_bundle");
    let metadata_only = text.replace("2026-07-10", "2026-07-11");
    MaterialResearchBundle::from_json_verified(&metadata_only).unwrap();
}

#[test]
fn verified_import_still_rejects_bad_structure_with_matching_digests() {
    for chain in [
        "null",
        r#"{"schema_version":"wrong","round_count":1,"stop_reason":"risk_mitigation_required"}"#,
        r#"{"schema_version":"kyuubiki.material-exploration-chain/v1","round_count":1,"stop_reason":"unrelated"}"#,
    ] {
        let text = signed_chain(chain);
        assert!(MaterialResearchBundle::from_json_verified(&text).is_err());
    }
    assert!(
        rejection(&FIXTURE.replace("screening_research_bundle", "qualified_research_bundle"))
            .contains("posture")
    );
}

#[test]
fn verified_import_rejects_duplicate_top_level_fields_and_invalid_json() {
    for &(artifact, _) in ARTIFACTS {
        let duplicate = format!(
            "{},\"{artifact}\":{} }}",
            FIXTURE.trim_end().strip_suffix('}').unwrap(),
            raw_field(FIXTURE, artifact)
        );
        assert!(rejection(&duplicate).contains("duplicate field"));
    }
    let duplicate_checksums = format!(
        "{},\"artifact_checksums\":{} }}",
        FIXTURE.trim_end().strip_suffix('}').unwrap(),
        raw_field(FIXTURE, "artifact_checksums")
    );
    assert!(rejection(&duplicate_checksums).contains("duplicate field"));
    for text in ["", "[]", "null", "{", &format!("{FIXTURE} false")] {
        assert!(MaterialResearchBundle::from_json_verified(text).is_err());
    }
}

#[test]
fn verified_import_borrows_original_document_without_modifying_it() {
    let original = FIXTURE.to_owned();
    let before = original.clone();
    MaterialResearchBundle::from_json_verified(&original).unwrap();
    assert_eq!(original, before);
    let corrupted = replace_digest(&original, "chain_sha256", &"0".repeat(64));
    let before = corrupted.clone();
    assert!(MaterialResearchBundle::from_json_verified(&corrupted).is_err());
    assert_eq!(corrupted, before);
}

fn rejection(text: &str) -> String {
    MaterialResearchBundle::from_json_verified(text)
        .expect_err("import must reject")
        .to_string()
}

fn raw_field<'a>(text: &'a str, field: &str) -> &'a str {
    let fields: BTreeMap<&str, &RawValue> = serde_json::from_str(text).unwrap();
    fields[field].get()
}

fn replace_field(text: &str, field: &str, replacement: &str) -> String {
    let raw = raw_field(text, field);
    let start = raw.as_ptr() as usize - text.as_ptr() as usize;
    format!(
        "{}{replacement}{}",
        &text[..start],
        &text[start + raw.len()..]
    )
}

fn replace_digest(text: &str, field: &str, digest: &str) -> String {
    let checksums = raw_field(text, "artifact_checksums");
    let updated = replace_field(checksums, field, &format!("\"{digest}\""));
    replace_field(text, "artifact_checksums", &updated)
}

fn signed_chain(compact_chain: &str) -> String {
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{compact_chain}\n").as_bytes())
    );
    let text = replace_field(FIXTURE, "chain", compact_chain);
    replace_digest(&text, "chain_sha256", &digest)
}
