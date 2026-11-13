use kyuubiki_headless_sdk::{
    OperatorDeploymentAction as Action, OperatorDeploymentChunk as Chunk,
    OperatorDeploymentRequest as Request, OperatorDeploymentSpec as Spec,
    OperatorDeploymentStatus as Status, ProjectOperatorEntry, ProjectOperatorPin as Pin,
    deploy_operator_package,
};
use std::cell::Cell;

fn pin() -> Pin {
    Pin {
        package_id: "demo".into(),
        package_version: "1.0.0".into(),
        manifest_sha256: "a".repeat(64),
        entrypoint_sha256: "b".repeat(64),
        operators: vec![ProjectOperatorEntry {
            operator_id: "extract.demo".into(),
            kind: "extract".into(),
        }],
    }
}
fn spec(platform: &str) -> Spec {
    Spec {
        package: pin(),
        platform: platform.into(),
        manifest_json: "{}".into(),
        size_bytes: 4,
    }
}
fn read(platform: &str, offset: u64) -> Result<Chunk, String> {
    Ok(Chunk {
        spec: spec(platform),
        offset,
        data: [1, 2, 3, 4][offset as usize..].to_vec(),
    })
}
fn status(platform: &str, state: &str, bytes: u64) -> Status {
    Status {
        schema_version: "kyuubiki.operator-deployment/v1".into(),
        package: pin(),
        platform: platform.into(),
        state: state.into(),
        installed: matches!(state, "ready" | "installed_restart_required").then(pin),
        active: (state == "ready").then(pin),
        received_bytes: bytes,
        total_bytes: if state == "missing" { 0 } else { 4 },
        process_instance_id: Some("process-1".into()),
        deployment_allowed: true,
        activation_error: None,
    }
}
fn respond(platform: &str, request: Request) -> Status {
    assert_eq!(request.package, pin());
    if !matches!(request.operation, Action::Inspect) {
        assert_eq!(
            request.expected_process_instance_id.as_deref(),
            Some("process-1")
        );
    }
    match request.operation {
        Action::Inspect => status(platform, "missing", 0),
        Action::Begin { spec: input } => {
            assert_eq!(input, spec(platform));
            status(platform, "uploading", 0)
        }
        Action::Chunk { offset, data } => status(platform, "uploading", offset + data.len() as u64),
        Action::Commit => status(platform, "installed_restart_required", 4),
        Action::Abort => panic!("must not automatically abort or restart"),
    }
}

#[test]
fn linux_variants_deploy_without_store_or_solver_dependencies() {
    for platform in ["linux-x86_64", "linux-aarch64"] {
        let mut calls = 0;
        let mut progress = Vec::new();
        let result = deploy_operator_package(
            &pin(),
            platform,
            |request| {
                calls += 1;
                Ok(respond(platform, request))
            },
            |offset| read(platform, offset),
            || true,
            |s| progress.push(s.state.clone()),
        )
        .unwrap();
        assert_eq!(result.state, "installed_restart_required");
        assert_eq!(calls, 4);
        assert_eq!(
            progress,
            [
                "missing",
                "uploading",
                "uploading",
                "installed_restart_required"
            ]
        );
    }
}

#[test]
fn resumes_from_confirmed_remote_cursor_without_replaying_prefix() {
    let platform = "linux-x86_64";
    let mut sent = Vec::new();
    let result = deploy_operator_package(
        &pin(),
        platform,
        |request| {
            if matches!(request.operation, Action::Inspect | Action::Begin { .. }) {
                return Ok(status(platform, "uploading", 2));
            }
            if let Action::Chunk { offset, ref data } = request.operation {
                sent.push((offset, data.clone()));
            }
            Ok(respond(platform, request))
        },
        |offset| read(platform, offset),
        || true,
        |_| {},
    )
    .unwrap();
    assert_eq!(sent, [(2, vec![3, 4])]);
    assert_eq!(result.state, "installed_restart_required");
}

#[test]
fn rejects_drifting_receipts_before_any_further_mutation() {
    let platform = "linux-x86_64";
    for mode in [
        "platform",
        "process",
        "inspect-size",
        "begin-size",
        "chunk-size",
        "offset",
        "disabled",
        "commit-uploading",
        "commit-missing",
        "partial-terminal",
    ] {
        let mut calls = 0;
        let error = deploy_operator_package(
            &pin(),
            platform,
            |request| {
                calls += 1;
                let inspect = matches!(request.operation, Action::Inspect);
                let begin = matches!(request.operation, Action::Begin { .. });
                let chunk = matches!(request.operation, Action::Chunk { .. });
                let commit = matches!(request.operation, Action::Commit);
                let mut s = respond(platform, request);
                if mode == "platform" && begin {
                    s.platform = "linux-aarch64".into();
                }
                if mode == "process" && begin {
                    s.process_instance_id = Some("replaced".into());
                }
                if mode == "inspect-size" && inspect {
                    s = status(platform, "uploading", 0);
                    s.total_bytes = 8;
                }
                if (mode == "begin-size" && begin) || (mode == "chunk-size" && chunk) {
                    s.total_bytes = 8;
                }
                if mode == "offset" && chunk {
                    s.received_bytes = 3;
                }
                if mode == "disabled" && begin {
                    s.deployment_allowed = false;
                }
                if mode == "commit-uploading" && commit {
                    s = status(platform, "uploading", 4);
                }
                if mode == "commit-missing" && commit {
                    s = status(platform, "missing", 0);
                }
                if mode == "partial-terminal" && inspect {
                    s = status(platform, "installed_restart_required", 2);
                }
                Ok(s)
            },
            |offset| read(platform, offset),
            || true,
            |_| {},
        )
        .unwrap_err();
        assert!(error.starts_with("deployment_"), "{mode}: {error}");
        let expected = match mode {
            "inspect-size" | "partial-terminal" => 1,
            "chunk-size" | "offset" => 3,
            "commit-uploading" | "commit-missing" => 4,
            _ => 2,
        };
        assert_eq!(calls, expected, "{mode}");
    }
}

#[test]
fn invalid_source_never_begins_upload() {
    let platform = "linux-x86_64";
    for mode in [
        "offset", "empty", "overrun", "size", "manifest", "package", "platform",
    ] {
        let mut calls = 0;
        let error = deploy_operator_package(
            &pin(),
            platform,
            |request| {
                calls += 1;
                Ok(respond(platform, request))
            },
            |offset| {
                let mut part = read(platform, offset)?;
                match mode {
                    "offset" => part.offset = 1,
                    "empty" => part.data.clear(),
                    "overrun" => part.data.push(5),
                    "size" => part.spec.size_bytes = 33554433,
                    "manifest" => part.spec.manifest_json = "x".repeat(65537),
                    "package" => part.spec.package.package_version = "99".into(),
                    _ => part.spec.platform = "macos-aarch64".into(),
                }
                Ok(part)
            },
            || true,
            |_| {},
        )
        .unwrap_err();
        assert!(error.starts_with("deployment_"), "{mode}: {error}");
        assert_eq!(calls, 1, "{mode}");
    }
}

#[test]
fn lost_response_requires_reinspection_not_blind_replay() {
    let platform = "linux-x86_64";
    for lost_commit in [false, true] {
        let mut remote = status(platform, "missing", 0);
        let mut dropped = false;
        let mut chunks = 0;
        let mut commits = 0;
        let mut exchange = |request: Request| {
            let lost = if lost_commit {
                matches!(request.operation, Action::Commit)
            } else {
                matches!(request.operation, Action::Chunk { .. })
            };
            match request.operation {
                Action::Inspect => {}
                Action::Begin { .. } => {
                    remote.state = "uploading".into();
                    remote.total_bytes = 4;
                }
                Action::Chunk { offset, data } => {
                    chunks += 1;
                    remote.received_bytes = offset + data.len() as u64;
                }
                Action::Commit => {
                    commits += 1;
                    remote = status(platform, "installed_restart_required", 4);
                }
                Action::Abort => panic!("must preserve resumable upload"),
            }
            if lost && !dropped {
                dropped = true;
                return Err("response_lost".into());
            }
            Ok(remote.clone())
        };
        let error = deploy_operator_package(
            &pin(),
            platform,
            &mut exchange,
            |offset| read(platform, offset),
            || true,
            |_| {},
        )
        .unwrap_err();
        assert_eq!(error, "response_lost");
        let result = deploy_operator_package(
            &pin(),
            platform,
            &mut exchange,
            |offset| read(platform, offset),
            || true,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.state, "installed_restart_required");
        assert_eq!((chunks, commits), (1, 1));
    }
}

#[test]
fn context_changes_after_read_or_progress_stop_mutations() {
    let platform = "linux-x86_64";
    for during_read in [true, false] {
        let current = Cell::new(true);
        let mut calls = 0;
        let result = deploy_operator_package(
            &pin(),
            platform,
            |request| {
                calls += 1;
                Ok(respond(platform, request))
            },
            |offset| {
                current.set(false);
                read(platform, offset)
            },
            || current.get(),
            |_| {
                if !during_read {
                    current.set(false);
                }
            },
        );
        assert_eq!(result.unwrap_err(), "deployment_context_changed");
        assert_eq!(calls, 1);
    }
}

#[test]
fn source_change_and_backwards_resume_cursor_do_not_clean_or_continue() {
    let platform = "linux-x86_64";
    for source_change in [true, false] {
        let mut calls = 0;
        let error = deploy_operator_package(
            &pin(),
            platform,
            |request| {
                calls += 1;
                let bytes = match request.operation {
                    Action::Inspect if !source_change => 2,
                    Action::Chunk { .. } => 2,
                    Action::Inspect | Action::Begin { .. } => 0,
                    _ => panic!("must not commit or clean an inconsistent upload"),
                };
                Ok(status(platform, "uploading", bytes))
            },
            |offset| {
                let mut chunk = read(platform, offset)?;
                chunk.data.truncate(2);
                if offset > 0 {
                    chunk.spec.manifest_json = "changed".into();
                }
                Ok(chunk)
            },
            || true,
            |_| {},
        )
        .unwrap_err();
        assert_eq!(
            error,
            if source_change {
                "deployment_source_changed"
            } else {
                "deployment_offset_conflict"
            }
        );
        assert_eq!(calls, if source_change { 3 } else { 2 });
    }
}

#[test]
fn activation_failure_is_terminal_and_cannot_claim_readiness() {
    let mut value =
        serde_json::to_value(status("linux-x86_64", "installed_restart_required", 4)).unwrap();
    value["state"] = serde_json::json!("activation_failed");
    value["activation_error"] = serde_json::json!("operator_package_activation_failed");
    let failed: Status = serde_json::from_value(value.clone()).unwrap();
    let result = deploy_operator_package(
        &pin(),
        "linux-x86_64",
        |r| {
            assert!(matches!(r.operation, Action::Inspect));
            Ok(failed.clone())
        },
        |_| panic!("do not reupload failed activation"),
        || true,
        |_| {},
    )
    .unwrap();
    assert_eq!(result.state, "activation_failed");
    for mode in [
        "ready",
        "missing_error",
        "unknown_error",
        "active",
        "uninstalled",
        "partial",
    ] {
        let mut invalid = value.clone();
        match mode {
            "ready" => {
                invalid["state"] = serde_json::json!("ready");
                invalid["active"] = serde_json::to_value(pin()).unwrap();
            }
            "missing_error" => {
                invalid.as_object_mut().unwrap().remove("activation_error");
            }
            "unknown_error" => {
                invalid["activation_error"] = serde_json::json!("untrusted text or path")
            }
            "active" => invalid["active"] = serde_json::to_value(pin()).unwrap(),
            "uninstalled" => invalid["installed"] = serde_json::Value::Null,
            _ => invalid["received_bytes"] = serde_json::json!(2),
        }
        assert!(
            serde_json::from_value::<Status>(invalid)
                .unwrap()
                .validate()
                .is_err(),
            "{mode}"
        );
    }
}
