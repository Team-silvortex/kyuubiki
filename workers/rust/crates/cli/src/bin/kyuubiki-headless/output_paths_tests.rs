use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-path-guard-test-{}-{stamp}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn physical_resolution_normalizes_missing_suffixes_without_creating_directories() {
    let scratch = Scratch::new();
    let physical = scratch.0.canonicalize().unwrap();
    let missing = scratch.0.join("new/inner/.././report.json");
    assert_eq!(
        resolve_physical_path(&missing).unwrap(),
        physical.join("new/report.json")
    );
    assert!(!scratch.0.join("new").exists());
    let alias = scratch.0.join("new/../source.json");
    assert_eq!(
        resolve_physical_path(&alias).unwrap(),
        physical.join("source.json")
    );
}

#[test]
fn relative_and_absolute_spelling_reserve_the_same_prospective_output() {
    let relative = Path::new("target/future-output/nested/../report.json");
    let absolute = std::env::current_dir()
        .unwrap()
        .join("target/future-output/report.json");
    let error = check_artifact_paths(
        &[],
        &[("first", text(relative)), ("second", text(&absolute))],
    )
    .unwrap_err();
    assert!(error.starts_with(PATH_CONFLICT));
    assert!(error.contains("second"));
}

#[test]
fn every_run_output_is_checked_against_all_four_read_inputs_and_other_outputs() {
    let scratch = Scratch::new();
    let input_paths = ["workflow.json", "patch.json", "spec.json", "previous.json"]
        .map(|name| scratch.0.join(name).to_string_lossy().into_owned());
    for path in &input_paths {
        fs::write(path, b"original input").unwrap();
    }
    let base = Flags {
        positional: vec![input_paths[0].clone()],
        parameter_patch: Some(input_paths[1].clone()),
        research_round_spec: Some(input_paths[2].clone()),
        previous_round_evidence: Some(input_paths[3].clone()),
        ..Flags::default()
    };
    let mut flags = base;
    for input in &input_paths {
        for output in 0..4 {
            flags.report_out = None;
            flags.material_report_out = None;
            flags.parameter_patch_receipt_out = None;
            flags.research_round_out = None;
            *match output {
                0 => &mut flags.report_out,
                1 => &mut flags.material_report_out,
                2 => &mut flags.parameter_patch_receipt_out,
                _ => &mut flags.research_round_out,
            } = Some(input.clone());
            assert!(
                validate_paths(&flags, OutputScope::Run)
                    .unwrap_err()
                    .starts_with(PATH_CONFLICT)
            );
            assert_eq!(fs::read(input).unwrap(), b"original input");
        }
    }
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 4);
}

#[test]
fn sharing_read_only_inputs_is_allowed_but_each_output_must_be_distinct() {
    let scratch = Scratch::new();
    let source = scratch.0.join("input.json");
    let first = scratch.0.join("new/first.json");
    let second = scratch.0.join("new/second.json");
    fs::write(&source, b"input").unwrap();
    assert!(
        check_artifact_paths(
            &[("workflow", text(&source)), ("patch", text(&source))],
            &[("report", text(&first)), ("receipt", text(&second))]
        )
        .is_ok()
    );
    assert!(!first.parent().unwrap().exists());
}

#[test]
fn output_ancestors_conflict_in_both_orders_without_confusing_sibling_prefixes() {
    let scratch = Scratch::new();
    let parent = scratch.0.join("new/report.json");
    let child = scratch.0.join("new/report.json/child.json");
    let sibling = scratch.0.join("new/report.json-extra/child.json");
    for (left, right) in [(&parent, &child), (&child, &parent)] {
        assert!(
            check_artifact_paths(&[], &[("first", text(left)), ("second", text(right))]).is_err()
        );
    }
    assert!(
        check_artifact_paths(&[], &[("first", text(&parent)), ("second", text(&sibling))]).is_ok()
    );
    assert!(!scratch.0.join("new").exists());
}

#[test]
fn command_scopes_protect_only_the_inputs_and_outputs_the_command_uses() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.json");
    fs::write(&source, b"input").unwrap();
    let flags = Flags {
        positional: vec![text(&source).into()],
        out: Some(text(&source).into()),
        ..Flags::default()
    };
    assert!(validate_paths(&flags, OutputScope::Transform).is_err());
    assert!(validate_paths(&flags, OutputScope::BatchInput).is_ok());
    assert!(validate_paths(&flags, OutputScope::Run).is_ok());
    let flags = Flags {
        parameter_patch_receipt_out: Some(text(&source).into()),
        ..flags
    };
    assert!(validate_paths(&flags, OutputScope::BatchInput).is_err());
}

#[test]
fn unresolvable_parent_fails_closed_without_creating_artifacts() {
    let scratch = Scratch::new();
    let parent = scratch.0.join("not-directory");
    fs::write(&parent, b"original").unwrap();
    assert!(
        check_artifact_paths(&[], &[("report", text(&parent.join("report.json")))])
            .unwrap_err()
            .starts_with(PATH_CONFLICT)
    );
    assert_eq!(fs::read(parent).unwrap(), b"original");
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn existing_parent_links_are_resolved_before_parent_traversal_and_missing_suffixes() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    fs::create_dir_all(scratch.0.join("physical/nested")).unwrap();
    symlink("physical/nested", scratch.0.join("alias")).unwrap();
    let via_link = scratch.0.join("alias/../future/report.json");
    let direct = scratch.0.join("physical/future/report.json");
    assert_eq!(
        resolve_physical_path(&via_link).unwrap(),
        resolve_physical_path(&direct).unwrap()
    );
    assert!(
        check_artifact_paths(
            &[],
            &[("report", text(&direct)), ("receipt", text(&via_link))]
        )
        .is_err()
    );
    assert!(!scratch.0.join("physical/future").exists());
}

#[cfg(unix)]
#[test]
fn existing_hard_links_and_symlinks_cannot_hide_an_input_output_alias() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let source = scratch.0.join("source.json");
    let hard = scratch.0.join("hard.json");
    let link = scratch.0.join("link.json");
    fs::write(&source, b"original").unwrap();
    fs::hard_link(&source, &hard).unwrap();
    symlink(&source, &link).unwrap();
    for output in [&hard, &link] {
        assert!(
            check_artifact_paths(&[("input", text(&source))], &[("report", text(output))]).is_err()
        );
    }
    symlink("missing", scratch.0.join("dangling")).unwrap();
    assert!(resolve_physical_path(&scratch.0.join("dangling/report.json")).is_err());
    assert_eq!(fs::read(source).unwrap(), b"original");
}

#[cfg(unix)]
#[test]
fn publication_rechecks_new_file_identity_and_cleans_staging_without_replacing_input() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.json");
    let output = scratch.0.join("report.json");
    fs::write(&source, b"original input").unwrap();
    let flags = Flags {
        positional: vec![text(&source).into()],
        report_out: Some(text(&output).into()),
        ..Flags::default()
    };
    struct LinkDuringEncoding<'a> {
        source: &'a Path,
        output: &'a Path,
    }
    impl Serialize for LinkDuringEncoding<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            fs::hard_link(self.source, self.output).map_err(serde::ser::Error::custom)?;
            json!({"new report":true}).serialize(serializer)
        }
    }
    let error = write_guarded_json_file(
        &flags,
        OutputScope::Run,
        text(&output),
        &LinkDuringEncoding {
            source: &source,
            output: &output,
        },
    )
    .unwrap_err();
    assert!(error.starts_with(PATH_CONFLICT));
    assert_eq!(fs::read(&source).unwrap(), b"original input");
    assert_eq!(fs::read(&output).unwrap(), b"original input");
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}

#[cfg(any(windows, target_os = "macos"))]
#[test]
fn prospective_ascii_case_aliases_are_reserved_conservatively() {
    let scratch = Scratch::new();
    let upper = scratch.0.join("new/Report.json");
    let lower = scratch.0.join("new/report.json");
    assert!(
        check_artifact_paths(&[], &[("first", text(&upper)), ("second", text(&lower))]).is_err()
    );
    assert!(!scratch.0.join("new").exists());
    let parent = scratch.0.join("new/Report.JSON");
    let child = scratch.0.join("new/report.json/material.json");
    for (left, right) in [(&parent, &child), (&child, &parent)] {
        assert!(
            check_artifact_paths(&[], &[("first", text(left)), ("second", text(right))]).is_err()
        );
    }
}

#[test]
fn distinct_existing_report_can_still_be_explicitly_replaced() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.json");
    let output = scratch.0.join("report.json");
    fs::write(&source, b"original input").unwrap();
    fs::write(&output, b"old report").unwrap();
    let flags = Flags {
        positional: vec![text(&source).into()],
        report_out: Some(text(&output).into()),
        ..Flags::default()
    };
    write_guarded_json_file(
        &flags,
        OutputScope::Run,
        text(&output),
        &json!({"new":true}),
    )
    .unwrap();
    assert_eq!(fs::read(source).unwrap(), b"original input");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(output).unwrap()).unwrap(),
        json!({"new":true})
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}
