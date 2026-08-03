use std::path::{Path, PathBuf};

use super::RunnerResult;

const DEFAULT_COMPOSITE_BUNDLE: &str = "tmp/material-research-bundle-composite.json";

pub(super) fn ensure_composite_bundle(root: &Path) -> RunnerResult<PathBuf> {
    let relative = crate::material_research_bundle_build::build_material_research_bundle_file(
        root,
        "composite-thermo-electric-panel",
        DEFAULT_COMPOSITE_BUNDLE,
        2,
    )?;
    Ok(root.join(relative))
}
