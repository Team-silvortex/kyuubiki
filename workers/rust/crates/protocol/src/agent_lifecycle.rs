use crate::{
    AGENT_DRAIN_CONTROLLER_ID_MAX_BYTES, AGENT_DRAIN_REASON_MAX_BYTES, AGENT_LIFECYCLE_SCHEMA,
    AGENT_PROCESS_INSTANCE_ID_MAX_BYTES, AgentLifecycleDescriptor,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AgentLifecycleValidationError {
    pub code: String,
    pub message: String,
}

pub fn validate_agent_lifecycle_descriptor(
    descriptor: &AgentLifecycleDescriptor,
) -> Result<(), Vec<AgentLifecycleValidationError>> {
    let mut errors = Vec::new();
    if descriptor.schema_version != AGENT_LIFECYCLE_SCHEMA {
        push_error(
            &mut errors,
            "unsupported_schema",
            "agent lifecycle schema_version is unsupported",
        );
    }
    if !valid_process_instance_id(&descriptor.process_instance_id) {
        push_error(
            &mut errors,
            "invalid_process_instance_id",
            "agent lifecycle process_instance_id is invalid",
        );
    }
    if descriptor.mutation_control_scope != "host_loopback" {
        push_error(
            &mut errors,
            "unsupported_mutation_control_scope",
            "agent lifecycle mutations must remain restricted to host loopback control",
        );
    }

    let has_complete_drain_metadata = descriptor.drain_owner_id.is_some()
        && descriptor.drain_reason.is_some()
        && descriptor.drain_started_unix_ms.is_some();
    let has_any_drain_metadata = descriptor.drain_owner_id.is_some()
        || descriptor.drain_reason.is_some()
        || descriptor.drain_started_unix_ms.is_some();
    if has_any_drain_metadata != has_complete_drain_metadata {
        push_error(
            &mut errors,
            "partial_drain_metadata",
            "agent lifecycle drain metadata must be complete or absent",
        );
    }
    if let Some(owner) = descriptor.drain_owner_id.as_deref()
        && !valid_controller_id(owner)
    {
        push_error(
            &mut errors,
            "invalid_drain_owner_id",
            "agent lifecycle drain_owner_id is invalid",
        );
    }
    if let Some(reason) = descriptor.drain_reason.as_deref()
        && !valid_reason(reason)
    {
        push_error(
            &mut errors,
            "invalid_drain_reason",
            "agent lifecycle drain_reason is invalid",
        );
    }

    match descriptor.state.as_str() {
        "accepting" => validate_accepting(descriptor, has_any_drain_metadata, &mut errors),
        "draining" => validate_draining(descriptor, has_complete_drain_metadata, &mut errors),
        "quiescent" => validate_quiescent(descriptor, has_complete_drain_metadata, &mut errors),
        "unavailable" => validate_unavailable(descriptor, has_any_drain_metadata, &mut errors),
        _ => push_error(
            &mut errors,
            "unsupported_state",
            "agent lifecycle state is unsupported",
        ),
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_accepting(
    descriptor: &AgentLifecycleDescriptor,
    has_metadata: bool,
    errors: &mut Vec<AgentLifecycleValidationError>,
) {
    if !descriptor.accepting_new_work
        || descriptor.quiescent
        || descriptor.safe_to_replace
        || has_metadata
    {
        push_error(
            errors,
            "inconsistent_accepting_state",
            "accepting state must accept work and must not claim drain safety",
        );
    }
}

fn validate_draining(
    descriptor: &AgentLifecycleDescriptor,
    has_metadata: bool,
    errors: &mut Vec<AgentLifecycleValidationError>,
) {
    if descriptor.accepting_new_work
        || descriptor.active_execution_count == 0
        || descriptor.quiescent
        || descriptor.safe_to_replace
        || descriptor.drain_generation == 0
        || !has_metadata
    {
        push_error(
            errors,
            "inconsistent_draining_state",
            "draining state requires a fenced lease and at least one active execution",
        );
    }
}

fn validate_quiescent(
    descriptor: &AgentLifecycleDescriptor,
    has_metadata: bool,
    errors: &mut Vec<AgentLifecycleValidationError>,
) {
    if descriptor.accepting_new_work
        || descriptor.active_execution_count != 0
        || !descriptor.quiescent
        || !descriptor.safe_to_replace
        || descriptor.drain_generation == 0
        || !has_metadata
    {
        push_error(
            errors,
            "inconsistent_quiescent_state",
            "quiescent state requires a fenced drain lease and zero active executions",
        );
    }
}

fn validate_unavailable(
    descriptor: &AgentLifecycleDescriptor,
    has_metadata: bool,
    errors: &mut Vec<AgentLifecycleValidationError>,
) {
    if descriptor.accepting_new_work
        || descriptor.quiescent
        || descriptor.safe_to_replace
        || has_metadata
    {
        push_error(
            errors,
            "inconsistent_unavailable_state",
            "unavailable state must fail closed without replacement safety",
        );
    }
}

fn valid_controller_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= AGENT_DRAIN_CONTROLLER_ID_MAX_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn valid_process_instance_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= AGENT_PROCESS_INSTANCE_ID_MAX_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn valid_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= AGENT_DRAIN_REASON_MAX_BYTES
        && !value.chars().any(char::is_control)
}

fn push_error(errors: &mut Vec<AgentLifecycleValidationError>, code: &str, message: &str) {
    errors.push(AgentLifecycleValidationError {
        code: code.to_string(),
        message: message.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_consistent_accepting_draining_and_quiescent_states() {
        validate_agent_lifecycle_descriptor(&AgentLifecycleDescriptor::default()).unwrap();
        let draining = draining_descriptor(1);
        validate_agent_lifecycle_descriptor(&draining).unwrap();
        let quiescent = AgentLifecycleDescriptor {
            state: "quiescent".to_string(),
            active_execution_count: 0,
            quiescent: true,
            safe_to_replace: true,
            ..draining
        };
        validate_agent_lifecycle_descriptor(&quiescent).unwrap();
    }

    #[test]
    fn rejects_false_safe_to_replace_claims() {
        let invalid = AgentLifecycleDescriptor {
            state: "quiescent".to_string(),
            active_execution_count: 1,
            quiescent: true,
            safe_to_replace: true,
            ..draining_descriptor(1)
        };
        let errors = validate_agent_lifecycle_descriptor(&invalid).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "inconsistent_quiescent_state")
        );
    }

    fn draining_descriptor(active_execution_count: usize) -> AgentLifecycleDescriptor {
        AgentLifecycleDescriptor {
            schema_version: AGENT_LIFECYCLE_SCHEMA.to_string(),
            process_instance_id: "agent-instance-test".to_string(),
            mutation_control_scope: "host_loopback".to_string(),
            state: "draining".to_string(),
            drain_generation: 1,
            accepting_new_work: false,
            active_execution_count,
            quiescent: false,
            safe_to_replace: false,
            drain_owner_id: Some("installer-1".to_string()),
            drain_reason: Some("rolling replacement".to_string()),
            drain_started_unix_ms: Some(1),
        }
    }
}
