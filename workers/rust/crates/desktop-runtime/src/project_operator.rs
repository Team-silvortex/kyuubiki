pub use kyuubiki_protocol::{ProjectOperatorRequest, ProjectOperatorStatus};

/// Project package preparation has no account session and cannot dispatch or start an Agent.
pub fn project_operator(request: ProjectOperatorRequest) -> Result<ProjectOperatorStatus, String> {
    let expected = request.project_id.clone();
    let scope = request.scope.clone();
    let platform = request.platform.clone();
    let value = crate::native_installer::invoke(
        &crate::native_installer::binary().map_err(str::to_owned)?,
        "project-operator",
        None,
        &serde_json::to_value(request).map_err(|_| "project_operator_invalid_request")?,
        None,
    )
    .map_err(str::to_owned)?;
    let status: ProjectOperatorStatus =
        serde_json::from_value(value).map_err(|_| "project_operator_invalid_response")?;
    if status.schema_version != "kyuubiki.project-operators/v1"
        || status.project_id != expected
        || status.scope != scope
        || status.platform != platform
        || status.installed.len() > 64
        || status.enabled.len() > 64
        || status.installed.iter().any(|p| p.validate().is_err())
        || status.enabled.iter().any(|s| {
            s.package.validate().is_err()
                || !matches!(s.state.as_str(), "ready" | "unavailable_or_changed")
        })
    {
        return Err("project_operator_invalid_response".into());
    }
    Ok(status)
}
