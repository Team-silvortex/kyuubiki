use kyuubiki_desktop_runtime::{
    HotServiceMode, ServiceMode, export_database, hot_service_start, hot_service_status,
    hot_service_stop, service_restart, service_start, service_status, service_stop,
};
use std::ffi::OsString;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_runtime_command(command: &str, args: Vec<OsString>) -> Option<RunnerResult<u8>> {
    let result = match command {
        "status" => no_args(command, args).and_then(|_| service_status()),
        "start" => no_args(command, args).and_then(|_| service_start(ServiceMode::Default)),
        "start-local" => no_args(command, args).and_then(|_| service_start(ServiceMode::Local)),
        "start-cloud" => no_args(command, args).and_then(|_| service_start(ServiceMode::Cloud)),
        "start-distributed" => {
            no_args(command, args).and_then(|_| service_start(ServiceMode::Distributed))
        }
        "restart" => no_args(command, args).and_then(|_| service_restart(ServiceMode::Default)),
        "restart-local" => no_args(command, args).and_then(|_| service_restart(ServiceMode::Local)),
        "restart-cloud" => no_args(command, args).and_then(|_| service_restart(ServiceMode::Cloud)),
        "restart-distributed" => {
            no_args(command, args).and_then(|_| service_restart(ServiceMode::Distributed))
        }
        "stop" => no_args(command, args).and_then(|_| service_stop()),
        "export-db" => export_database_arg(args).and_then(|url| export_database(url.as_deref())),
        "hot-status" => no_args(command, args).and_then(|_| hot_service_status()),
        "hot-start-local" => {
            no_args(command, args).and_then(|_| hot_service_start(HotServiceMode::Local))
        }
        "hot-start-cloud" => {
            no_args(command, args).and_then(|_| hot_service_start(HotServiceMode::Cloud))
        }
        "hot-start-distributed" => {
            no_args(command, args).and_then(|_| hot_service_start(HotServiceMode::Distributed))
        }
        "hot-stop" => no_args(command, args).and_then(|_| hot_service_stop()),
        _ => return None,
    };
    Some(result.map(|rendered| {
        if !rendered.is_empty() {
            println!("{rendered}");
        }
        0
    }))
}

fn no_args(command: &str, args: Vec<OsString>) -> RunnerResult<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!("{command} does not accept arguments"))
    }
}

fn export_database_arg(args: Vec<OsString>) -> RunnerResult<Option<String>> {
    if args.len() > 1 {
        return Err("export-db accepts at most one loopback URL".to_string());
    }
    args.into_iter()
        .next()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "export-db URL must be UTF-8".to_string())
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{export_database_arg, no_args, run_runtime_command};
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn runtime_dispatch_rejects_unknown_commands() {
        assert!(run_runtime_command("not-runtime", Vec::new()).is_none());
    }

    #[test]
    fn fixed_runtime_commands_reject_extra_arguments() {
        assert!(no_args("status", vec![OsString::from("extra")]).is_err());
    }

    #[test]
    fn export_database_accepts_one_url() {
        let url = export_database_arg(vec![OsString::from("http://127.0.0.1:4000/export")])
            .expect("valid export URL");
        assert_eq!(url.as_deref(), Some("http://127.0.0.1:4000/export"));
    }

    #[test]
    fn production_entrypoints_do_not_use_legacy_node_runtime() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        for relative in [
            "workers/rust/crates/script-runner/src/main.rs",
            "scripts/kyuubiki.cmd",
        ] {
            let source =
                fs::read_to_string(root.join(relative)).expect("read production entrypoint");
            assert!(
                !source.contains("kyuubiki-runtime.mjs"),
                "{relative} must use kyuubiki-desktop-runtime"
            );
        }
    }
}
