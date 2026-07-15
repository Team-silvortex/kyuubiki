use crate::RunnerResult;
use std::ffi::OsString;

use super::DEFAULT_OUT;

pub(super) struct CheckOptions {
    pub(super) input: String,
    pub(super) self_test: bool,
}

pub(super) fn parse_out(args: Vec<OsString>) -> RunnerResult<String> {
    let mut out = DEFAULT_OUT.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => {
                let Some(value) = iter.next() else {
                    return Err("--out requires a repo-local path".to_string());
                };
                out = value.to_string_lossy().to_string();
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if out.is_empty() {
        return Err("--out requires a repo-local path".to_string());
    }
    Ok(out)
}

pub(super) fn parse_check_args(args: Vec<OsString>) -> RunnerResult<CheckOptions> {
    let mut input = "tmp/operator-qualification-readiness.json".to_string();
    let mut self_test = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--self-test" => self_test = true,
            "--in" => {
                let Some(value) = iter.next() else {
                    return Err("--in requires a repo-local path".to_string());
                };
                input = value.to_string_lossy().to_string();
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !self_test && input.is_empty() {
        return Err("--in requires a repo-local path".to_string());
    }
    Ok(CheckOptions { input, self_test })
}
