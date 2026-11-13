use kyuubiki_desktop_runtime::store_account_portal_url;

#[test]
fn account_portal_is_a_fixed_https_page_without_credentials_or_redirects() {
    assert_eq!(
        store_account_portal_url("https://id.silvortex.test:3443/", "zh-cn").unwrap(),
        "https://id.silvortex.test:3443/account?lang=zh"
    );
    assert_eq!(
        store_account_portal_url("https://example.test", "ja").unwrap(),
        "https://example.test/account?lang=en"
    );
    for origin in [
        "",
        "http://example.test",
        "file:///tmp",
        "javascript:alert(1)",
        "https://u:p@example.test",
        "https://example.test/account",
        "https://example.test?token=secret",
        "https://example.test#x",
        "https://example.test\\evil",
        "https://example.test\n",
        "https://example.test:0",
        "https://example.test:99999",
        "https://:443",
        "https://[invalid]:443",
        "https://example.test/%2f",
        "https://example.test/.",
        "https://example.test:",
        "https://example.test/foo/..",
        "https:example.test",
    ] {
        assert!(
            store_account_portal_url(origin, "en").is_err(),
            "{origin:?}"
        );
    }
}
