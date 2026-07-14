use std::ffi::OsString;
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
        "frontend-cli" => run_frontend_cli(&paths.frontend, rest),
        "frontend-typecheck" => run_frontend_typecheck(&paths.frontend, rest),
        "frontend-unit-test" => run_frontend_unit_test(&paths.frontend, &[], rest),
        "frontend-unit-headless-test" => {
            run_frontend_unit_test(&paths.frontend, &["headless"], rest)
        }
        "frontend-unit-headless-live-test" => {
            run_frontend_unit_test(&paths.frontend, &["kyuubiki-headless-live"], rest)
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

pub(crate) fn run_frontend_cli(frontend: &Path, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_node_script(frontend, "./scripts/kyuubiki-cli.mjs", &[], rest)
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

pub(crate) fn run_frontend_check(frontend: &Path, script_path: &str) -> RunnerResult<u8> {
    run_node_script(frontend, script_path, &[], Vec::new())
}
