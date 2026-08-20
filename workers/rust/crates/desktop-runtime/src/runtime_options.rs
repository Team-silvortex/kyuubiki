use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_ORCHESTRATOR_PORT: u16 = 4000;

#[derive(Clone, Copy)]
pub(crate) struct RuntimeOptions {
    pub(crate) orchestrator_port: u16,
    pub(crate) orchestrator_only: bool,
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
        let orchestrator_only = match env
            .get("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY")
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            None | Some("") | Some("0") | Some("false") => false,
            Some("1") | Some("true") => true,
            Some(_) => {
                return Err(
                    "KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY must be true, false, 1, or 0".into(),
                );
            }
        };
        Ok(Self {
            orchestrator_port,
            orchestrator_only,
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

    fn scoped_name(self, stem: &str, extension: &str) -> String {
        if self.orchestrator_port == DEFAULT_ORCHESTRATOR_PORT {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{}.{extension}", self.orchestrator_port)
        }
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
        ]
        .into();
        let options = RuntimeOptions::from_env(&env).expect("runtime options");
        assert_eq!(options.orchestrator_port, 6400);
        assert!(options.orchestrator_only);
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
}
