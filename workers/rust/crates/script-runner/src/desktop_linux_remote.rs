use crate::remote_host::{remote_shell_path, rsync_to, shell_escape, ssh_status};
use std::env;
use std::ffi::OsString;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const LINUX_DESKTOP_APT_PACKAGES: &[&str] = &[
    "libwebkit2gtk-4.1-dev",
    "libgtk-3-dev",
    "librsvg2-dev",
    "patchelf",
];

struct Options {
    remote_build_command: String,
    remote_dir: String,
    remote_host: String,
    sync_to_remote: bool,
}

pub(crate) fn run_desktop_linux_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    let action = RemoteLinuxAction::parse(args.first())?;
    if args.len() > usize::from(action != RemoteLinuxAction::Build) {
        return Err("desktop-linux-remote accepts at most one action".into());
    }

    let options = Options::from_env();
    if options.sync_to_remote && action == RemoteLinuxAction::Build {
        sync_repo(root, &options)?;
    }
    let command = match action {
        RemoteLinuxAction::Build => remote_build_command(&options),
        RemoteLinuxAction::InstallDeps => remote_install_deps_command(&options),
        RemoteLinuxAction::Preflight => remote_preflight_command(&options),
    };

    let status = ssh_status(root, &options.remote_host, command)?;
    if status != 0 {
        return Ok(status);
    }

    println!(
        "remote Linux desktop command completed on {}",
        options.remote_host
    );
    println!("remote output: {}/dist/linux", options.remote_dir);
    Ok(0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RemoteLinuxAction {
    Build,
    InstallDeps,
    Preflight,
}

impl RemoteLinuxAction {
    fn parse(value: Option<&OsString>) -> RunnerResult<Self> {
        match value.and_then(|value| value.to_str()) {
            None | Some("") => Ok(Self::Build),
            Some("install-deps") => Ok(Self::InstallDeps),
            Some("preflight") => Ok(Self::Preflight),
            Some(other) => Err(format!(
                "unsupported desktop-linux-remote action `{other}`; expected preflight or install-deps"
            )),
        }
    }
}

impl Options {
    fn from_env() -> Self {
        Self {
            remote_build_command: env::var("REMOTE_BUILD_COMMAND").unwrap_or_else(|_| {
                [
                    "npm ci --prefix apps/hub-gui",
                    "npm ci --prefix apps/installer-gui",
                    "npm ci --prefix apps/workbench-gui",
                    "make package-desktop PLATFORM=linux",
                    "make desktop-verify PLATFORM=linux",
                ]
                .join(" && ")
            }),
            remote_dir: env::var("KYUUBIKI_LAB_DESKTOP_DIR")
                .unwrap_or_else(|_| "~/kyuubiki-desktop".to_string()),
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            sync_to_remote: env::var("SYNC_TO_REMOTE").unwrap_or_else(|_| "1".into()) == "1",
        }
    }
}

fn sync_repo(root: &Path, options: &Options) -> RunnerResult<()> {
    let mkdir_status = ssh_status(
        root,
        &options.remote_host,
        format!("mkdir -p {}", shell_escape(&options.remote_dir)),
    )?;
    if mkdir_status != 0 {
        return Err(format!("remote mkdir failed with status {mkdir_status}"));
    }

    let source = root.join(".");
    let destination = format!("{}:{}/", options.remote_host, options.remote_dir);
    let status = rsync_to(
        root,
        &[".git/", "node_modules/", "target/", "dist/", "tmp/"],
        &[source],
        &destination,
    )?;
    if status != 0 {
        return Err(format!("rsync failed with status {status}"));
    }
    Ok(())
}

fn remote_build_command(options: &Options) -> String {
    let env_prefix = remote_env_prefix();
    format!(
        "cd {} && {}{}",
        remote_shell_path(&options.remote_dir),
        env_prefix,
        options.remote_build_command
    )
}

fn remote_preflight_command(options: &Options) -> String {
    let required_packages = LINUX_DESKTOP_APT_PACKAGES.join(" ");
    let env_prefix = remote_env_prefix();
    format!(
        "cd {} && \
{}\
echo node=$(node --version) && \
node -e \"const [maj,min,patch]=process.versions.node.split('.').map(Number); process.exit(maj===20&&min===19&&patch>=2?0:1)\" && \
echo npm=$(npm --version) && \
echo cargo=$(cargo --version) && \
echo rustc=$(rustc --version) && \
command -v make >/dev/null && \
dpkg -s {} >/dev/null && \
echo linux-desktop-preflight=ok",
        remote_shell_path(&options.remote_dir),
        env_prefix,
        required_packages
    )
}

fn remote_install_deps_command(options: &Options) -> String {
    let packages = LINUX_DESKTOP_APT_PACKAGES.join(" ");
    format!(
        "cd {} && sudo -n apt-get update && sudo -n apt-get install -y {}",
        remote_shell_path(&options.remote_dir),
        packages
    )
}

fn remote_env_prefix() -> String {
    let mut exports = Vec::new();
    exports.push(
        "export PATH=$HOME/.local/kyuubiki-runtimes/node-v20.19.2-linux-x64/bin:$PATH".to_string(),
    );
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "no_proxy",
    ] {
        if let Ok(value) = env::var(key) {
            if !value.trim().is_empty() {
                exports.push(format!("export {key}={}", shell_escape(&value)));
            }
        }
    }
    if exports.is_empty() {
        String::new()
    } else {
        format!("{}; ", exports.join("; "))
    }
}

fn print_usage() {
    println!(
        "desktop-linux-remote\n\n\
Usage:\n  \
./scripts/kyuubiki desktop-linux-remote [preflight|install-deps|--help]\n\n\
Build Linux desktop packages on the Ubuntu lab host.\n\n\
Environment:\n  \
KYUUBIKI_LAB_HOST              SSH host alias, default kyuubiki-lab\n  \
KYUUBIKI_LAB_DESKTOP_DIR       remote checkout, default ~/kyuubiki-desktop\n  \
REMOTE_BUILD_COMMAND           remote command, defaults to npm ci for desktop apps plus Linux package/verify\n  \
SYNC_TO_REMOTE                 1 to rsync before building, 0 to reuse remote checkout\n  \
HTTP_PROXY / HTTPS_PROXY / NO_PROXY are forwarded when present\n"
    );
}
