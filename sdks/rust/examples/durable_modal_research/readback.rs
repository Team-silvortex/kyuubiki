use kyuubiki_headless_sdk::DurableExecutionHandle;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Read caller-owned local evidence without contacting an Agent or repairing files.
pub fn verify(
    out: &Path,
    input: &Value,
    handle: &DurableExecutionHandle,
    receipt: &Value,
) -> Result<Value> {
    handle.completed_result(receipt)?;
    let verified = out.join("verified");
    let document = super::evidence::read(&verified, "result.json")?;
    let result = document.value;
    let saved = super::evidence::read(&verified, "verification.json")?.value;
    let digest = format!("{:x}", Sha256::digest(&document.bytes));
    if saved["result_sha256"] != digest {
        return Err("export result bytes do not match the verification hash".into());
    }
    handle
        .validate_completed_result(receipt, &result)
        .map_err(|error| format!("verified/result.json / receipt.json consistency: {error}"))?;
    if saved["execution"] != serde_json::to_value(handle)?
        || saved["recovery_basis"] != "prepared_input_and_original_store"
        || !saved["admission_response_available"].is_boolean()
    {
        return Err("export verification identity or recovery metadata mismatch".into());
    }
    let mut fresh = super::checks::verify(input, &result)?;
    for key in [
        "scope",
        "passed",
        "mass_orthogonality_checked",
        "material_certification",
    ] {
        if saved[key] != fresh[key] {
            return Err(format!("export verification claim mismatch: {key}").into());
        }
    }
    // Historical floating-point diagnostics are not authority. Recompute them
    // rather than requiring bit-identical reference arithmetic across toolchains.
    let mut diagnostics_match = true;
    for key in ["independent_roots", "physical_residuals"] {
        let values = saved[key]
            .as_array()
            .ok_or("missing historical diagnostics")?;
        if values.len() != fresh[key].as_array().unwrap().len()
            || values
                .iter()
                .any(|v| v.as_f64().is_none_or(|v| !v.is_finite() || v < 0.0))
        {
            return Err("malformed historical diagnostics".into());
        }
        diagnostics_match &= saved[key] == fresh[key];
    }
    fresh["historical_diagnostics_match"] = json!(diagnostics_match);
    fresh["execution"] = serde_json::to_value(handle)?;
    fresh["result_sha256"] = json!(digest);
    fresh["recovery_basis"] = saved["recovery_basis"].clone();
    fresh["admission_response_available"] = saved["admission_response_available"].clone();
    fresh["verification_source"] = json!("offline_recomputed");
    Ok(fresh)
}
