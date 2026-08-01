use std::ffi::OsString;
use std::fs;
use std::path::Path;

use crate::RunnerResult;

pub(crate) fn run_node_command(
    paths: &crate::RepoPaths,
    command: &str,
    rest: Vec<OsString>,
) -> Option<RunnerResult<u8>> {
    let result = match command {
        "playground-fem-node-test" => {
            run_node_test(&paths.root, &["apps/web/playground/test/fem.test.mjs"])
        }
        "frontend-typecheck" => run_frontend_typecheck(&paths.frontend, rest),
        "frontend-unit-test" => run_frontend_unit_test(&paths.frontend, &[], rest),
        "frontend-unit-coverage-test" => {
            run_frontend_unit_coverage_test(&paths.root, &paths.frontend, rest)
        }
        "frontend-unit-workflow-test" => {
            run_frontend_unit_test(&paths.frontend, &["workflow"], rest)
        }
        "frontend-ui-layout-check" => {
            run_frontend_check(&paths.frontend, "./scripts/check-ui-layout.mjs")
        }
        "frontend-workflow-search-layout-check" => run_frontend_check(
            &paths.frontend,
            "./scripts/check-workflow-search-layout.mjs",
        ),
        "frontend-workflow-topology-check" => run_frontend_check(
            &paths.frontend,
            "./scripts/check-workflow-topology-regression.mjs",
        ),
        "frontend-workflow-benchmark" => {
            run_frontend_check(&paths.frontend, "./scripts/workflow-benchmark.mjs")
        }
        "hub-gui-compile-ui" => run_hub_gui_compile(&paths.hub_gui),
        "hub-gui-smoke-node-test" => run_hub_gui_smoke(&paths.hub_gui),
        "installer-gui-smoke-node-test" => run_app_smoke(&paths.installer_gui),
        "workbench-gui-smoke-node-test" => run_app_smoke(&paths.workbench_gui),
        "integration-api-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/orchestrator-agent-api-smoke.test.mjs"],
        ),
        "integration-cluster-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/distributed-control-plane-smoke.test.mjs"],
        ),
        "integration-direct-mesh-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/direct-mesh-gui-smoke.test.mjs"],
        ),
        "integration-desktop-gui-node-test" => run_node_test(
            &paths.root,
            &[
                "tests/integration/desktop-gui-action-sweep.test.mjs",
                "tests/integration/desktop-gui-capability-closure.test.mjs",
                "tests/integration/desktop-gui-call-chain-contract.test.mjs",
                "tests/integration/desktop-gui-layout-priority.test.mjs",
                "tests/integration/desktop-gui-navigation-closure.test.mjs",
                "tests/integration/desktop-shell-regression.test.mjs",
                "tests/integration/workbench-shell-regression.test.mjs",
            ],
        ),
        "integration-benchmark-profile-index-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/benchmark-profile-index.test.mjs"],
        ),
        "integration-ui-mechanical-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/workbench-ui-mechanical-smoke.test.mjs"],
        ),
        "integration-ui-thermal-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/workbench-ui-thermal-smoke.test.mjs"],
        ),
        "integration-ui-workflow-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/workbench-ui-workflow-invocation.test.mjs"],
        ),
        _ => return None,
    };
    Some(result)
}

pub(crate) fn run_node_script(
    cwd: &Path,
    script_path: &str,
    fixed_args: &[&str],
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    crate::run_command(
        cwd,
        "node",
        std::iter::once(OsString::from(script_path))
            .chain(fixed_args.iter().map(OsString::from))
            .chain(rest),
    )
}

pub(crate) fn run_node_test(cwd: &Path, test_paths: &[&str]) -> RunnerResult<u8> {
    crate::run_command(
        cwd,
        "node",
        std::iter::once(OsString::from("--test")).chain(test_paths.iter().map(OsString::from)),
    )
}

pub(crate) fn run_hub_gui_compile(hub_gui: &Path) -> RunnerResult<u8> {
    crate::run_command(
        hub_gui,
        "node",
        [OsString::from("./scripts/compile-ui.mjs")],
    )
}

pub(crate) fn run_hub_gui_smoke(hub_gui: &Path) -> RunnerResult<u8> {
    let compile = run_hub_gui_compile(hub_gui)?;
    if compile != 0 {
        return Ok(compile);
    }
    run_node_test(hub_gui, &["./test/smoke.test.mjs"])
}

pub(crate) fn run_app_smoke(app_dir: &Path) -> RunnerResult<u8> {
    run_node_test(app_dir, &["./test/smoke.test.mjs"])
}

pub(crate) fn run_frontend_typecheck(frontend: &Path, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_node_script(frontend, "./scripts/typecheck.mjs", &[], rest)
}

pub(crate) fn run_frontend_unit_test(
    frontend: &Path,
    fixed_args: &[&str],
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    run_node_script(frontend, "./scripts/test-unit.mjs", fixed_args, rest)
}

pub(crate) fn run_frontend_unit_coverage_test(
    root: &Path,
    frontend: &Path,
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    let (out_dir, filters) = parse_frontend_coverage_args(rest)?;
    validate_coverage_out_dir(&out_dir)?;
    let absolute_out = root.join(&out_dir);
    if absolute_out.exists() {
        fs::remove_dir_all(&absolute_out)
            .map_err(|error| format!("failed to clear {}: {error}", absolute_out.display()))?;
    }
    fs::create_dir_all(&absolute_out)
        .map_err(|error| format!("failed to create {}: {error}", absolute_out.display()))?;
    let out_string = absolute_out.to_string_lossy().into_owned();
    crate::run_with_env(
        frontend,
        "node",
        std::iter::once(OsString::from("./scripts/test-unit.mjs")).chain(filters),
        &[("KYUUBIKI_FRONTEND_COVERAGE_DIR", out_string.as_str())],
    )
}

pub(crate) fn run_frontend_check(frontend: &Path, script_path: &str) -> RunnerResult<u8> {
    run_node_script(frontend, script_path, &[], Vec::new())
}

fn parse_frontend_coverage_args(rest: Vec<OsString>) -> RunnerResult<(String, Vec<OsString>)> {
    let mut out_dir = "tmp/coverage/frontend/v8".to_string();
    let mut filters = Vec::new();
    let mut iter = rest.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "--out-dir" {
            out_dir = iter
                .next()
                .ok_or_else(|| "--out-dir requires a value".to_string())?
                .into_string()
                .map_err(|_| "--out-dir value is not valid utf-8".to_string())?;
        } else {
            filters.push(arg);
        }
    }
    Ok((out_dir, filters))
}

fn validate_coverage_out_dir(out_dir: &str) -> RunnerResult<()> {
    if Path::new(out_dir).is_absolute()
        || out_dir.split('/').any(|part| part == "..")
        || !out_dir.starts_with("tmp/coverage/")
    {
        return Err("frontend coverage out dir must be under tmp/coverage/".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_coverage_args_reject_unsafe_out_dir() {
        assert!(parse_frontend_coverage_args(vec![OsString::from("workflow")]).is_ok());
        let parsed = parse_frontend_coverage_args(vec![
            OsString::from("--out-dir"),
            OsString::from("tmp/coverage/frontend/v8"),
            OsString::from("workflow"),
        ])
        .unwrap();
        assert_eq!(parsed.0, "tmp/coverage/frontend/v8");
        assert_eq!(parsed.1, vec![OsString::from("workflow")]);
        assert!(validate_coverage_out_dir("/tmp/coverage").is_err());
        assert!(validate_coverage_out_dir("tmp/../coverage").is_err());
        assert!(validate_coverage_out_dir("dist/coverage").is_err());
    }
}
