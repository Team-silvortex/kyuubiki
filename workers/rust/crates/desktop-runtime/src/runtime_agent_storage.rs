use kyuubiki_protocol::{AGENT_ALLOWED_PEER_IPS_ENV, AgentNetworkAccessPolicy};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn network_policy(
    env: &HashMap<String, String>,
) -> Result<AgentNetworkAccessPolicy, String> {
    AgentNetworkAccessPolicy::parse(env.get(AGENT_ALLOWED_PEER_IPS_ENV).map(String::as_str))
}

pub(crate) fn network_config_line(env: &HashMap<String, String>) -> String {
    match network_policy(env) {
        Ok(policy) => format!(
            "agent-network-config: {} (explicit-peers={}; apply=process_restart; running-policy=not-observed; authentication=false)",
            policy.mode(),
            policy.allowed_peer_ips().len()
        ),
        Err(error) => format!(
            "agent-network-config: invalid ({error}; existing services unchanged; local Agent start/restart blocked)"
        ),
    }
}

pub(crate) fn execution_store_root(
    state: &Path,
    env: &HashMap<String, String>,
) -> Result<PathBuf, String> {
    let root = env
        .get("KYUUBIKI_AGENT_EXECUTION_STORE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| state.join("agent-executions"));
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(
            "Agent execution store root must be an absolute path without parent traversal".into(),
        );
    }
    Ok(root)
}

pub(crate) fn agent_environment(
    state: &Path,
    port: u16,
    env: &HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    network_policy(env)?;
    if env.contains_key("KYUUBIKI_AGENT_EXECUTION_STORE") {
        return Err("Managed Agents require KYUUBIKI_AGENT_EXECUTION_STORE_ROOT, not one shared KYUUBIKI_AGENT_EXECUTION_STORE".into());
    }
    let root = execution_store_root(state, env)?;
    // Reject symlinked ancestors before creating a managed persistence directory.
    for ancestor in root.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            _ => return Err("Unsafe Agent execution store root".into()),
        }
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&root)
        .map_err(|error| format!("Cannot prepare Agent execution store root: {error}"))?;
    let mut configured = env.clone();
    configured.insert(
        "KYUUBIKI_AGENT_EXECUTION_STORE".into(),
        root.join(format!("agent-{port}"))
            .to_string_lossy()
            .into_owned(),
    );
    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_preflight_rejects_before_creating_managed_storage() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("kyuubiki-network-storage-{}", std::process::id()));
        assert!(!root.exists());
        let env = HashMap::from([(AGENT_ALLOWED_PEER_IPS_ENV.into(), "*".into())]);
        assert!(
            agent_environment(&root, 5001, &env)
                .unwrap_err()
                .contains(AGENT_ALLOWED_PEER_IPS_ENV)
        );
        assert!(!root.exists());
        assert!(network_config_line(&env).contains("invalid"));
    }

    #[test]
    fn network_config_summary_is_not_an_applied_policy_or_authentication_claim() {
        let env = HashMap::from([(
            AGENT_ALLOWED_PEER_IPS_ENV.into(),
            "192.0.2.1,::ffff:192.0.2.1".into(),
        )]);
        assert_eq!(network_policy(&env).unwrap().allowed_peer_ips().len(), 1);
        let summary = network_config_line(&env);
        assert!(summary.contains("explicit-peers=1"));
        assert!(summary.contains("running-policy=not-observed"));
        assert!(summary.contains("authentication=false"));
    }

    #[test]
    fn endpoints_have_separate_stable_stores_and_config_is_not_mutated() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("kyuubiki-agent-paths-{}", std::process::id()));
        let env = HashMap::new();
        let a = agent_environment(&root, 5001, &env).unwrap();
        let b = agent_environment(&root, 5002, &env).unwrap();
        assert_ne!(
            a["KYUUBIKI_AGENT_EXECUTION_STORE"],
            b["KYUUBIKI_AGENT_EXECUTION_STORE"]
        );
        assert_eq!(a, agent_environment(&root, 5001, &env).unwrap());
        assert!(env.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_relative_roots_and_shared_agent_store_override() {
        let root = std::env::temp_dir();
        let env = HashMap::from([(
            "KYUUBIKI_AGENT_EXECUTION_STORE_ROOT".into(),
            "relative".into(),
        )]);
        assert!(agent_environment(&root, 5001, &env).is_err());
        let env = HashMap::from([("KYUUBIKI_AGENT_EXECUTION_STORE".into(), "shared".into())]);
        assert!(agent_environment(&root, 5001, &env).is_err());
    }
}
