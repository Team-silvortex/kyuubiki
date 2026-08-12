use std::ffi::OsString;
use std::path::Path;

use serde::Serialize;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_kcore_command(args: Vec<OsString>) -> RunnerResult<u8> {
    let values = args
        .into_iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let Some(command) = values.first().map(String::as_str) else {
        return Err(usage());
    };
    match command {
        "export" => {
            require_shape(
                &values,
                "kcore export <export-spec.json> --out <result.kcore>",
            )?;
            print_report(&kyuubiki_kcore::export_path(&values[1], &values[3])?)?;
        }
        "research-export" => {
            require_shape(
                &values,
                "kcore research-export <research-series.json> --out <result.kcore>",
            )?;
            print_report(&kyuubiki_kcore::export_research_series_path(
                &values[1], &values[3],
            )?)?;
        }
        "inspect" if values.len() == 2 => {
            print_report(&kyuubiki_kcore::inspect_path(Path::new(&values[1]))?)?;
        }
        "verify" if values.len() == 2 => {
            print_report(&kyuubiki_kcore::verify_path(Path::new(&values[1]))?)?;
        }
        "extract" => {
            require_shape(&values, "kcore extract <result.kcore> --out <directory>")?;
            print_report(&kyuubiki_kcore::extract_path(&values[1], &values[3])?)?;
        }
        _ => return Err(usage()),
    }
    Ok(0)
}

fn require_shape(values: &[String], usage: &str) -> RunnerResult<()> {
    if values.len() == 4 && values[2] == "--out" {
        Ok(())
    } else {
        Err(format!("usage: {usage}"))
    }
}

fn print_report(value: &impl Serialize) -> RunnerResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to render kcore report: {error}"))?
    );
    Ok(())
}

fn usage() -> String {
    "usage: kcore export|research-export|inspect|verify|extract (run help for command shapes)"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_export_dispatches_to_native_kcore_builder() {
        let missing = std::env::temp_dir().join("kyuubiki-missing-research-series.json");
        let output = std::env::temp_dir().join("kyuubiki-unused-research-series.kcore");
        let error = run_kcore_command(vec![
            OsString::from("research-export"),
            missing.into_os_string(),
            OsString::from("--out"),
            output.into_os_string(),
        ])
        .expect_err("missing source must reach the native builder");

        assert!(error.contains("failed to inspect research series spec"));
        assert!(!error.contains("usage:"));
    }
}
