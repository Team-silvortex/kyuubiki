use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = kyuubiki_deploy_server::run_cli(args) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
