use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kyuubiki_kcore::{export_path, extract_path, inspect_path, verify_path};
use serde_json::{Value, json};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

fn fixture(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kyuubiki-kcore-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn write_spec(root: &Path, metadata: Value) -> PathBuf {
    fs::create_dir_all(root).expect("create fixture");
    fs::write(root.join("result.json"), br#"{"temperature": 312.5}"#).expect("write result");
    let spec = json!({
        "schema_version": "kyuubiki.kcore-export/v1",
        "core_id": "thermal-study",
        "title": "Thermal study",
        "kind": "simulation-result",
        "producer": {"name": "test", "version": "1"},
        "artifacts": [
            {
                "id": "result",
                "role": "result.field",
                "media_type": "application/json",
                "source": "result.json",
                "encoding": "json",
                "unit": "K"
            },
            {
                "id": "result-copy",
                "role": "evidence.validation",
                "media_type": "application/json",
                "source": "result.json",
                "encoding": "json"
            }
        ],
        "contracts": [{
            "name": "thermal-result",
            "schema_version": "example.thermal-result/v1",
            "artifact_id": "result"
        }],
        "entrypoints": ["result"],
        "provenance": {"execution": "native"},
        "metadata": metadata
    });
    let path = root.join("export.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&spec).expect("serialize spec"),
    )
    .expect("write spec");
    path
}

#[test]
fn exports_deduplicates_verifies_and_extracts_without_source_paths() {
    let root = fixture("roundtrip");
    let spec = write_spec(&root, json!({"domain": "thermal"}));
    let first = root.join("first.kcore");
    let second = root.join("second.kcore");

    let report = export_path(&spec, &first).expect("export kcore");
    assert_eq!(report.artifact_count, 2);
    assert_eq!(report.object_count, 1);
    assert_eq!(fs::read(&first).expect("read first"), {
        export_path(&spec, &second).expect("repeat export");
        fs::read(&second).expect("read second")
    });

    let inspection = inspect_path(&first).expect("inspect kcore");
    assert_eq!(inspection.core_id, "thermal-study");
    assert_eq!(inspection.artifact_count, 2);
    let verification = verify_path(&first).expect("verify kcore");
    assert_eq!(verification.object_count, 1);

    let extracted = root.join("extracted");
    extract_path(&first, &extracted).expect("extract kcore");
    let manifest = fs::read_to_string(extracted.join("manifest.json")).expect("read manifest");
    assert!(!manifest.contains(root.to_string_lossy().as_ref()));
    assert!(!manifest.contains("result.json"));
    assert_eq!(
        fs::read_to_string(extracted.join("mimetype")).expect("read mimetype"),
        "application/vnd.kyuubiki.kcore"
    );
    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn rejects_tampered_and_unreferenced_objects() {
    let root = fixture("tamper");
    let spec = write_spec(&root, json!({}));
    let original = root.join("original.kcore");
    export_path(&spec, &original).expect("export kcore");

    let tampered = root.join("tampered.kcore");
    rewrite_archive(&original, &tampered, true, false, false);
    let error = verify_path(&tampered).expect_err("tampered object must fail");
    assert!(error.contains("size mismatch") || error.contains("digest mismatch"));
    assert!(
        extract_path(&tampered, root.join("tampered-output"))
            .expect_err("tampered object must not extract")
            .contains("mismatch")
    );

    let extra = root.join("extra.kcore");
    rewrite_archive(&original, &extra, false, true, false);
    assert!(
        inspect_path(&extra)
            .expect_err("unreferenced object must fail")
            .contains("unreferenced")
    );

    let missing = root.join("missing.kcore");
    rewrite_archive(&original, &missing, false, false, true);
    assert!(
        inspect_path(&missing)
            .expect_err("missing object must fail inspection")
            .contains("is missing")
    );
    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn rejects_host_absolute_paths_before_writing_output() {
    let root = fixture("host-path");
    let host_path = ["", "private", "research", "result.json"].join("/");
    let spec = write_spec(&root, json!({"private_source": host_path}));
    let output = root.join("blocked.kcore");
    assert!(
        export_path(&spec, &output)
            .expect_err("host path must fail")
            .contains("host-absolute")
    );
    assert!(!output.exists());
    fs::remove_dir_all(root).expect("clean fixture");
}

fn rewrite_archive(
    source: &Path,
    output: &Path,
    tamper_object: bool,
    add_extra: bool,
    remove_object: bool,
) {
    let input = File::open(source).expect("open source archive");
    let mut archive = ZipArchive::new(input).expect("read source archive");
    let target = File::create(output).expect("create rewritten archive");
    let mut writer = ZipWriter::new(target);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("read source entry");
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("read entry bytes");
        if remove_object && entry.name().starts_with("objects/") {
            continue;
        }
        if tamper_object && entry.name().starts_with("objects/") {
            bytes.push(b'!');
        }
        let entry_options = if entry.name() == "mimetype" {
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
        } else {
            options
        };
        writer
            .start_file(entry.name(), entry_options)
            .expect("start entry");
        writer.write_all(&bytes).expect("write entry");
    }
    if add_extra {
        writer
            .start_file("objects/ff/unreferenced", options)
            .expect("start extra");
        writer.write_all(b"extra").expect("write extra");
    }
    writer.finish().expect("finish rewritten archive");
}
