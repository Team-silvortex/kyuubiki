//! Opt-in, bounded fixture service. No production lifecycle or Store credentials.
#![cfg(unix)]
#[path = "support/operator_cross_host_transport.rs"]
mod transport;
use kyuubiki_installer::{
    Platform, active_agent_binary_in, install_agent_update_package_into,
    prepare_agent_update_package,
};
use serde_json::json;
use std::{
    fs,
    io::Write,
    net::{IpAddr, SocketAddr, TcpListener},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use transport::{SERVER_NAME, Transport};

struct Host {
    root: PathBuf,
    child: Option<Child>,
}
impl Host {
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            child.wait().unwrap();
        }
    }
    fn start(&mut self, endpoint: SocketAddr, peer: IpAddr, token: &str, pem: &str, phase: &str) {
        let binary =
            active_agent_binary_in(&self.root.join("agent-store"), Platform::current()).unwrap();
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(self.root.join("agent.log"))
            .unwrap();
        self.child = Some(
            Command::new(binary)
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .env("HOME", &self.root)
                .env("TMPDIR", &self.root)
                .env("KYUUBIKI_AGENT_API_TOKEN", token)
                .env("KYUUBIKI_AGENT_TLS_CERT_FILE", self.root.join("ca.pem"))
                .env("KYUUBIKI_AGENT_TLS_KEY_FILE", self.root.join("key.pem"))
                .env(
                    "KYUUBIKI_AGENT_ALLOWED_PEER_IPS",
                    format!("{peer},{}", endpoint.ip()),
                )
                .env("KYUUBIKI_AGENT_TRUSTED_LAN_CONTROL", "true")
                .env("KYUUBIKI_AGENT_PACKAGE_DEPLOYMENT", "true")
                .args([
                    "agent",
                    "--host",
                    &endpoint.ip().to_string(),
                    "--port",
                    &endpoint.port().to_string(),
                    "--agent-id",
                    "owned-tls-deployment-test",
                    "--operator-package-host-id",
                    "owned-tls-host",
                    "--operator-packages-root",
                    self.root.join("operator-store/packages").to_str().unwrap(),
                    "--operator-activated-package-count",
                    "0",
                ])
                .stdin(Stdio::null())
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        let client = Transport::configured(
            &endpoint.to_string(),
            "native-tls",
            Some(pem),
            Some(SERVER_NAME),
            Some(token.into()),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let descriptor = loop {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "test Agent exited before ready; inspect the private fixture log"
            );
            if let Ok(descriptor) = client.request("describe_agent", json!({})) {
                break descriptor;
            }
            assert!(Instant::now() < deadline, "test Agent readiness timeout");
            thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(descriptor["connection_access"]["encrypted"], true);
        private_file(&self.root.join("ready.pending"), &serde_json::to_vec(&json!({
            "phase":phase,"endpoint":endpoint.to_string(),"agent_pid":self.child.as_ref().unwrap().id()
        })).unwrap());
        fs::rename(
            self.root.join("ready.pending"),
            self.root.join("ready.json"),
        )
        .unwrap();
        println!("owned native TLS fixture ready: phase={phase}, endpoint={endpoint}");
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn private_file(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn random_token() -> String {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).unwrap();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
#[ignore = "bounded owned TLS host; requires explicit fresh root, bind IP and permitted controller IP"]
fn serve_owned_installer_managed_tls_deployment_fixture() {
    let root =
        PathBuf::from(std::env::var("KYUUBIKI_TLS_HOST_ROOT").expect("fresh owned test root"));
    assert!(root.is_absolute());
    let ip: IpAddr = std::env::var("KYUUBIKI_TLS_HOST_BIND")
        .unwrap()
        .parse()
        .unwrap();
    let peer: IpAddr = std::env::var("KYUUBIKI_TLS_HOST_PEER")
        .unwrap()
        .parse()
        .unwrap();
    assert!(!ip.is_unspecified() && !ip.is_multicast());
    assert!(!peer.is_unspecified() && !peer.is_multicast());
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .expect("never reuse an existing root");
    let mut host = Host { root, child: None };
    let certificate = rcgen::generate_simple_self_signed(vec![SERVER_NAME.into()]).unwrap();
    let pem = certificate.cert.pem();
    let tokens = [random_token(), random_token()];
    for (name, bytes) in [
        ("ca.pem", pem.as_bytes()),
        (
            "key.pem",
            certificate.signing_key.serialize_pem().as_bytes(),
        ),
        ("token.before", tokens[0].as_bytes()),
        ("token.after", tokens[1].as_bytes()),
    ] {
        private_file(&host.root.join(name), bytes);
    }
    fs::create_dir_all(host.root.join("operator-store/packages")).unwrap();
    prepare_agent_update_package(
        Path::new(env!("CARGO_BIN_EXE_kyuubiki-cli")),
        &host.root.join("agent-package"),
        env!("CARGO_PKG_VERSION"),
        Platform::current(),
    )
    .unwrap();
    install_agent_update_package_into(
        &host.root.join("agent-package"),
        &host.root.join("agent-store"),
        Platform::current(),
    )
    .unwrap();
    let reserved = TcpListener::bind((ip, 0)).unwrap();
    let endpoint = reserved.local_addr().unwrap();
    drop(reserved);
    host.start(endpoint, peer, &tokens[0], &pem, "initial");
    let deadline = Instant::now() + Duration::from_secs(600);
    let mut rotated = false;
    while !host.root.join("stop").exists() {
        assert!(Instant::now() < deadline, "bounded host lease expired");
        assert!(
            host.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "owned test Agent unexpectedly exited"
        );
        if !rotated && host.root.join("rotate").exists() {
            host.stop();
            // Verify ownership release before the exact installed binary is restarted.
            drop(TcpListener::bind(endpoint).expect("old Agent must release its listener"));
            host.start(endpoint, peer, &tokens[1], &pem, "rotated");
            rotated = true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    host.stop();
    let logs = fs::read_to_string(host.root.join("agent.log")).unwrap();
    assert!(tokens.iter().all(|token| !logs.contains(token)));
    assert!(!logs.contains("PRIVATE KEY"));
    println!(
        "owned native TLS fixture stopped; credentials and installed test store removed on exit"
    );
}
