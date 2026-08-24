use super::*;

#[test]
fn headless_security_redacts_executor_debug_and_rejects_auth_injection() {
    let token = "private-headless-token";
    let executor = ServiceHeadlessExecutor::with_token("http://127.0.0.1:4000", Some(token));
    let rendered = format!("{executor:?}");

    assert!(!rendered.contains(token));
    assert!(rendered.contains("api_token_configured: true"));
    let error = ServiceHeadlessExecutor::try_with_token(
        "http://127.0.0.1:4000",
        Some("token\r\nX-Injected: yes"),
    )
    .expect_err("header injection must fail before network I/O");
    assert!(error.message.contains("unsupported control characters"));
    assert!(!error.message.contains("X-Injected"));
    assert!(
        ServiceHeadlessExecutor::try_with_token(
            "http://127.0.0.1:4000",
            Some("token with whitespace")
        )
        .is_err()
    );
}

#[test]
fn headless_security_rejects_route_escape_and_dynamic_segment_injection() {
    assert!(sanitize_request_path("/api/v1/jobs/job-safe").is_ok());
    assert!(sanitize_request_path("/api/v1/jobs/../credentials").is_err());
    assert!(sanitize_request_path("/api/v1/jobs/job%0d%0aInjected").is_err());

    for job_id in ["../credentials", "job/other", "job\r\nX-Bad: yes"] {
        let payload = json!({"job_id": job_id});
        let error = required_path_segment(&payload, &["job_id"])
            .expect_err("unsafe dynamic route segment must fail");
        assert!(error.message.contains("safe path segment"));
    }
}

#[test]
fn headless_security_enforces_inline_contract_budget_before_dispatch() {
    let error =
        validate_inline_json_size("/api/v1/workflows/graph/jobs", MAX_INLINE_JSON_BYTES + 1)
            .expect_err("oversized inline payload must not reach a socket");

    assert!(error.message.contains("inline JSON transport limit"));
    assert!(error.message.contains("artifact reference"));
}
