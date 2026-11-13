//! Transport-independent package delivery; never performs store authorization or execution.
use kyuubiki_protocol::{
    OPERATOR_DEPLOYMENT_CHUNK_BYTES, OPERATOR_DEPLOYMENT_MAX_BYTES,
    OperatorDeploymentAction as Action, OperatorDeploymentChunk as Chunk,
    OperatorDeploymentRequest as Request, OperatorDeploymentSpec as Spec,
    OperatorDeploymentStatus as Status, ProjectOperatorPin as Pin,
};

fn check_current(current: &impl Fn() -> bool) -> Result<(), String> {
    if current() {
        Ok(())
    } else {
        Err("deployment_context_changed".into())
    }
}

fn terminal(status: &Status) -> bool {
    matches!(
        status.state.as_str(),
        "ready" | "installed_restart_required" | "version_conflict" | "activation_failed"
    )
}

fn source(
    chunk: &Chunk,
    pin: &Pin,
    platform: &str,
    offset: u64,
    expected: Option<&Spec>,
) -> Result<(), String> {
    let spec = &chunk.spec;
    if &spec.package != pin
        || chunk.offset != offset
        || spec.size_bytes == 0
        || spec.size_bytes > OPERATOR_DEPLOYMENT_MAX_BYTES
        || spec.manifest_json.is_empty()
        || spec.manifest_json.len() > OPERATOR_DEPLOYMENT_CHUNK_BYTES
        || chunk.data.is_empty()
        || chunk.data.len() > OPERATOR_DEPLOYMENT_CHUNK_BYTES
        || offset
            .checked_add(chunk.data.len() as u64)
            .is_none_or(|end| end > spec.size_bytes)
        || expected.is_some_and(|value| value != spec)
    {
        return Err("deployment_source_changed".into());
    }
    if spec.platform != platform {
        return Err("deployment_platform_mismatch".into());
    }
    Ok(())
}

fn transfer(status: &Status, spec: &Spec) -> Result<(), String> {
    if status.state == "missing"
        || (status.state == "uploading" && status.total_bytes != spec.size_bytes)
        || (status.total_bytes != 0 && status.total_bytes != spec.size_bytes)
    {
        return Err("deployment_invalid_receipt".into());
    }
    if status.state == "uploading" && !status.deployment_allowed {
        return Err("deployment_disabled".into());
    }
    Ok(())
}

/// Deliver an immutable project-enabled package, resuming only the Agent's verified cursor.
///
/// The caller supplies a bounded transport to one fixed endpoint (TLS and Agent API token
/// for remote connections), a native verified package reader, and an authority/context guard.
/// No store credentials, installer paths or engine dependencies enter this driver.
/// A transport error is returned without retry or abort: a later invocation starts with Inspect,
/// including when a successful Commit response was lost. Success can mean restart required or
/// version conflict or activation failure, not executable readiness. Inspect the returned state.
/// Cryptographic content/manifest validation and atomic installation remain the receiver's job.
pub fn deploy_operator_package(
    package: &Pin,
    platform: &str,
    mut exchange: impl FnMut(Request) -> Result<Status, String>,
    mut read: impl FnMut(u64) -> Result<Chunk, String>,
    current: impl Fn() -> bool,
    mut on_progress: impl FnMut(&Status),
) -> Result<Status, String> {
    package.validate()?;
    if platform.is_empty()
        || platform.len() > 128
        || !platform
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("deployment_platform_mismatch".into());
    }
    let mut send = |operation: Action, process: Option<&str>| -> Result<Status, String> {
        check_current(&current)?;
        let status = exchange(Request {
            package: package.clone(),
            expected_process_instance_id: process.map(str::to_owned),
            operation,
        })?;
        check_current(&current)?;
        status
            .validate()
            .map_err(|_| "deployment_invalid_receipt".to_string())?;
        if &status.package != package
            || status.process_instance_id.is_none()
            || process.is_some_and(|p| status.process_instance_id.as_deref() != Some(p))
            || (matches!(
                status.state.as_str(),
                "ready" | "installed_restart_required"
            ) && status.received_bytes != status.total_bytes)
        {
            return Err("deployment_invalid_receipt".into());
        }
        if status.platform != platform {
            return Err("deployment_platform_mismatch".into());
        }
        Ok(status)
    };
    let mut progress = |status: &Status| -> Result<(), String> {
        on_progress(status);
        check_current(&current)
    };
    let mut status = send(Action::Inspect, None)?;
    progress(&status)?;
    if terminal(&status) {
        return Ok(status);
    }
    if !status.deployment_allowed {
        return Err("deployment_disabled".into());
    }
    let process = status
        .process_instance_id
        .clone()
        .ok_or("deployment_invalid_receipt")?;
    let first = read(0)?;
    check_current(&current)?;
    source(&first, package, platform, 0, None)?;
    let spec = &first.spec;
    if status.state == "uploading" {
        transfer(&status, spec)?;
    }
    let previous = status.received_bytes;
    status = send(Action::Begin { spec: spec.clone() }, Some(&process))?;
    transfer(&status, spec)?;
    if status.state == "uploading" && status.received_bytes < previous {
        return Err("deployment_offset_conflict".into());
    }
    progress(&status)?;
    let mut blocks = 0;
    while status.state == "uploading" && status.received_bytes < status.total_bytes {
        // Bound a faulty reader producing tiny chunks without accumulating the full library.
        if blocks >= 513 {
            return Err("deployment_limit".into());
        }
        blocks += 1;
        let offset = status.received_bytes;
        let next;
        let chunk = if offset == 0 {
            &first
        } else {
            next = read(offset)?;
            &next
        };
        check_current(&current)?;
        source(chunk, package, platform, offset, Some(spec))?;
        status = send(
            Action::Chunk {
                offset,
                data: chunk.data.clone(),
            },
            Some(&process),
        )?;
        transfer(&status, spec)?;
        if status.state == "uploading" && status.received_bytes != offset + chunk.data.len() as u64
        {
            return Err("deployment_offset_conflict".into());
        }
        progress(&status)?;
    }
    if terminal(&status) {
        return Ok(status);
    }
    status = send(Action::Commit, Some(&process))?;
    transfer(&status, spec)?;
    if !terminal(&status) {
        return Err("deployment_invalid_receipt".into());
    }
    progress(&status)?;
    Ok(status)
}
