use std::env;
use std::path::Path;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("kcore error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    let report = match command {
        "export" if args.len() == 4 && args[2] == "--out" => serde_json::to_value(
            kyuubiki_kcore::export_path(Path::new(&args[1]), Path::new(&args[3]))?,
        ),
        "research-export" if args.len() == 4 && args[2] == "--out" => serde_json::to_value(
            kyuubiki_kcore::export_research_series_path(Path::new(&args[1]), Path::new(&args[3]))?,
        ),
        "inspect" if args.len() == 2 => {
            serde_json::to_value(kyuubiki_kcore::inspect_path(Path::new(&args[1]))?)
        }
        "verify" if args.len() == 2 => {
            serde_json::to_value(kyuubiki_kcore::verify_path(Path::new(&args[1]))?)
        }
        "extract" if args.len() == 4 && args[2] == "--out" => serde_json::to_value(
            kyuubiki_kcore::extract_path(Path::new(&args[1]), Path::new(&args[3]))?,
        ),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            return Ok(());
        }
        _ => return Err(usage()),
    }
    .map_err(|error| format!("failed to render command report: {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("failed to render command report: {error}"))?
    );
    Ok(())
}

fn usage() -> String {
    "usage:\n  kyuubiki-kcore export <export-spec.json> --out <result.kcore>\n  \
     kyuubiki-kcore research-export <research-series.json> --out <result.kcore>\n  \
     kyuubiki-kcore inspect <result.kcore>\n  \
     kyuubiki-kcore verify <result.kcore>\n  \
     kyuubiki-kcore extract <result.kcore> --out <directory>"
        .to_string()
}
