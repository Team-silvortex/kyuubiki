use std::ffi::OsString;
use std::path::Path;

use crate::RunnerResult;

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
