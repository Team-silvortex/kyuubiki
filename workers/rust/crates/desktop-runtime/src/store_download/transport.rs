use super::{Result, trust, types::*};
use crate::store_identity::StoreSessionLease;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use zeroize::Zeroizing;

fn error_code(
    response: &mut ureq::http::Response<ureq::Body>,
    session: &StoreSessionLease,
) -> &'static str {
    if response.status() == 401 {
        session.reject();
        return "login_required";
    }
    let value = response
        .body_mut()
        .with_config()
        .limit(8192)
        .read_to_vec()
        .ok()
        .and_then(|body| serde_json::from_slice::<Value>(&body).ok());
    let code = value
        .as_ref()
        .and_then(|v| v.pointer("/error/code"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match code {
        "quota_exhausted" => "quota_exhausted",
        "downloads_unavailable" => "downloads_unavailable",
        "metering_policy_unconfirmed" => "metering_policy_unconfirmed",
        "grant_revoked" => "grant_revoked",
        "grant_expired" => "grant_expired",
        "release_recalled" => "release_recalled",
        "release_withdrawn" => "release_withdrawn",
        "artifact_mismatch" => "artifact_mismatch",
        "ticket_expired" => "ticket_expired",
        "invalid_ticket" => "invalid_ticket",
        "idempotency_conflict" => "idempotency_conflict",
        "forbidden" | "access_denied" => "permission_denied",
        "rate_limited" => "rate_limited",
        _ => {
            if response.status() == 429 {
                "rate_limited"
            } else {
                "store_request_rejected"
            }
        }
    }
}
pub(super) fn response<T: DeserializeOwned>(
    response: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    session: &StoreSessionLease,
) -> Result<T> {
    let mut response = response.map_err(|_| "store_connection_failed")?;
    if !matches!(response.status().as_u16(), 200 | 201) {
        return Err(error_code(&mut response, session));
    }
    let bytes = Zeroizing::new(
        response
            .body_mut()
            .with_config()
            .limit(262144)
            .read_to_vec()
            .map_err(|_| "invalid_store_response")?,
    );
    serde_json::from_slice(&bytes).map_err(|_| "invalid_store_response")
}
pub(super) fn get<T: DeserializeOwned>(
    session: &StoreSessionLease,
    path: &str,
    auth: bool,
) -> Result<T> {
    session.check()?;
    let mut request = session.http.get(format!("{}{path}", session.issuer));
    if auth {
        request = request.header(
            "Authorization",
            format!("Bearer {}", session.access.as_str()),
        );
    }
    let value = response(request.call(), session)?;
    session.check()?;
    Ok(value)
}
#[derive(Deserialize)]
struct Identity {
    issuer: String,
    subject: String,
    session_expires_at: u64,
    access_token_expires_at: u64,
}

pub(super) fn preflight(
    session: &StoreSessionLease,
    selection: &DownloadSelection,
    attempted: bool,
) -> Result<(Vec<ArtifactFile>, Quota, bool, u64)> {
    selection.validate()?;
    let capabilities: Value = get(session, "/store/v1/capabilities", false)?;
    let is = |path: &str, expected: Value| capabilities.pointer(path) == Some(&expected);
    if !is("/api_version", json!("store/v1"))
        || !is("/identity/issuer", json!(session.issuer))
        || !is("/identity/public_client_profile", json!("kyuubiki_desktop"))
        || !is("/identity/mode", json!("oidc_authorization_code_pkce"))
    {
        return Err("store_identity_mismatch");
    }
    if !is("/features/downloads", json!(true)) {
        return Err("downloads_unavailable");
    }
    let metered = ["operator", "workflow_template"].contains(&selection.kind.as_str());
    if !is("/policy/quota_event", json!("grant_issued"))
        || !is("/policy/renewal_charges_quota", json!(false))
        || (metered && !is("/policy/metering_policy_confirmed", json!(true)))
    {
        return Err("metering_policy_unconfirmed");
    }
    let retention = capabilities
        .pointer("/policy/idempotency_retention_seconds")
        .and_then(Value::as_u64)
        .filter(|n| *n > 60)
        .ok_or("invalid_store_response")?
        .min(2_592_000);
    let identity: Identity = get(session, "/store/v1/me", true)?;
    if identity.issuer != session.issuer
        || identity.subject != session.subject
        || identity.session_expires_at <= now()
        || identity.access_token_expires_at <= now()
    {
        return Err("store_identity_mismatch");
    }
    let rights: Entitlements = get(session, "/store/v1/me/entitlements", true)?;
    if rights.subject != session.subject
        || !rights.download_kinds.contains(&selection.kind)
        || !rights.block_reasons.is_empty()
        || rights.metered_kinds.contains(&selection.kind) != metered
    {
        return Err("permission_denied");
    }
    if !attempted
        && metered
        && !rights.quota.unlimited
        && rights.quota.remaining.is_none_or(|n| n == 0)
    {
        return Err("quota_exhausted");
    }
    let release: Release = get(
        session,
        &format!(
            "/store/v1/assets/{}/{}/releases/{}",
            selection.kind, selection.asset_id, selection.version
        ),
        false,
    )?;
    if release.kind != selection.kind
        || release.asset_id != selection.asset_id
        || release.version != selection.version
    {
        return Err("artifact_mismatch");
    }
    if release.status != "published" && !(attempted && release.status == "withdrawn") {
        return Err(if release.status == "recalled" {
            "release_recalled"
        } else {
            "release_withdrawn"
        });
    }
    let mut artifacts: Vec<_> = release
        .artifacts
        .into_iter()
        .filter(|a| a.platform == selection.platform)
        .collect();
    if artifacts.len() != 1 {
        return Err("platform_unavailable");
    }
    let files = artifacts.remove(0).files;
    validate_files(&files)?;
    for (path, count) in [
        (
            "/artifact_limits/maximum_file_bytes",
            files.iter().map(|f| f.size_bytes).max().unwrap_or(0),
        ),
        (
            "/artifact_limits/maximum_package_bytes",
            files.iter().map(|f| f.size_bytes).sum(),
        ),
        (
            "/artifact_limits/maximum_files_per_platform",
            files.len() as u64,
        ),
    ] {
        if capabilities
            .pointer(path)
            .and_then(Value::as_u64)
            .is_none_or(|limit| count > limit)
        {
            return Err("package_too_large");
        }
    }
    let roots = get(session, "/store/v1/trust-roots", false)?;
    trust::verify(roots, selection, &files)?;
    Ok((files, rights.quota, metered, retention - 60))
}

pub(super) fn grant(session: &StoreSessionLease, record: &Record) -> Result<Grant> {
    session.check()?;
    if record.grant_id.is_none() && record.replay_until.is_none_or(|until| until <= now()) {
        return Err("authorization_recovery_expired");
    }
    let request = session
        .http
        .post(format!(
            "{}/store/v1/download-grants{}",
            session.issuer,
            record
                .grant_id
                .as_ref()
                .map_or(String::new(), |id| format!("/{id}/renew"))
        ))
        .header(
            "Authorization",
            format!("Bearer {}", session.access.as_str()),
        );
    let grant: Grant = if record.grant_id.is_some() {
        response(request.send_empty(), session)?
    } else {
        response(request.header("Idempotency-Key",format!("kyuubiki-{}",record.id)).send_json(json!({
            "kind":record.selection.kind,"asset_id":record.selection.asset_id,"version":record.selection.version,
            "platform":record.selection.platform,"files":record.files.iter().map(|f|json!({"path":f.path,"sha256":f.sha256})).collect::<Vec<_>>()
        })),session)?
    };
    session.check()?;
    validate_grant(session, &grant, record)?;
    Ok(grant)
}
fn validate_grant(session: &StoreSessionLease, grant: &Grant, record: &Record) -> Result<()> {
    if !grant_id(&grant.grant_id)
        || record
            .grant_id
            .as_ref()
            .is_some_and(|id| id != &grant.grant_id)
        || grant.kind != record.selection.kind
        || grant.asset_id != record.selection.asset_id
        || grant.version != record.selection.version
        || grant.platform != record.selection.platform
        || grant.status != "active"
        || grant.files.len() != record.files.len()
        || grant.created_at > now().saturating_add(60)
        || grant.renew_until <= now()
        || grant.ticket_expires_at <= now()
        || grant.ticket_expires_at > grant.renew_until
        || grant.quota.event != "grant_issued"
        || grant.quota.units > 1
        || grant.quota.charged != (grant.quota.units == 1)
        || ((!record.metered || grant.quota.unlimited) && grant.quota.charged)
        || record
            .outcome
            .as_ref()
            .is_some_and(|q| q.charged != grant.quota.charged || q.units != grant.quota.units)
    {
        return Err("grant_mismatch");
    }
    let mut paths = std::collections::HashSet::new();
    for file in &grant.files {
        if !paths.insert(&file.path)
            || !record.files.iter().any(|f| {
                f.path == file.path && f.sha256 == file.sha256 && f.size_bytes == file.size_bytes
            })
            || file.ticket_expires_at <= now()
            || file.ticket_expires_at > grant.renew_until
        {
            return Err("grant_mismatch");
        }
        ticket_url(session, &grant.grant_id, &file.download_url)?;
    }
    Ok(())
}
fn ticket_url(session: &StoreSessionLease, id: &str, text: &str) -> Result<()> {
    let url = url::Url::parse(text).map_err(|_| "unsafe_download_url")?;
    if text.len() > 8192
        || url.origin().ascii_serialization() != session.issuer
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.path() != format!("/store/v1/downloads/{id}")
    {
        return Err("unsafe_download_url");
    }
    let query: Vec<_> = url.query_pairs().collect();
    if query.len() != 1
        || query[0].0 != "ticket"
        || query[0].1.is_empty()
        || query[0].1.len() > 4096
    {
        return Err("unsafe_download_url");
    }
    Ok(())
}
pub(super) fn bytes(
    session: &StoreSessionLease,
    file: &GrantFile,
) -> Result<ureq::http::Response<ureq::Body>> {
    session.check()?;
    // Intentionally no Authorization header on ticket requests, and no redirects.
    let mut response = session
        .http
        .get(&file.download_url)
        .header("Accept-Encoding", "identity")
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .build()
        .call()
        .map_err(|_| "download_connection_failed")?;
    if response.status() != 200 {
        return Err(error_code(&mut response, session));
    }
    if response
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        != Some(file.size_bytes)
        || response
            .headers()
            .get("Content-Encoding")
            .is_some_and(|v| v != "identity")
    {
        return Err("artifact_mismatch");
    }
    Ok(response)
}
