use super::*;
use kyuubiki_protocol::AGENT_NETWORK_MAX_CONFIG_BYTES as MAX_CONFIG_BYTES;

fn peer(ip: &str) -> io::Result<SocketAddr> {
    Ok(SocketAddr::new(ip.parse().unwrap(), 50123))
}

#[test]
fn native_network_default_accepts_only_canonical_loopback() {
    let access = NetworkAccess::parse(None).unwrap();
    for ip in ["127.0.0.1", "127.12.0.5", "::1", "::ffff:127.0.0.1"] {
        assert!(access.admit(peer(ip)), "{ip}");
    }
    for ip in ["192.0.2.1", "10.0.0.1", "2001:db8::1", "::ffff:10.0.0.1"] {
        assert!(!access.admit(peer(ip)), "{ip}");
    }
    assert_eq!(access.snapshot()["denied_connections"], 4);
}

#[test]
fn explicit_peer_ips_are_exact_normalized_and_not_subnet_grants() {
    let access = NetworkAccess::parse(Some("192.0.2.1, ::ffff:192.0.2.1, 2001:db8::1")).unwrap();
    assert_eq!(access.policy.allowed_peer_ips().len(), 2);
    for ip in ["192.0.2.1", "::ffff:192.0.2.1", "2001:db8::1", "::1"] {
        assert!(access.admit(peer(ip)));
    }
    for ip in ["192.0.2.2", "2001:db8::2", "10.0.0.1"] {
        assert!(!access.admit(peer(ip)));
    }
}

#[test]
fn malformed_or_overbroad_network_configuration_fails_closed_without_echo() {
    for invalid in [
        "",
        " ",
        "*",
        "0.0.0.0",
        "::",
        "::ffff:0.0.0.0",
        "224.0.0.1",
        "ff02::1",
        "255.255.255.255",
        "::ffff:255.255.255.255",
        "::ffff:224.0.0.1",
        "192.0.2.1,",
        ",192.0.2.1",
        "192.0.2.1,,192.0.2.2",
        "192.0.2.0/24",
        "192.0.2.1:5001",
        "[::1]",
        "fe80::1%en0",
        "localhost",
        "secret.example.invalid",
        "192.0.2.1\n192.0.2.2",
        "192.0.2.999",
        "192.0.002.1",
        "\0",
    ] {
        let error = NetworkAccess::parse(Some(invalid)).err().expect(invalid);
        assert!(error.starts_with(ENV_NAME));
        assert!(!error.contains("secret.example.invalid"));
    }
    let too_long = " ".repeat(MAX_CONFIG_BYTES + 1);
    assert!(NetworkAccess::parse(Some(&too_long)).is_err());
    let max = vec!["192.0.2.1"; MAX_PEERS].join(",");
    assert!(NetworkAccess::parse(Some(&max)).is_ok());
    assert!(NetworkAccess::parse(Some(&format!("{max},192.0.2.1"))).is_err());
}

#[test]
fn unavailable_kernel_peer_identity_is_denied_even_with_explicit_permissions() {
    let access = NetworkAccess::parse(Some("192.0.2.1")).unwrap();
    assert!(!access.admit(Err(io::ErrorKind::NotConnected.into())));
    assert_eq!(access.snapshot()["denied_connections"], 1);
}

#[test]
fn source_ip_policy_never_advertises_transport_or_controller_authentication() {
    for config in [None, Some("192.0.2.1")] {
        let access = NetworkAccess::parse(config).unwrap();
        let snapshot = access.snapshot();
        for field in [
            "authenticated",
            "encrypted",
            "authority_binding_verified",
            "local_process_identity_verified",
            "recovery_reserve_bypass",
        ] {
            assert_eq!(snapshot[field], false, "{field}");
        }
        assert_eq!(snapshot["configuration_apply"], "process_restart");
        assert_eq!(snapshot["peer_source"], "kernel_socket_peer_ip");
        assert_eq!(snapshot["trusted_network_required"], true);
    }
}

#[test]
fn denied_connection_counter_is_bounded_and_safe_under_concurrency() {
    let access = NetworkAccess::default();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let access = &access;
            scope.spawn(move || {
                for _ in 0..100 {
                    assert!(!access.admit(peer("192.0.2.1")));
                }
            });
        }
    });
    assert_eq!(access.snapshot()["denied_connections"], 800);
    access.denied_connections.store(u64::MAX, Ordering::Relaxed);
    assert!(!access.admit(peer("192.0.2.1")));
    assert_eq!(access.snapshot()["denied_connections"], u64::MAX);
}
