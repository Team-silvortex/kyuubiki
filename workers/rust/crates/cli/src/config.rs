#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkerConfig {
    pub(crate) job_id: String,
    pub(crate) project_id: String,
    pub(crate) case_id: String,
    pub(crate) steps: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) agent_id: Option<String>,
    pub(crate) advertise_host: Option<String>,
    pub(crate) orchestrator_url: Option<String>,
    pub(crate) cluster_api_token: Option<String>,
    pub(crate) agent_fingerprint: Option<String>,
    pub(crate) certificate_id: Option<String>,
    pub(crate) cert_path: Option<String>,
    pub(crate) key_path: Option<String>,
    pub(crate) ca_cert_path: Option<String>,
    pub(crate) register_interval_ms: u64,
    pub(crate) cluster_id: Option<String>,
    pub(crate) peers: Vec<String>,
    pub(crate) operator_package_host_id: Option<String>,
    pub(crate) operator_packages_root: Option<String>,
    pub(crate) operator_activated_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Worker(WorkerConfig),
    Agent(AgentConfig),
}

impl Command {
    pub(crate) fn from_env() -> Self {
        let args = std::env::args().skip(1).collect::<Vec<_>>();

        match args.first().map(String::as_str) {
            Some("agent") => Self::Agent(AgentConfig::from_args(&args[1..])),
            _ => Self::Worker(WorkerConfig::from_args(&args)),
        }
    }
}

impl WorkerConfig {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let mut config = Self {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--job-id" => assign_next(&mut config.job_id, &mut args),
                "--project-id" => assign_next(&mut config.project_id, &mut args),
                "--case-id" => assign_next(&mut config.case_id, &mut args),
                "--steps" => {
                    if let Some(value) = args.next() {
                        config.steps = value.parse().unwrap_or(config.steps);
                    }
                }
                _ => {}
            }
        }

        config
    }
}

impl AgentConfig {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let mut config = Self {
            host: "127.0.0.1".to_string(),
            port: 5001,
            agent_id: None,
            advertise_host: None,
            orchestrator_url: None,
            cluster_api_token: None,
            agent_fingerprint: None,
            certificate_id: None,
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            register_interval_ms: 5_000,
            cluster_id: None,
            peers: vec![],
            operator_package_host_id: None,
            operator_packages_root: None,
            operator_activated_package_count: 0,
        };

        config.apply_args(args);
        config.apply_env_defaults();
        config
    }

    fn apply_args(&mut self, args: &[String]) {
        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => assign_next(&mut self.host, &mut args),
                "--port" => {
                    if let Some(value) = args.next() {
                        self.port = value.parse().unwrap_or(self.port);
                    }
                }
                "--agent-id" => assign_next_option(&mut self.agent_id, &mut args),
                "--advertise-host" => assign_next_option(&mut self.advertise_host, &mut args),
                "--orchestrator-url" => assign_next_option(&mut self.orchestrator_url, &mut args),
                "--cluster-api-token" => assign_next_option(&mut self.cluster_api_token, &mut args),
                "--agent-fingerprint" => {
                    assign_next_option(&mut self.agent_fingerprint, &mut args);
                }
                "--certificate-id" => assign_next_option(&mut self.certificate_id, &mut args),
                "--cert-path" => assign_next_option(&mut self.cert_path, &mut args),
                "--key-path" => assign_next_option(&mut self.key_path, &mut args),
                "--ca-cert-path" => assign_next_option(&mut self.ca_cert_path, &mut args),
                "--register-interval-ms" => {
                    if let Some(value) = args.next() {
                        self.register_interval_ms =
                            value.parse().unwrap_or(self.register_interval_ms);
                    }
                }
                "--cluster-id" => assign_next_option(&mut self.cluster_id, &mut args),
                "--peer" => {
                    if let Some(value) = args.next() {
                        self.peers.push(value.clone());
                    }
                }
                "--operator-package-host-id" => {
                    assign_next_option(&mut self.operator_package_host_id, &mut args);
                }
                "--operator-packages-root" => {
                    assign_next_option(&mut self.operator_packages_root, &mut args);
                }
                "--operator-activated-package-count" => {
                    if let Some(value) = args.next() {
                        self.operator_activated_package_count = value
                            .parse()
                            .unwrap_or(self.operator_activated_package_count);
                    }
                }
                _ => {}
            }
        }
    }

    fn apply_env_defaults(&mut self) {
        fill_string_from_env(&mut self.host, "KYUUBIKI_AGENT_HOST");
        fill_u16_from_env(&mut self.port, "KYUUBIKI_AGENT_PORT");
        fill_from_env(&mut self.agent_id, "KYUUBIKI_AGENT_ID");
        fill_from_env(&mut self.advertise_host, "KYUUBIKI_AGENT_ADVERTISE_HOST");
        fill_from_env(&mut self.orchestrator_url, "KYUUBIKI_ORCHESTRATOR_URL");
        fill_from_env(&mut self.cluster_id, "KYUUBIKI_AGENT_CLUSTER_ID");
        fill_from_env(&mut self.cluster_api_token, "KYUUBIKI_CLUSTER_API_TOKEN");
        fill_from_env(&mut self.agent_fingerprint, "KYUUBIKI_AGENT_FINGERPRINT");
        fill_from_env(&mut self.certificate_id, "KYUUBIKI_AGENT_CERTIFICATE_ID");
        fill_from_env(&mut self.cert_path, "KYUUBIKI_AGENT_CERT_PATH");
        fill_from_env(&mut self.key_path, "KYUUBIKI_AGENT_KEY_PATH");
        fill_from_env(&mut self.ca_cert_path, "KYUUBIKI_AGENT_CA_CERT_PATH");
        fill_from_env(
            &mut self.operator_package_host_id,
            "KYUUBIKI_OPERATOR_PACKAGE_HOST_ID",
        );
        fill_from_env(
            &mut self.operator_packages_root,
            "KYUUBIKI_OPERATOR_PACKAGES_ROOT",
        );
        fill_u64_from_env(
            &mut self.register_interval_ms,
            "KYUUBIKI_AGENT_REGISTER_INTERVAL_MS",
        );

        if self.operator_activated_package_count == 0 {
            self.operator_activated_package_count =
                std::env::var("KYUUBIKI_OPERATOR_ACTIVATED_PACKAGE_COUNT")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
        }

        if self.peers.is_empty() {
            self.peers = std::env::var("KYUUBIKI_AGENT_PEERS")
                .ok()
                .map(parse_peer_list)
                .unwrap_or_default();
        }
    }
}

fn assign_next<'a>(target: &mut String, args: &mut impl Iterator<Item = &'a String>) {
    if let Some(value) = args.next() {
        *target = value.clone();
    }
}

fn assign_next_option<'a>(
    target: &mut Option<String>,
    args: &mut impl Iterator<Item = &'a String>,
) {
    if let Some(value) = args.next() {
        *target = Some(value.clone());
    }
}

fn fill_from_env(target: &mut Option<String>, key: &str) {
    if target.is_none() {
        *target = std::env::var(key).ok();
    }
}

fn fill_string_from_env(target: &mut String, key: &str) {
    if let Ok(value) = std::env::var(key) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *target = trimmed.to_string();
        }
    }
}

fn fill_u16_from_env(target: &mut u16, key: &str) {
    if let Ok(value) = std::env::var(key)
        && let Ok(parsed) = value.trim().parse()
    {
        *target = parsed;
    }
}

fn fill_u64_from_env(target: &mut u64, key: &str) {
    if let Ok(value) = std::env::var(key)
        && let Ok(parsed) = value.trim().parse()
    {
        *target = parsed;
    }
}

fn parse_peer_list(value: String) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}
