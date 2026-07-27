use kyuubiki_desktop_runtime::{
    HotServiceMode, ServiceMode, export_database, hot_service_start, hot_service_status,
    hot_service_stop, service_restart, service_start, service_status, service_stop,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());
    let rendered = match command.as_str() {
        "status" => service_status()?,
        "start" => service_start(ServiceMode::Default)?,
        "start-local" => service_start(ServiceMode::Local)?,
        "start-cloud" => service_start(ServiceMode::Cloud)?,
        "start-distributed" => service_start(ServiceMode::Distributed)?,
        "restart" => service_restart(ServiceMode::Default)?,
        "restart-local" => service_restart(ServiceMode::Local)?,
        "restart-cloud" => service_restart(ServiceMode::Cloud)?,
        "restart-distributed" => service_restart(ServiceMode::Distributed)?,
        "stop" => service_stop()?,
        "hot-status" => hot_service_status()?,
        "hot-start-local" => hot_service_start(HotServiceMode::Local)?,
        "hot-start-cloud" => hot_service_start(HotServiceMode::Cloud)?,
        "hot-start-distributed" => hot_service_start(HotServiceMode::Distributed)?,
        "hot-stop" => hot_service_stop()?,
        "export-db" => export_database(args.next().as_deref())?,
        "help" | "--help" | "-h" => help(),
        other => return Err(format!("unknown native runtime command: {other}")),
    };
    println!("{rendered}");
    Ok(())
}

fn help() -> String {
    [
        "kyuubiki native runtime controller",
        "",
        "Commands:",
        "  status",
        "  start | start-local | start-cloud | start-distributed",
        "  restart | restart-local | restart-cloud | restart-distributed",
        "  stop",
        "  export-db [loopback-url]",
        "  hot-status",
        "  hot-start-local | hot-start-cloud | hot-start-distributed",
        "  hot-stop",
    ]
    .join("\n")
}
