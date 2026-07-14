use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_central_database_smoke(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    let readiness_status = crate::central_database_readiness::run_check_central_database_readiness(
        root,
        vec![
            OsString::from("--mode"),
            OsString::from(&options.mode),
            OsString::from("--backend"),
            OsString::from(&options.backend),
        ],
    )?;
    if readiness_status != 0 {
        return Ok(readiness_status);
    }
    if !options.run_smoke {
        println!(
            "central database smoke dry-run ok; set RUN_DB_SMOKE=1 or pass --run to execute Postgres-backed tests"
        );
        return Ok(0);
    }

    let mut command = Command::new("mix");
    command
        .current_dir(root.join("apps/web"))
        .env("KYUUBIKI_DEPLOYMENT_MODE", &options.mode)
        .env("KYUUBIKI_STORAGE_BACKEND", &options.backend)
        .args([
            "test",
            "test/kyuubiki_web/api/central_store_api_test.exs",
            "test/kyuubiki_web/api/asset_store_api_test.exs",
        ]);
    let status = command
        .status()
        .map_err(|error| format!("failed to run mix central database smoke: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

struct Options {
    backend: String,
    mode: String,
    run_smoke: bool,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            backend: env::var("BACKEND").unwrap_or_else(|_| "postgres".to_string()),
            mode: env::var("MODE").unwrap_or_else(|_| "cloud".to_string()),
            run_smoke: env::var("RUN_DB_SMOKE").is_ok_and(|value| value == "1"),
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => {
                    print_usage();
                    return Ok(options);
                }
                "--mode" => options.mode = string_arg(&mut iter, "--mode")?,
                "--backend" => options.backend = string_arg(&mut iter, "--backend")?,
                "--run" => options.run_smoke = true,
                other => return Err(format!("unknown or incomplete argument: {other}")),
            }
        }
        Ok(options)
    }
}

fn string_arg(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki central-database-smoke [options]\n\n\
Options:\n  --mode <mode>       Default: MODE or cloud.\n  \
--backend <backend> Default: BACKEND or postgres.\n  \
--run               Execute DB-backed tests; otherwise dry-run only.\n  \
--help              Show this message."
    );
}
