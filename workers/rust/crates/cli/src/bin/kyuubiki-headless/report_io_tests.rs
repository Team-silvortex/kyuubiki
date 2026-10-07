use super::*;
use serde::ser::{Error, SerializeSeq};
use serde_json::{Value, json};
use std::sync::{Arc, Barrier};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-output-test-{}-{}-{sequence}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn entries(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn streamed_publication_preserves_pretty_bytes_without_an_extra_newline() {
    let scratch = Scratch::new();
    let path = scratch.0.join("nested/report.json");
    let value = json!({
        "text": "\u{7814}\u{7a76}\n\u{0628}\u{062d}\u{062b}\t\"\\{{literal}}".repeat(8192),
        "nodes": (0..4096).map(|index| json!({"id":index,"value":index as f64 / 7.0})).collect::<Vec<_>>(),
        "values": [null, true, false, -12, 42.5, ""]
    });
    let expected = serde_json::to_vec_pretty(&value).unwrap();
    assert!(expected.len() > BUFFER_BYTES * 4);
    let resolved = write_json_file(path.to_str().unwrap(), &value).unwrap();
    assert_eq!(resolved, path.canonicalize().unwrap());
    assert_eq!(fs::read(&path).unwrap(), expected);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    write_json_file(path.to_str().unwrap(), &json!({"replacement":true})).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"{\n  \"replacement\": true\n}");
}

struct BrokenSerialization;

#[test]
fn streamed_typed_run_report_matches_previous_serializer_with_original_map_iteration_order() {
    use kyuubiki_headless_sdk::{
        build_template_document, normalize_workflow_document, run_batch_dry,
    };
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    let workflow = build_template_document("solve_wait_result", None).unwrap();
    let batch = normalize_workflow_document(&workflow).unwrap();
    let report = run_batch_dry(&batch, false, false);
    let expected = serde_json::to_vec_pretty(&report).unwrap();
    write_json_file(path.to_str().unwrap(), &report).unwrap();
    assert!(
        fs::read(&path).unwrap() == expected,
        "typed report serialization changed"
    );
}

impl Serialize for BrokenSerialization {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(None)?;
        sequence.serialize_element(&"x".repeat(BUFFER_BYTES * 4))?;
        Err(S::Error::custom("injected serialization failure"))
    }
}

#[test]
fn serialization_failure_after_large_partial_output_preserves_the_committed_file() {
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    fs::write(&path, b"{\"previous\":true}").unwrap();
    let error = write_json_file(path.to_str().unwrap(), &BrokenSerialization).unwrap_err();
    assert!(error.starts_with(OUTPUT_FAILURE));
    assert!(error.contains("injected serialization failure"));
    assert_eq!(fs::read(&path).unwrap(), b"{\"previous\":true}");
    assert_eq!(scratch.entries(), [path]);
}

#[test]
fn failed_first_publication_leaves_neither_a_partial_report_nor_a_pending_file() {
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    assert!(write_json_file(path.to_str().unwrap(), &BrokenSerialization).is_err());
    assert!(!path.exists());
    assert!(scratch.entries().is_empty());
}

#[test]
fn injected_write_failure_cleans_partial_staging_without_truncating_the_previous_report() {
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    fs::write(&path, b"old report").unwrap();
    let error = publish_with(&path, |writer| {
        writer.write_all(&vec![b'x'; BUFFER_BYTES * 2]).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"old report");
        assert!(scratch.entries().iter().any(|entry| {
            entry != &path && fs::metadata(entry).unwrap().len() > BUFFER_BYTES as u64
        }));
        Err("injected disk write failure".into())
    })
    .unwrap_err();
    assert!(error.contains("injected disk write failure"));
    assert_eq!(fs::read(&path).unwrap(), b"old report");
    assert_eq!(scratch.entries(), [path]);
}

#[test]
fn changed_destination_rejects_publication_and_cleans_its_pending_file() {
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    let error = publish_with(&path, |writer| {
        writer.write_all(b"{\"ok\":true}").unwrap();
        fs::create_dir(&path).unwrap();
        Ok(())
    })
    .unwrap_err();
    assert!(error.contains("regular output file"));
    assert!(path.is_dir());
    assert_eq!(scratch.entries(), [path]);
}

#[test]
fn concurrent_writers_use_distinct_staging_files_and_publish_only_complete_documents() {
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    let value = |id| json!({"writer":id,"data":"x".repeat(BUFFER_BYTES * 2)});
    write_json_file(path.to_str().unwrap(), &value(0)).unwrap();
    let barrier = Arc::new(Barrier::new(5));
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for id in 1..=4 {
            let path = &path;
            let barrier = barrier.clone();
            threads.push(scope.spawn(move || {
                barrier.wait();
                for _ in 0..4 {
                    write_json_file(path.to_str().unwrap(), &value(id)).unwrap();
                }
            }));
        }
        barrier.wait();
        while threads.iter().any(|thread| !thread.is_finished()) {
            let observed: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let id = observed["writer"].as_u64().unwrap();
            assert!(id <= 4);
            assert_eq!(observed, value(id));
        }
        for thread in threads {
            thread.join().unwrap();
        }
    });
    let observed: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(observed, value(observed["writer"].as_u64().unwrap()));
    assert_eq!(scratch.entries(), [path]);
}

#[cfg(unix)]
#[test]
fn published_research_output_is_private_to_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    write_json_file(path.to_str().unwrap(), &json!({"research":"private"})).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn read_only_committed_output_is_not_replaced_even_if_the_parent_is_writable() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let path = scratch.0.join("report.json");
    fs::write(&path, b"read-only report").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    let error = write_json_file(path.to_str().unwrap(), &json!({"new":true})).unwrap_err();
    assert!(error.contains("destination is read-only"));
    assert_eq!(fs::read(&path).unwrap(), b"read-only report");
    assert_eq!(scratch.entries(), [path]);
}

#[cfg(unix)]
#[test]
fn symlink_destinations_and_special_files_are_not_followed_or_replaced() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let original = scratch.0.join("original.json");
    fs::write(&original, b"old report").unwrap();
    let link = scratch.0.join("link.json");
    symlink(&original, &link).unwrap();
    let error = write_json_file(link.to_str().unwrap(), &json!({"new":true})).unwrap_err();
    assert!(error.contains("symlinks and special files"));
    assert_eq!(fs::read(original).unwrap(), b"old report");
    assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    assert!(write_json_file("/dev/null", &json!({"new":true})).is_err());
    assert_eq!(scratch.entries().len(), 2);
}

#[test]
fn directory_or_invalid_parent_is_rejected_without_overwriting_existing_data() {
    let scratch = Scratch::new();
    let parent_file = scratch.0.join("parent");
    fs::write(&parent_file, b"existing parent data").unwrap();
    let error =
        write_json_file(parent_file.join("report.json").to_str().unwrap(), &true).unwrap_err();
    assert!(error.starts_with(OUTPUT_FAILURE));
    assert_eq!(fs::read(&parent_file).unwrap(), b"existing parent data");
    assert!(write_json_file(scratch.0.to_str().unwrap(), &true).is_err());
    assert_eq!(scratch.entries(), [parent_file]);
}
