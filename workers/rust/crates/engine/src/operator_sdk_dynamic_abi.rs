use kyuubiki_operator_sdk::{
    MAX_OPERATOR_JSON_ABI_BYTES, OPERATOR_JSON_ABI_FREE_SYMBOL, OperatorDescriptorBuilder,
    OperatorHandler, OperatorJsonAbiBuffer, OperatorJsonEntrypoint, OperatorJsonFreeEntrypoint,
    OperatorPackageActivator, OperatorPackageLoadError, OperatorPackageLoadPlan, OperatorRegistry,
    OperatorSdkError, decode_operator_json_abi_response, operator_port, partial_validation,
    verified_validation,
};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorRunRequest, OperatorRunResult,
    OperatorValidationStatus,
};
use libloading::Library;
use std::slice;
use std::sync::Mutex;

#[derive(Default)]
pub struct DynamicLibraryOperatorActivator {
    loaded_libraries: Mutex<Vec<Library>>,
}

impl DynamicLibraryOperatorActivator {
    pub fn into_loaded_libraries(self) -> Vec<Library> {
        self.loaded_libraries
            .into_inner()
            .expect("dynamic library activator lock should not be poisoned")
    }
}

impl OperatorPackageActivator for DynamicLibraryOperatorActivator {
    fn activate_package(
        &self,
        plan: &OperatorPackageLoadPlan,
        registry: &mut OperatorRegistry,
    ) -> Result<(), OperatorPackageLoadError> {
        if plan.manifest.execution_abi != kyuubiki_operator_sdk::OPERATOR_JSON_ABI_SCHEMA_VERSION {
            return Err(activation_error(
                plan,
                format!(
                    "unsupported execution ABI {}; expected {}",
                    plan.manifest.execution_abi,
                    kyuubiki_operator_sdk::OPERATOR_JSON_ABI_SCHEMA_VERSION
                ),
            ));
        }
        let library = unsafe { Library::new(&plan.entrypoint_path) }.map_err(|error| {
            activation_error(
                plan,
                format!(
                    "failed to open dynamic library {}: {error}",
                    plan.entrypoint_path.display()
                ),
            )
        })?;
        let free = unsafe {
            *library
                .get::<OperatorJsonFreeEntrypoint>(OPERATOR_JSON_ABI_FREE_SYMBOL.as_bytes())
                .map_err(|error| {
                    activation_error(
                        plan,
                        format!(
                            "failed to resolve stable ABI free symbol {OPERATOR_JSON_ABI_FREE_SYMBOL}: {error}"
                        ),
                    )
                })?
        };

        registry.try_transaction(|staged_registry| {
            for operator in &plan.manifest.operators {
                let entrypoint = unsafe {
                    *library
                        .get::<OperatorJsonEntrypoint>(operator.entry_symbol.as_bytes())
                        .map_err(|error| {
                            activation_error(
                                plan,
                                format!(
                                    "failed to resolve stable JSON ABI symbol {} in {}: {error}",
                                    operator.entry_symbol,
                                    plan.entrypoint_path.display()
                                ),
                            )
                        })?
                };
                staged_registry
                    .register(DynamicJsonOperatorHandler {
                        descriptor: external_descriptor(plan, operator)?,
                        entrypoint,
                        free,
                    })
                    .map_err(|error| activation_error(plan, error.to_string()))?;
            }
            Ok::<(), OperatorPackageLoadError>(())
        })?;

        self.loaded_libraries
            .lock()
            .expect("dynamic library activator lock should not be poisoned")
            .push(library);
        Ok(())
    }
}

struct DynamicJsonOperatorHandler {
    descriptor: OperatorDescriptor,
    entrypoint: OperatorJsonEntrypoint,
    free: OperatorJsonFreeEntrypoint,
}

impl OperatorHandler for DynamicJsonOperatorHandler {
    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run(&self, request: OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError> {
        let operator_id = request.operator_id.clone();
        let request = serde_json::to_vec(&request).map_err(|error| OperatorSdkError::Handler {
            operator_id: operator_id.clone(),
            message: format!("failed to encode stable operator ABI request: {error}"),
        })?;
        if request.len() > MAX_OPERATOR_JSON_ABI_BYTES {
            return Err(OperatorSdkError::Handler {
                operator_id,
                message: format!(
                    "stable operator ABI request exceeds {MAX_OPERATOR_JSON_ABI_BYTES} bytes"
                ),
            });
        }

        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe { (self.entrypoint)(request.as_ptr(), request.len(), &mut output) };
        unsafe { decode_and_release_output(status, output, self.free) }.map_err(|message| {
            OperatorSdkError::Handler {
                operator_id,
                message,
            }
        })
    }
}

struct DynamicAbiResponseBuffer {
    output: OperatorJsonAbiBuffer,
    free: OperatorJsonFreeEntrypoint,
}

impl DynamicAbiResponseBuffer {
    unsafe fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.output.ptr, self.output.len) }
    }
}

impl Drop for DynamicAbiResponseBuffer {
    fn drop(&mut self) {
        unsafe { (self.free)(self.output) };
    }
}

unsafe fn decode_and_release_output(
    status: i32,
    output: OperatorJsonAbiBuffer,
    free: OperatorJsonFreeEntrypoint,
) -> Result<OperatorRunResult, String> {
    if output.ptr.is_null() {
        return Err("stable operator ABI returned a null response buffer".to_string());
    }
    if output.capacity < output.len {
        return Err("stable operator ABI returned an invalid response capacity".to_string());
    }
    let response = DynamicAbiResponseBuffer { output, free };
    if output.len > MAX_OPERATOR_JSON_ABI_BYTES {
        return Err(format!(
            "stable operator ABI response exceeds {MAX_OPERATOR_JSON_ABI_BYTES} bytes"
        ));
    }
    if output.len == 0 {
        return Err("stable operator ABI returned an empty response".to_string());
    }
    decode_operator_json_abi_response(status, unsafe { response.as_bytes() })
}

fn external_descriptor(
    plan: &OperatorPackageLoadPlan,
    operator: &kyuubiki_operator_sdk::OperatorPackageOperatorEntry,
) -> Result<OperatorDescriptor, OperatorPackageLoadError> {
    let validation_case = format!(
        "package:{}.{}",
        plan.manifest.package_id, operator.operator_id
    );
    let validation = match plan.manifest.validation_status {
        OperatorValidationStatus::Verified => verified_validation(validation_case),
        OperatorValidationStatus::Partial | OperatorValidationStatus::Unverified => {
            partial_validation(validation_case)
        }
    };
    Ok(OperatorDescriptorBuilder::new(
        operator.operator_id.clone(),
        operator_kind(plan, &operator.kind)?,
        "external_package",
        operator.operator_id.replace('.', "_"),
    )
    .version(plan.manifest.package_version.clone())
    .summary(format!(
        "Stable JSON ABI operator from package {}",
        plan.manifest.package_id
    ))
    .capability_tags(["external_package", "stable_json_abi", "headless_safe"])
    .input_port(operator_port(
        "input",
        "artifact/json",
        "Operator JSON input",
    ))
    .output_port(operator_port(
        "output",
        "artifact/json",
        "Operator JSON output",
    ))
    .validation(validation)
    .build())
}

fn operator_kind(
    plan: &OperatorPackageLoadPlan,
    kind: &str,
) -> Result<OperatorKind, OperatorPackageLoadError> {
    match kind {
        "solver" => Ok(OperatorKind::Solver),
        "transform" => Ok(OperatorKind::Transform),
        "extract" => Ok(OperatorKind::Extract),
        "export" => Ok(OperatorKind::Export),
        "workflow_bridge" => Ok(OperatorKind::WorkflowBridge),
        other => Err(activation_error(
            plan,
            format!("unsupported operator kind for stable JSON ABI: {other}"),
        )),
    }
}

fn activation_error(plan: &OperatorPackageLoadPlan, message: String) -> OperatorPackageLoadError {
    OperatorPackageLoadError::Activation {
        package_id: plan.manifest.package_id.clone(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_operator_sdk::{
        OPERATOR_JSON_ABI_OK, OPERATOR_JSON_ABI_SCHEMA_VERSION, free_operator_json_abi_buffer,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
    static INVALID_FREE_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn counting_free(output: OperatorJsonAbiBuffer) {
        FREE_CALLS.fetch_add(1, Ordering::SeqCst);
        unsafe { free_operator_json_abi_buffer(output) };
    }

    unsafe extern "C" fn invalid_counting_free(_output: OperatorJsonAbiBuffer) {
        INVALID_FREE_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn owned_buffer(bytes: impl Into<Vec<u8>>) -> OperatorJsonAbiBuffer {
        let mut bytes = bytes.into();
        let output = OperatorJsonAbiBuffer {
            ptr: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        };
        std::mem::forget(bytes);
        output
    }

    #[test]
    fn borrowed_response_decode_releases_exactly_once_on_every_decodable_path() {
        FREE_CALLS.store(0, Ordering::SeqCst);
        let response = serde_json::to_vec(&serde_json::json!({
            "schema_version": OPERATOR_JSON_ABI_SCHEMA_VERSION,
            "ok": true,
            "result": {
                "operator_id": "extract.response_fixture",
                "summary": {"values": [1, 2, 3]},
                "artifacts": []
            }
        }))
        .expect("response JSON");
        let result = unsafe {
            decode_and_release_output(OPERATOR_JSON_ABI_OK, owned_buffer(response), counting_free)
        }
        .expect("borrowed response should decode");
        assert_eq!(result.summary["values"][2], 3);
        assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 1);

        let error = unsafe {
            decode_and_release_output(
                OPERATOR_JSON_ABI_OK,
                owned_buffer(b"{".to_vec()),
                counting_free,
            )
        }
        .expect_err("malformed response should fail");
        assert!(error.contains("invalid operator ABI response JSON"));
        assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn invalid_response_capacity_is_rejected_without_calling_untrusted_free() {
        INVALID_FREE_CALLS.store(0, Ordering::SeqCst);
        let output = OperatorJsonAbiBuffer {
            ptr: std::ptr::dangling_mut(),
            len: 2,
            capacity: 1,
        };
        let error = unsafe {
            decode_and_release_output(OPERATOR_JSON_ABI_OK, output, invalid_counting_free)
        }
        .expect_err("invalid capacity must fail closed");
        assert!(error.contains("invalid response capacity"));
        assert_eq!(INVALID_FREE_CALLS.load(Ordering::SeqCst), 0);
    }
}
