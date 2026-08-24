use kyuubiki_headless_sdk::{ControlPlaneClient, KyuubikiAuth};

#[test]
fn auth_debug_redacts_and_rejects_header_injection() {
    let token = "private-rust-sdk-token";
    let auth = KyuubikiAuth::access_token(token);
    let rendered = format!("{auth:?}");

    assert!(!rendered.contains(token));
    assert!(rendered.contains("[REDACTED]"));
    auth.validate().expect("ordinary token is valid");

    let injected = KyuubikiAuth::access_token("token\r\nX-Injected: yes");
    let error = injected.validate().expect_err("injected token must fail");
    assert!(
        error
            .to_string()
            .contains("invalid authentication header value")
    );
    assert!(!error.to_string().contains("X-Injected"));

    let client = ControlPlaneClient::new_with_auth("http://127.0.0.1:9", Some(injected))
        .expect("client construction remains side-effect free");
    let request_error = client
        .health()
        .expect_err("invalid auth must fail before a connection attempt");
    assert!(
        request_error
            .to_string()
            .contains("invalid authentication header value")
    );
}

#[test]
fn auth_rejects_unsafe_custom_header_names_and_empty_values() {
    let unsafe_name = KyuubikiAuth {
        header_name: "X-Test\r\nInjected".into(),
        header_value: "token".into(),
    };
    assert!(unsafe_name.validate().is_err());
    assert!(KyuubikiAuth::access_token("").validate().is_err());
}
