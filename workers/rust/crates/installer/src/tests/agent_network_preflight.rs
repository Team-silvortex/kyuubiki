use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use kyuubiki_protocol::AGENT_ALLOWED_PEER_IPS_ENV;

struct EnvFile(PathBuf);
impl EnvFile {
    fn new(value: Option<&str>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-network-preflight-{}-{}.env",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut content = "KYUUBIKI_STORAGE_BACKEND=sqlite\n".to_string();
        if let Some(value) = value {
            content.push_str(&format!("{AGENT_ALLOWED_PEER_IPS_ENV}={value}\n"));
        }
        fs::write(&path, content).unwrap();
        Self(path)
    }
}
impl Drop for EnvFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn agent_network_preflight_accepts_local_default_and_explicit_literals() {
    for (value, expected) in [
        (None, "loopback_only"),
        (
            Some("192.0.2.1, ::ffff:192.0.2.1, 2001:db8::1"),
            "loopback_and_explicit_peer_ips",
        ),
    ] {
        let file = EnvFile::new(value);
        let report = crate::validate_env_path(&file.0).unwrap();
        assert!(report.contains(&format!("agent-network-config={expected}")));
    }
}

#[test]
fn agent_network_preflight_rejects_the_same_invalid_policy_as_native_agent() {
    for invalid in [
        "",
        "*",
        "0.0.0.0",
        "::",
        "192.0.2.0/24",
        "secret.example.invalid",
        "192.0.2.1,",
        "224.0.0.1",
        "[::1]:5001",
    ] {
        let file = EnvFile::new(Some(invalid));
        let error = crate::validate_env_path(&file.0).unwrap_err();
        let native = kyuubiki_protocol::AgentNetworkAccessPolicy::parse(Some(invalid)).unwrap_err();
        assert_eq!(error, native);
        assert!(!error.contains("secret.example.invalid"));
    }
}
