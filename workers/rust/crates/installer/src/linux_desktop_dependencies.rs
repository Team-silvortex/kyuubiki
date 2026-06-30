const LINUX_DESKTOP_DEPS_SCHEMA_VERSION: &str = "kyuubiki.linux-desktop-dependencies/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxDesktopDependencyPlan {
    pub schema_version: String,
    pub target: String,
    pub node_runtime: String,
    pub apt_packages: Vec<String>,
    pub preflight_command: String,
    pub install_command: String,
    pub notes: Vec<String>,
}

impl LinuxDesktopDependencyPlan {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki Linux desktop dependency plan".to_string(),
            format!("schema: {}", self.schema_version),
            format!("target: {}", self.target),
            format!("node_runtime: {}", self.node_runtime),
            "apt_packages:".to_string(),
        ];
        for package in &self.apt_packages {
            lines.push(format!("  - {package}"));
        }
        lines.push(format!("preflight: {}", self.preflight_command));
        lines.push(format!("install: {}", self.install_command));
        lines.push("notes:".to_string());
        for note in &self.notes {
            lines.push(format!("  - {note}"));
        }
        lines.join("\n")
    }
}

pub fn linux_desktop_dependency_plan() -> LinuxDesktopDependencyPlan {
    let apt_packages = vec![
        "libwebkit2gtk-4.1-dev",
        "libgtk-3-dev",
        "librsvg2-dev",
        "patchelf",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();

    LinuxDesktopDependencyPlan {
        schema_version: LINUX_DESKTOP_DEPS_SCHEMA_VERSION.to_string(),
        target: "Ubuntu lab host for Tauri Linux desktop bundles".to_string(),
        node_runtime: "~/.local/kyuubiki-runtimes/node-v20.19.2-linux-x64".to_string(),
        preflight_command: "make desktop-linux-remote-preflight".to_string(),
        install_command: format!(
            "sudo apt-get update && sudo apt-get install -y {}",
            apt_packages.join(" ")
        ),
        apt_packages,
        notes: vec![
            "Node 20.19.2 is user-scoped under the Kyuubiki runtime directory; do not replace the system Node just for this project.".to_string(),
            "Apt package installation is privileged host state and should be performed by installer-managed remote execution or an operator-controlled sudo session.".to_string(),
            "Run make desktop-linux-remote only after preflight passes.".to_string(),
        ],
    }
}
