use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_ORCHESTRATOR_PORT: u16 = 4000;
pub(crate) const DEFAULT_FRONTEND_PORT: u16 = 3000;

#[derive(Clone, Copy)]
pub(crate) struct RuntimeOptions {
    pub(crate) orchestrator_port: u16,
    pub(crate) frontend_port: u16,
    pub(crate) orchestrator_only: bool,
    pub(crate) frontend_disabled: bool,
}

impl RuntimeOptions {
    pub(crate) fn from_env(env: &HashMap<String, String>) -> Result<Self, String> {
        let orchestrator_port = env
            .get("KYUUBIKI_ORCHESTRATOR_PORT")
            .map(String::as_str)
            .unwrap_or("4000")
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| "KYUUBIKI_ORCHESTRATOR_PORT must be a valid TCP port".to_string())?;
        let orchestrator_only = bool_from_env(env, "KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY")?;
        let frontend_port = env
            .get("KYUUBIKI_FRONTEND_PORT")
            .map(String::as_str)
            .unwrap_or("3000")
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or("KYUUBIKI_FRONTEND_PORT must be a valid TCP port")?;
        let frontend_disabled = bool_from_env(env, "KYUUBIKI_RUNTIME_FRONTEND_DISABLED")?;
        Ok(Self {
            orchestrator_port,
            frontend_port,
            orchestrator_only,
            frontend_disabled,
        })
    }

    pub(crate) fn orchestrator_url(self) -> String {
        format!("http://127.0.0.1:{}", self.orchestrator_port)
    }

    pub(crate) fn orchestrator_pid(self, run: &Path) -> PathBuf {
        run.join(self.scoped_name("orchestrator", "pid"))
    }

    pub(crate) fn orchestrator_log(self, run: &Path) -> PathBuf {
        run.join(self.scoped_name("orchestrator", "log"))
    }

    pub(crate) fn runtime_mode(self, run: &Path) -> PathBuf {
        run.join(self.scoped_name("runtime-mode", "txt"))
    }

    pub(crate) fn frontend_pid(self, run: &Path) -> PathBuf {
        run.join(self.frontend_name("pid"))
    }

    pub(crate) fn frontend_log(self, run: &Path) -> PathBuf {
        run.join(self.frontend_name("log"))
    }

    fn frontend_name(self, extension: &str) -> String {
        if self.frontend_port == DEFAULT_FRONTEND_PORT {
            format!("frontend.{extension}")
        } else {
            format!("frontend-{}.{extension}", self.frontend_port)
        }
    }

    fn scoped_name(self, stem: &str, extension: &str) -> String {
        if self.orchestrator_port == DEFAULT_ORCHESTRATOR_PORT {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{}.{extension}", self.orchestrator_port)
        }
    }
}

fn bool_from_env(env: &HashMap<String, String>, key: &str) -> Result<bool, String> {
    match env
        .get(key)
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        None | Some("") | Some("0") | Some("false") => Ok(false),
        Some("1") | Some("true") => Ok(true),
        Some(_) => Err(format!("{key} must be true, false, 1, or 0")),
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeOptions;

    #[test]
    fn custom_orchestrator_scope_isolated_from_default_runtime() {
        let env = [
            ("KYUUBIKI_ORCHESTRATOR_PORT".into(), "6400".into()),
            ("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY".into(), "true".into()),
            ("KYUUBIKI_RUNTIME_FRONTEND_DISABLED".into(), "1".into()),
        ]
        .into();
        let options = RuntimeOptions::from_env(&env).expect("runtime options");
        assert_eq!(options.orchestrator_port, 6400);
        assert!(options.orchestrator_only);
        assert!(options.frontend_disabled);
        assert!(
            options
                .orchestrator_pid("run".as_ref())
                .ends_with("orchestrator-6400.pid")
        );
        assert!(
            options
                .runtime_mode("run".as_ref())
                .ends_with("runtime-mode-6400.txt")
        );
    }

    #[test]
    fn rejects_invalid_boolean_runtime_option() {
        let env = [(
            "KYUUBIKI_RUNTIME_FRONTEND_DISABLED".into(),
            "sometimes".into(),
        )]
        .into();
        let error = RuntimeOptions::from_env(&env)
            .err()
            .expect("invalid option");
        assert!(error.contains("KYUUBIKI_RUNTIME_FRONTEND_DISABLED"));
    }

    #[test]
    fn frontend_ports_are_validated_and_pid_files_are_scoped() {
        let env = [("KYUUBIKI_FRONTEND_PORT".into(), "6300".into())].into();
        let options = RuntimeOptions::from_env(&env).unwrap();
        assert!(
            options
                .frontend_pid("run".as_ref())
                .ends_with("frontend-6300.pid")
        );
        for port in ["0", "65536", "invalid"] {
            let env = [("KYUUBIKI_FRONTEND_PORT".into(), port.into())].into();
            assert!(RuntimeOptions::from_env(&env).is_err());
        }
    }
}
