use serde::{Deserialize, Serialize};

pub const EXECUTION_AUTHORITY_SCHEMA_VERSION: &str = "kyuubiki.execution-authority/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAuthority {
    pub schema_version: String,
    pub execution_class: String,
    pub executor_id: String,
    pub runtime: String,
    pub result_origin: String,
    pub mock_execution: bool,
    pub fallback_used: bool,
    pub production_eligible: bool,
    pub evidence_statement: String,
}

impl ExecutionAuthority {
    pub fn local_rust_solver() -> Self {
        Self {
            schema_version: EXECUTION_AUTHORITY_SCHEMA_VERSION.to_string(),
            execution_class: "real_solver".to_string(),
            executor_id: "kyuubiki.rust.local-solver".to_string(),
            runtime: "rust_native".to_string(),
            result_origin: "computed_in_process".to_string(),
            mock_execution: false,
            fallback_used: false,
            production_eligible: true,
            evidence_statement:
                "result payloads were computed by linked Rust solver kernels without mock fallback"
                    .to_string(),
        }
    }

    pub fn from_material_mode(mode: &str) -> Self {
        if mode.starts_with("local_solver") {
            return Self::local_rust_solver();
        }
        if mode.contains("mock") {
            return Self {
                schema_version: EXECUTION_AUTHORITY_SCHEMA_VERSION.to_string(),
                execution_class: "mock".to_string(),
                executor_id: mode.to_string(),
                runtime: "in_process".to_string(),
                result_origin: "synthetic".to_string(),
                mock_execution: true,
                fallback_used: false,
                production_eligible: false,
                evidence_statement: "result payloads were produced by a mock executor".to_string(),
            };
        }
        Self {
            schema_version: EXECUTION_AUTHORITY_SCHEMA_VERSION.to_string(),
            execution_class: "unverified".to_string(),
            executor_id: mode.to_string(),
            runtime: "unknown".to_string(),
            result_origin: "caller_supplied".to_string(),
            mock_execution: false,
            fallback_used: false,
            production_eligible: false,
            evidence_statement: "execution authority was not verified by this SDK boundary"
                .to_string(),
        }
    }
}

pub fn validate_execution_authority(authority: &ExecutionAuthority) -> Result<(), String> {
    if authority.schema_version != EXECUTION_AUTHORITY_SCHEMA_VERSION {
        return Err(format!(
            "execution authority schema must be {EXECUTION_AUTHORITY_SCHEMA_VERSION}"
        ));
    }
    for (field, value) in [
        ("execution_class", authority.execution_class.as_str()),
        ("executor_id", authority.executor_id.as_str()),
        ("runtime", authority.runtime.as_str()),
        ("result_origin", authority.result_origin.as_str()),
        ("evidence_statement", authority.evidence_statement.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("execution authority {field} must not be empty"));
        }
    }
    if authority.production_eligible
        && (authority.mock_execution
            || authority.fallback_used
            || authority.execution_class != "real_solver")
    {
        return Err(
            "production-eligible execution authority must be a real solver without fallback"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ExecutionAuthority, validate_execution_authority};

    #[test]
    fn local_solver_authority_is_real_and_production_eligible() {
        let authority = ExecutionAuthority::local_rust_solver();

        validate_execution_authority(&authority).expect("valid authority");
        assert_eq!(authority.execution_class, "real_solver");
        assert!(!authority.mock_execution);
        assert!(!authority.fallback_used);
        assert!(authority.production_eligible);
    }

    #[test]
    fn unknown_material_mode_stays_unverified() {
        let authority = ExecutionAuthority::from_material_mode("unit-test");

        validate_execution_authority(&authority).expect("valid authority");
        assert_eq!(authority.execution_class, "unverified");
        assert!(!authority.production_eligible);
    }
}
