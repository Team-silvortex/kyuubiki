mod builder;
mod loader;
mod manifest;
#[cfg(test)]
mod manifest_fuzz;
mod readiness;
mod surface;
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub use builder::{
    OperatorDescriptorBuilder, operator_port, operator_port_with_dataset, operator_summary_result,
    partial_validation, verified_validation,
};
pub use kyuubiki_platform::{
    LIB_EXTENSION_PLACEHOLDER, LIB_PREFIX_PLACEHOLDER, current_platform_library_extension,
    current_platform_library_file_name, current_platform_library_path,
    current_platform_library_prefix, expand_platform_library_template,
};
pub use loader::{
    OperatorPackageActivator, OperatorPackageLoadError, OperatorPackageLoadPlan,
    OperatorPackageLoadSummary, activate_discovered_operator_packages,
    build_operator_package_load_plan, discover_and_activate_operator_packages,
};
pub use manifest::{
    DiscoveredOperatorPackage, OPERATOR_PACKAGE_MANIFEST_FILE, OPERATOR_PACKAGE_SCHEMA_VERSION,
    OPERATOR_SDK_API_VERSION, OperatorManifestError, OperatorPackageManifest,
    OperatorPackageOperatorEntry, discover_operator_packages, read_operator_package_manifest,
};
pub use readiness::{
    OperatorSdkReadinessIssue, OperatorSdkReadinessReport, OperatorSdkReadinessSeverity,
    operator_descriptor_readiness, operator_package_descriptor_readiness,
    operator_package_manifest_readiness,
};
pub use surface::{
    OPERATOR_SDK_SURFACE_SCHEMA_VERSION, OperatorSdkSurfaceArea, OperatorSdkSurfaceManifest,
    find_operator_sdk_surface_area, operator_sdk_surface_areas, operator_sdk_surface_manifest,
};

pub type OperatorRegistrationEntrypoint =
    unsafe fn(&mut OperatorRegistry) -> Result<(), OperatorSdkError>;

pub trait OperatorHandler: Send + Sync {
    fn descriptor(&self) -> &OperatorDescriptor;
    fn run(&self, request: OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError>;
}

pub trait JsonOperator: Send + Sync {
    type Input: DeserializeOwned;

    fn descriptor(&self) -> &OperatorDescriptor;

    fn run_typed(
        &self,
        input: Self::Input,
        context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError>;
}

pub struct JsonOperatorAdapter<T> {
    inner: T,
}

impl<T> JsonOperatorAdapter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> OperatorHandler for JsonOperatorAdapter<T>
where
    T: JsonOperator,
{
    fn descriptor(&self) -> &OperatorDescriptor {
        self.inner.descriptor()
    }

    fn run(&self, request: OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError> {
        let decoded = serde_json::from_value::<T::Input>(request.input).map_err(|error| {
            OperatorSdkError::DecodeInput {
                operator_id: request.operator_id.clone(),
                message: error.to_string(),
            }
        })?;
        self.inner.run_typed(decoded, &request.context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorSdkError {
    DuplicateOperator {
        operator_id: String,
    },
    UnknownOperator {
        operator_id: String,
    },
    DecodeInput {
        operator_id: String,
        message: String,
    },
    Handler {
        operator_id: String,
        message: String,
    },
}

impl Display for OperatorSdkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateOperator { operator_id } => {
                write!(f, "operator already registered: {operator_id}")
            }
            Self::UnknownOperator { operator_id } => {
                write!(f, "operator is not registered: {operator_id}")
            }
            Self::DecodeInput {
                operator_id,
                message,
            } => {
                write!(f, "operator {operator_id} input decode failed: {message}")
            }
            Self::Handler {
                operator_id,
                message,
            } => {
                write!(f, "operator {operator_id} failed: {message}")
            }
        }
    }
}

impl std::error::Error for OperatorSdkError {}

#[derive(Default)]
pub struct OperatorRegistry {
    handlers: BTreeMap<String, Arc<dyn OperatorHandler>>,
}

impl OperatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<H>(&mut self, handler: H) -> Result<(), OperatorSdkError>
    where
        H: OperatorHandler + 'static,
    {
        self.insert_handler(Arc::new(handler))
    }

    pub fn register_json<T>(&mut self, operator: T) -> Result<(), OperatorSdkError>
    where
        T: JsonOperator + 'static,
    {
        self.insert_handler(Arc::new(JsonOperatorAdapter::new(operator)))
    }

    pub fn descriptors(&self) -> Vec<OperatorDescriptor> {
        self.handlers
            .values()
            .map(|handler| handler.descriptor().clone())
            .collect()
    }

    pub fn describe(&self, operator_id: &str) -> Option<OperatorDescriptor> {
        self.handlers
            .get(operator_id)
            .map(|handler| handler.descriptor().clone())
    }

    pub fn run(&self, request: OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError> {
        let handler = self.handlers.get(&request.operator_id).ok_or_else(|| {
            OperatorSdkError::UnknownOperator {
                operator_id: request.operator_id.clone(),
            }
        })?;
        handler.run(request)
    }

    fn insert_handler(
        &mut self,
        handler: Arc<dyn OperatorHandler>,
    ) -> Result<(), OperatorSdkError> {
        let operator_id = handler.descriptor().id.clone();
        if self.handlers.contains_key(&operator_id) {
            return Err(OperatorSdkError::DuplicateOperator { operator_id });
        }
        self.handlers.insert(operator_id, handler);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        JsonOperator, OperatorDescriptorBuilder, OperatorRegistry, OperatorSdkError,
        operator_summary_result, partial_validation,
    };
    use kyuubiki_protocol::{
        OperatorDescriptor, OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
    };
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct EchoInput {
        value: i64,
    }

    struct EchoOperator {
        descriptor: OperatorDescriptor,
    }

    impl EchoOperator {
        fn new() -> Self {
            Self {
                descriptor: OperatorDescriptorBuilder::new(
                    "transform.echo_integer",
                    OperatorKind::Transform,
                    "multi_domain",
                    "echo_integer",
                )
                .summary("Echo an integer payload for operator-SDK testing.")
                .capability_tags(["test", "headless_safe"])
                .validation(partial_validation("operator_sdk_echo"))
                .build(),
            }
        }
    }

    impl JsonOperator for EchoOperator {
        type Input = EchoInput;

        fn descriptor(&self) -> &OperatorDescriptor {
            &self.descriptor
        }

        fn run_typed(
            &self,
            input: Self::Input,
            context: &OperatorRunContext,
        ) -> Result<OperatorRunResult, OperatorSdkError> {
            Ok(operator_summary_result(
                self.descriptor.id.clone(),
                serde_json::json!({
                    "echo": input.value,
                    "project_id": context.project_id,
                    "orchestrated": context.orchestrated,
                }),
            ))
        }
    }

    #[test]
    fn registers_json_operator_and_runs_it() {
        let mut registry = OperatorRegistry::new();
        registry
            .register_json(EchoOperator::new())
            .expect("operator should register");

        let descriptors = registry.descriptors();
        assert_eq!(descriptors.len(), 1);
        assert_eq!(descriptors[0].id, "transform.echo_integer");

        let result = registry
            .run(OperatorRunRequest {
                operator_id: "transform.echo_integer".to_string(),
                input: serde_json::json!({ "value": 42 }),
                context: OperatorRunContext {
                    orchestrated: true,
                    project_id: Some("project-alpha".to_string()),
                    model_id: None,
                    workflow_run_id: None,
                },
            })
            .expect("operator should run");

        assert_eq!(result.summary["echo"].as_i64(), Some(42));
        assert_eq!(result.summary["project_id"].as_str(), Some("project-alpha"));
    }

    #[test]
    fn rejects_duplicate_operator_registration() {
        let mut registry = OperatorRegistry::new();
        registry
            .register_json(EchoOperator::new())
            .expect("first registration should succeed");

        let error = registry
            .register_json(EchoOperator::new())
            .expect_err("duplicate registration should fail");

        assert_eq!(
            error,
            OperatorSdkError::DuplicateOperator {
                operator_id: "transform.echo_integer".to_string(),
            }
        );
    }

    #[test]
    fn reports_decode_failures_with_operator_context() {
        let mut registry = OperatorRegistry::new();
        registry
            .register_json(EchoOperator::new())
            .expect("operator should register");

        let error = registry
            .run(OperatorRunRequest {
                operator_id: "transform.echo_integer".to_string(),
                input: serde_json::json!({ "value": "not-an-integer" }),
                context: OperatorRunContext::default(),
            })
            .expect_err("invalid input should fail");

        match error {
            OperatorSdkError::DecodeInput { operator_id, .. } => {
                assert_eq!(operator_id, "transform.echo_integer");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
