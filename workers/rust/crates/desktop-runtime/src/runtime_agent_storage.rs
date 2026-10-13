use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
