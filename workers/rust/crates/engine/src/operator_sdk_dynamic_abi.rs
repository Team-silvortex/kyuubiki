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
        let response =
            unsafe { copy_and_release_output(output, self.free) }.map_err(|message| {
                OperatorSdkError::Handler {
                    operator_id: operator_id.clone(),
                    message,
                }
            })?;
        decode_operator_json_abi_response(status, &response).map_err(|message| {
            OperatorSdkError::Handler {
                operator_id,
                message,
            }
        })
    }
}

unsafe fn copy_and_release_output(
    output: OperatorJsonAbiBuffer,
    free: OperatorJsonFreeEntrypoint,
) -> Result<Vec<u8>, String> {
    if output.ptr.is_null() {
        return Err("stable operator ABI returned a null response buffer".to_string());
    }
    if output.capacity < output.len {
        return Err("stable operator ABI returned an invalid response capacity".to_string());
    }
    if output.len > MAX_OPERATOR_JSON_ABI_BYTES {
        unsafe { free(output) };
        return Err(format!(
            "stable operator ABI response exceeds {MAX_OPERATOR_JSON_ABI_BYTES} bytes"
        ));
    }
    let response = unsafe { slice::from_raw_parts(output.ptr, output.len) }.to_vec();
    unsafe { free(output) };
    if response.is_empty() {
        Err("stable operator ABI returned an empty response".to_string())
    } else {
        Ok(response)
    }
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
