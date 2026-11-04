use super::*;
use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

struct NativeAgent {
    child: Option<Child>,
    binary: PathBuf,
    port: u16,
    lan_ip: Option<Ipv4Addr>,
}

impl NativeAgent {
    fn start(lan_ip: Option<Ipv4Addr>) -> Self {
        let binary = PathBuf::from(
            std::env::var_os("KYUUBIKI_TEST_AGENT_BINARY")
                .expect("set KYUUBIKI_TEST_AGENT_BINARY to the matching built native Agent"),
        )
        .canonicalize()
        .unwrap();
        let reserved = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = reserved.local_addr().unwrap().port();
        drop(reserved);
        let mut agent = Self {
            child: None,
            binary,
            port,
            lan_ip,
        };
        agent.restart(false);
        agent
    }

    fn local_endpoint(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn endpoint(&self) -> String {
        format!(
            "{}:{}",
            self.lan_ip.unwrap_or(Ipv4Addr::LOCALHOST),
            self.port
        )
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().unwrap().is_none() {
                child.kill().unwrap();
            }
            child.wait().unwrap();
        }
    }

    fn restart(&mut self, trusted: bool) {
        self.stop();
        let mut command = Command::new(&self.binary);
        // Isolated source processes: never inherit real host authority, credentials or stores.
        command
            .env_clear()
            .args([
                "agent",
                "--host",
                if self.lan_ip.is_some() {
                    "0.0.0.0"
                } else {
                    "127.0.0.1"
                },
                "--port",
                &self.port.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(ip) = self.lan_ip {
            command.env("KYUUBIKI_AGENT_ALLOWED_PEER_IPS", ip.to_string());
        }
        command.env(
            "KYUUBIKI_AGENT_TRUSTED_LAN_CONTROL",
            if trusted { "true" } else { "false" },
        );
        self.child = Some(command.spawn().unwrap());
        let pid = self.child.as_ref().unwrap().id();
        let prefix = format!("agent-instance-{pid}-");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "Agent exited before readiness"
            );
            if let Ok((descriptor, _)) = request_with_timeout(
                &self.local_endpoint(),
                "describe_agent",
                json!({}),
                Duration::from_millis(250),
            ) {
                if descriptor["lifecycle"]["process_instance_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with(&prefix))
                {
                    break;
                }
            }
            assert!(
                Instant::now() < deadline,
                "Agent readiness did not match owned child"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn descriptor(&self) -> Value {
        request(&self.local_endpoint(), "describe_agent", json!({}))
            .unwrap()
            .0
    }

    fn started_count(&self) -> u64 {
        self.descriptor()["watchdog"]["total_started_execution_count"]
            .as_u64()
            .unwrap()
    }
}

impl Drop for NativeAgent {
    fn drop(&mut self) {
        self.stop();
    }
}

fn input() -> Value {
    json!({"length":1.0, "area":2.0, "youngs_modulus":1000.0,
        "elements":2, "tip_force":20.0})
}

#[test]
#[ignore = "requires matching KYUUBIKI_TEST_AGENT_BINARY and this host's KYUUBIKI_TEST_AGENT_LAN_IP"]
fn native_selection_skips_discovery_only_lan_and_observes_restart_policy_changes() {
    let ip: Ipv4Addr = std::env::var("KYUUBIKI_TEST_AGENT_LAN_IP")
        .expect("select an explicit local IPv4 interface")
        .parse()
        .unwrap();
    assert!(!ip.is_loopback() && !ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast());
    // Refuse non-local targets; no server or production deployment is probed.
    drop(TcpListener::bind((ip, 0)).unwrap());
    let mut lan = NativeAgent::start(Some(ip));
    let local = NativeAgent::start(None);
    let endpoints = [lan.endpoint(), local.endpoint()];
    for mode in ["first_reachable", "healthiest"] {
        let result = solve("solve_bar_1d", input(), &endpoints, mode).unwrap();
        assert_eq!(result.endpoint, local.endpoint());
        assert_eq!(result.result["max_stress"], 10.0);
    }
    assert_eq!(lan.started_count(), 0);
    assert_eq!(local.started_count(), 2);
    assert_eq!(
        lan.descriptor()["request_ingress"]["network_access"]["lan_control"]["denied_requests"],
        0
    );
    let error = solve("solve_bar_1d", input(), &[lan.endpoint()], "healthiest").unwrap_err();
    assert!(
        error.contains("connection_control_not_permitted") && error.contains("no task dispatched")
    );

    lan.restart(true);
    let result = solve("solve_bar_1d", input(), &endpoints, "first_reachable").unwrap();
    assert_eq!(result.endpoint, lan.endpoint());
    assert_eq!(result.result["max_stress"], 10.0);
    assert_eq!(lan.started_count(), 1);
    assert_eq!(local.started_count(), 2);

    lan.restart(false);
    let result = solve("solve_bar_1d", input(), &endpoints, "first_reachable").unwrap();
    assert_eq!(result.endpoint, local.endpoint());
    assert_eq!(lan.started_count(), 0);
    assert_eq!(local.started_count(), 3);
    assert_eq!(
        lan.descriptor()["request_ingress"]["network_access"]["lan_control"]["denied_requests"],
        0
    );
}
