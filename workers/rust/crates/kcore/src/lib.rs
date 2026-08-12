mod archive;
mod canonical;
mod export;
mod model;
mod semantic;

use std::path::Path;

pub use archive::{
    ExtractionReport, InspectionReport, ReaderLimits, VerificationReport, extract, inspect, verify,
    verify_with_limits,
};
pub use export::{ExportReport, export, export_spec};
pub use model::{
    Artifact, ContractBinding, ExportArtifact, ExportSpec, FORMAT_SCHEMA_VERSION, FORMAT_VERSION,
    Integrity, MEDIA_TYPE, Manifest, Producer, SchemaReference,
};
pub use semantic::{
    HEADLESS_RESEARCH_CONTRACT_NAME, RESEARCH_BATCH_ROLE, RESEARCH_PATCH_ROLE, RESEARCH_ROUND_ROLE,
    RESEARCH_RUN_ROLE, SemanticVerification,
};

pub fn export_path(
    spec: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<ExportReport, String> {
    export(spec.as_ref(), output.as_ref())
}

pub fn inspect_path(path: impl AsRef<Path>) -> Result<InspectionReport, String> {
    inspect(path.as_ref())
}

pub fn verify_path(path: impl AsRef<Path>) -> Result<VerificationReport, String> {
    verify(path.as_ref())
}

pub fn extract_path(
    path: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<ExtractionReport, String> {
    extract(path.as_ref(), output.as_ref())
}
