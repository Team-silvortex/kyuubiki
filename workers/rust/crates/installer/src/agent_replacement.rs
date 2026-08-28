use crate::{AgentLifecycleControl, AgentLifecycleControlError};
use kyuubiki_protocol::AgentLifecycleDescriptor;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const AGENT_REPLACEMENT_RECEIPT_SCHEMA_VERSION: &str = "kyuubiki.agent-replacement-receipt/v1";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReplacementReceipt {
    pub schema_version: String,
    pub node_id: String,
    pub controller_id: String,
    pub drain_generation: u64,
    pub previous_process_instance_id: String,
    pub active_process_instance_id: String,
    pub quiescent_observed: bool,
    pub replacement_verified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentReplacementFailure {
    pub node_id: String,
    pub failed_stage: String,
    pub cause: String,
    pub compensated: bool,
    pub compensation_errors: Vec<String>,
}

impl fmt::Display for AgentReplacementFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Agent {} replacement failed at {}: {}; compensated={}",
            self.node_id, self.failed_stage, self.cause, self.compensated
        )?;
        if !self.compensation_errors.is_empty() {
            write!(
                formatter,
                "; compensation errors: {}",
                self.compensation_errors.join("; ")
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for AgentReplacementFailure {}

pub fn replace_agent_with_drain<Control, Replace, Compensate>(
    control: &Control,
    node_id: &str,
    controller_id: &str,
    reason: &str,
    replace: Replace,
    compensate: Compensate,
) -> Result<AgentReplacementReceipt, AgentReplacementFailure>
where
    Control: AgentLifecycleControl,
    Replace: FnOnce() -> Result<(), String>,
    Compensate: FnOnce() -> Result<(), String>,
{
    let before = control
        .describe()
        .map_err(|error| failure(node_id, "lifecycle-preflight", error, false, vec![]))?;
    if !before.accepting_new_work && before.drain_owner_id.as_deref() != Some(controller_id) {
        return Err(failure_message(
            node_id,
            "lifecycle-preflight",
            "Agent is controlled by another active drain lease",
            false,
            vec![],
        ));
    }

    let draining = control
        .begin_drain(controller_id, reason)
        .map_err(|error| failure(node_id, "begin-drain", error, false, vec![]))?;
    let generation = draining.drain_generation;
    let quiescent = match control.wait_until_quiescent(controller_id, generation) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let compensation_errors = resume_errors(control, controller_id, generation);
            return Err(failure(
                node_id,
                "wait-quiescent",
                error,
                compensation_errors.is_empty(),
                compensation_errors,
            ));
        }
    };

    if let Err(cause) = replace() {
        return Err(compensate_replacement_failure(
            control,
            node_id,
            controller_id,
            generation,
            &before.process_instance_id,
            "replace-process",
            cause,
            compensate,
        ));
    }

    let active = match control.wait_until_replaced(&before.process_instance_id) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Err(compensate_replacement_failure(
                control,
                node_id,
                controller_id,
                generation,
                &before.process_instance_id,
                "verify-replacement",
                error.to_string(),
                compensate,
            ));
        }
    };

    Ok(AgentReplacementReceipt {
        schema_version: AGENT_REPLACEMENT_RECEIPT_SCHEMA_VERSION.to_string(),
        node_id: node_id.to_string(),
        controller_id: controller_id.to_string(),
        drain_generation: generation,
        previous_process_instance_id: before.process_instance_id,
        active_process_instance_id: active.process_instance_id,
        quiescent_observed: quiescent.quiescent && quiescent.safe_to_replace,
        replacement_verified: true,
    })
}

#[allow(clippy::too_many_arguments)]
fn compensate_replacement_failure<Control, Compensate>(
    control: &Control,
    node_id: &str,
    controller_id: &str,
    generation: u64,
    previous_instance_id: &str,
    stage: &str,
    cause: String,
    compensate: Compensate,
) -> AgentReplacementFailure
where
    Control: AgentLifecycleControl,
    Compensate: FnOnce() -> Result<(), String>,
{
    let mut errors = Vec::new();
    if let Err(error) = compensate() {
        errors.push(format!("replacement compensation failed: {error}"));
    }
    if let Err(error) =
        restore_service_admission(control, controller_id, generation, previous_instance_id)
    {
        errors.push(error);
    }
    failure_message(node_id, stage, cause, errors.is_empty(), errors)
}

fn restore_service_admission(
    control: &impl AgentLifecycleControl,
    controller_id: &str,
    generation: u64,
    previous_instance_id: &str,
) -> Result<AgentLifecycleDescriptor, String> {
    match control.describe() {
        Ok(snapshot) if snapshot.process_instance_id != previous_instance_id => {
            if snapshot.state == "accepting" && snapshot.accepting_new_work {
                Ok(snapshot)
            } else {
                Err("compensated Agent process is not accepting work".to_string())
            }
        }
        Ok(snapshot) if snapshot.state == "accepting" && snapshot.accepting_new_work => {
            Ok(snapshot)
        }
        Ok(_) => control
            .resume_admission(controller_id, generation)
            .map_err(|error| format!("failed to resume original Agent admission: {error}")),
        Err(error) if error.retryable => {
            control
                .wait_until_replaced(previous_instance_id)
                .map_err(|wait_error| {
                    format!("compensated Agent did not recover: {error}; {wait_error}")
                })
        }
        Err(error) => Err(format!("failed to inspect compensated Agent: {error}")),
    }
}

fn resume_errors(
    control: &impl AgentLifecycleControl,
    controller_id: &str,
    generation: u64,
) -> Vec<String> {
    control
        .resume_admission(controller_id, generation)
        .err()
        .map(|error| vec![format!("failed to abort Agent drain: {error}")])
        .unwrap_or_default()
}

fn failure(
    node_id: &str,
    stage: &str,
    error: AgentLifecycleControlError,
    compensated: bool,
    compensation_errors: Vec<String>,
) -> AgentReplacementFailure {
    failure_message(
        node_id,
        stage,
        error.to_string(),
        compensated,
        compensation_errors,
    )
}

fn failure_message(
    node_id: &str,
    stage: &str,
    cause: impl Into<String>,
    compensated: bool,
    compensation_errors: Vec<String>,
) -> AgentReplacementFailure {
    AgentReplacementFailure {
        node_id: node_id.to_string(),
        failed_stage: stage.to_string(),
        cause: cause.into(),
        compensated,
        compensation_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct FakeControl {
        lifecycle: Arc<Mutex<AgentLifecycleDescriptor>>,
    }

    impl AgentLifecycleControl for FakeControl {
        fn describe(&self) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
            Ok(self.lifecycle.lock().unwrap().clone())
        }

        fn begin_drain(
            &self,
            controller_id: &str,
            reason: &str,
        ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
            let mut lifecycle = self.lifecycle.lock().unwrap();
            lifecycle.state = "quiescent".to_string();
            lifecycle.drain_generation += 1;
            lifecycle.accepting_new_work = false;
            lifecycle.quiescent = true;
            lifecycle.safe_to_replace = true;
            lifecycle.drain_owner_id = Some(controller_id.to_string());
            lifecycle.drain_reason = Some(reason.to_string());
            lifecycle.drain_started_unix_ms = Some(1);
            Ok(lifecycle.clone())
        }

        fn wait_until_quiescent(
            &self,
            _controller_id: &str,
            _drain_generation: u64,
        ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
            self.describe()
        }

        fn wait_until_replaced(
            &self,
            previous_process_instance_id: &str,
        ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
            let lifecycle = self.describe()?;
            if lifecycle.process_instance_id == previous_process_instance_id {
                Err(AgentLifecycleControlError {
                    code: "agent_replacement_timeout".to_string(),
                    message: "not replaced".to_string(),
                    retryable: true,
                })
            } else {
                Ok(lifecycle)
            }
        }

        fn resume_admission(
            &self,
            _controller_id: &str,
            _drain_generation: u64,
        ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
            let mut lifecycle = self.lifecycle.lock().unwrap();
            *lifecycle = accepting(&lifecycle.process_instance_id);
            Ok(lifecycle.clone())
        }
    }

    #[test]
    fn replacement_requires_a_new_accepting_process_instance() {
        let control = FakeControl {
            lifecycle: Arc::new(Mutex::new(accepting("agent-old"))),
        };
        let replacement_state = Arc::clone(&control.lifecycle);
        let receipt = replace_agent_with_drain(
            &control,
            "agent-01",
            "installer-1",
            "upgrade",
            move || {
                *replacement_state.lock().unwrap() = accepting("agent-new");
                Ok(())
            },
            || Ok(()),
        )
        .unwrap();
        assert_eq!(receipt.previous_process_instance_id, "agent-old");
        assert_eq!(receipt.active_process_instance_id, "agent-new");
        assert!(receipt.quiescent_observed);
        assert!(receipt.replacement_verified);
    }

    #[test]
    fn failed_replacement_resumes_the_original_agent() {
        let control = FakeControl {
            lifecycle: Arc::new(Mutex::new(accepting("agent-old"))),
        };
        let failure = replace_agent_with_drain(
            &control,
            "agent-01",
            "installer-1",
            "upgrade",
            || Err("injected restart failure".to_string()),
            || Ok(()),
        )
        .unwrap_err();
        assert_eq!(failure.failed_stage, "replace-process");
        assert!(failure.compensated);
        assert_eq!(control.describe().unwrap().state, "accepting");
    }

    fn accepting(instance_id: &str) -> AgentLifecycleDescriptor {
        AgentLifecycleDescriptor {
            process_instance_id: instance_id.to_string(),
            ..AgentLifecycleDescriptor::default()
        }
    }
}
