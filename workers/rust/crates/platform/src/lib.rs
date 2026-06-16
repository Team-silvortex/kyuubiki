use std::env;
use std::path::{Path, PathBuf};

pub const LIB_PREFIX_PLACEHOLDER: &str = "{lib_prefix}";
pub const LIB_EXTENSION_PLACEHOLDER: &str = "{lib_extension}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Macos,
    Linux,
    Windows,
}

impl Platform {
    pub fn current() -> Self {
        match env::consts::OS {
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            _ => Self::Linux,
        }
    }

    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("macos") => Self::Macos,
            Some("linux") => Self::Linux,
            Some("windows") => Self::Windows,
            Some(_) | None => Self::current(),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }

    pub fn default_shell(self) -> &'static str {
        match self {
            Self::Windows => "cmd",
            Self::Macos | Self::Linux => "sh",
        }
    }

    pub fn entrypoint_command(self) -> &'static str {
        match self {
            Self::Windows => "./scripts/start.cmd",
            Self::Macos | Self::Linux => "./scripts/start.sh",
        }
    }

    pub fn desktop_bundle_kinds_json(self) -> &'static str {
        match self {
            Self::Macos => "\"app\", \"dmg\"",
            Self::Linux => "\"appimage\", \"deb\", \"rpm\"",
            Self::Windows => "\"msi\", \"nsis\"",
        }
    }
}

pub fn current_platform_library_prefix() -> &'static str {
    if Platform::current() == Platform::Windows {
        ""
    } else {
        "lib"
    }
}

pub fn current_platform_library_extension() -> &'static str {
    match Platform::current() {
        Platform::Macos => "dylib",
        Platform::Windows => "dll",
        Platform::Linux => "so",
    }
}

pub fn current_platform_library_file_name(stem: &str) -> String {
    format!(
        "{}{}.{}",
        current_platform_library_prefix(),
        stem,
        current_platform_library_extension()
    )
}

pub fn current_platform_library_path(parent: impl AsRef<Path>, stem: &str) -> PathBuf {
    parent
        .as_ref()
        .join(current_platform_library_file_name(stem))
}

pub fn expand_platform_library_template(entrypoint: &str) -> String {
    entrypoint
        .replace(LIB_PREFIX_PLACEHOLDER, current_platform_library_prefix())
        .replace(
            LIB_EXTENSION_PLACEHOLDER,
            current_platform_library_extension(),
        )
}

pub fn desktop_preferences_dir(app_name: &str) -> Result<PathBuf, String> {
    match Platform::current() {
        Platform::Macos => {
            let home = env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| "HOME is not available".to_string())?;
            Ok(home
                .join("Library")
                .join("Application Support")
                .join(app_name))
        }
        Platform::Windows => {
            let appdata = env::var_os("APPDATA")
                .map(PathBuf::from)
                .ok_or_else(|| "APPDATA is not available".to_string())?;
            Ok(appdata.join(app_name))
        }
        Platform::Linux => {
            if let Some(config_home) = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
                Ok(config_home.join(app_name))
            } else {
                let home = env::var_os("HOME")
                    .map(PathBuf::from)
                    .ok_or_else(|| "HOME is not available".to_string())?;
                Ok(home.join(".config").join(app_name))
            }
        }
    }
}
