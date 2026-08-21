use super::*;
use serde_json::json;

#[test]
fn registry_requires_the_recovered_registration_count() {
    let value = json!({
        "agents": [{
            "id": AGENT_ID,
            "control_plane_link": {
                "state": "registered",
                "successful_registration_count": 2
            }
        }]
    });
    assert!(registry_contains_agent(&value, 2));
    assert!(!registry_contains_agent(&value, 3));
}

#[test]
fn chunked_json_is_decoded_without_retaining_headers() {
    let decoded = decode_chunked(b"7\r\n{\"a\":1}\r\n0\r\n\r\n").expect("chunked body");
    assert_eq!(decoded, b"{\"a\":1}");
}

#[test]
fn remote_cleanup_is_scoped_to_managed_run_root() {
    let command = remote_reset_agent_command("~/.kyuubiki/lab-runs/test");
    assert!(command.contains("readlink -f"));
    assert!(command.contains("$run_root/kyuubiki-agent"));
}
