use kyuubiki_desktop_runtime::store_identity::{IdentityConfig, StoreIdentity};

fn config() -> IdentityConfig {
    serde_json::from_value(serde_json::json!({
        "issuer":"https://identity.example.test", "client_id":"registered-client",
        "callback_port":47831
    }))
    .unwrap()
}

#[test]
fn configuration_is_public_explicit_https_and_has_a_fixed_loopback_callback() {
    let settings = config();
    settings.validate().unwrap();
    assert_eq!(
        settings.redirect_uri(),
        "http://127.0.0.1:47831/kyuubiki/account/callback"
    );
    for issuer in [
        "http://example.test",
        "https://u:p@example.test",
        "https://example.test/path",
        "https://:443",
    ] {
        let mut invalid = config();
        invalid.issuer = issuer.into();
        assert!(invalid.validate().is_err());
    }
    let mut invalid = config();
    invalid.client_id.clear();
    assert!(invalid.validate().is_err());
    invalid = config();
    invalid.callback_port = 0;
    assert!(invalid.validate().is_err());
    assert!(
        serde_json::from_value::<IdentityConfig>(serde_json::json!({
            "issuer":"https://example.test", "client_id":"id", "client_secret":"not-supported"
        }))
        .is_err()
    );
}

#[test]
fn an_idle_session_exposes_no_credential_and_logout_is_idempotent() {
    let manager = StoreIdentity::default();
    let snapshot = serde_json::to_value(manager.status()).unwrap();
    assert_eq!(snapshot["status"], "signed_out");
    for key in [
        "access_token",
        "refresh_token",
        "id_token",
        "verifier",
        "nonce",
    ] {
        assert!(snapshot.get(key).is_none());
    }
    assert_eq!(manager.logout().status, "signed_out");
    assert_eq!(manager.logout().status, "signed_out");
}
