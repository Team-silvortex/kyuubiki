use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{RpcMethod, task_digest_v2};

pub const DURABLE_AGENT_EXECUTION_SCHEMA: &str = "kyuubiki.agent-durable-execution/v1";
pub const DURABLE_AGENT_INDEX_SCHEMA: &str = "kyuubiki.agent-durable-execution-index/v1";
pub const DURABLE_AGENT_CANCELLATION_SCHEMA: &str = "kyuubiki.agent-durable-cancellation/v1";
pub const DURABLE_AGENT_REJECTION_SCHEMA: &str = "kyuubiki.agent-durable-admission-rejection/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableAgentExecutionList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_store_id: Option<String>,
    pub limit: usize,
    pub after: Option<String>,
}

impl DurableAgentExecutionList {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(id) = &self.expected_store_id {
            validate_durable_execution_id(id)?;
        }
        if !(1..=64).contains(&self.limit) {
            return Err("execution index limit must be between 1 and 64".into());
        }
        if let Some(id) = &self.after {
            validate_durable_execution_id(id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableAgentExecutionCancel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_store_id: Option<String>,
    pub execution_id: String,
    pub process_instance_id: String,
    pub request_digest: String,
}

impl DurableAgentExecutionCancel {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(id) = &self.expected_store_id {
            validate_durable_execution_id(id)?;
        }
        validate_durable_execution_id(&self.execution_id)?;
        if self.process_instance_id.is_empty()
            || self.process_instance_id.len() > 256
            || self.request_digest.len() != 64
            || !self
                .request_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(
                "durable cancellation requires the inspected process and request digest".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableAgentTask {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_store_id: Option<String>,
    pub execution_id: String,
    pub method: RpcMethod,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableAgentExecutionQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_store_id: Option<String>,
    pub execution_id: String,
}

pub fn validate_durable_execution_id(id: &str) -> Result<(), String> {
    if id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err("execution_id must be a caller-generated 32-character lowercase hex identifier".into())
    }
}

impl DurableAgentTask {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(id) = &self.expected_store_id {
            validate_durable_execution_id(id)?;
        }
        validate_durable_execution_id(&self.execution_id)?;
        let method = serde_json::to_value(&self.method).map_err(|error| error.to_string())?;
        let solver = method
            .as_str()
            .is_some_and(|value| value.starts_with("solve_"));
        let task = self.method == RpcMethod::RunOperatorTaskIr && self.params["mode"] == "execute";
        if !(solver || task) || !self.params.is_object() {
            return Err(
                "durable tasks require a solver RPC or execute-mode TaskIR with object params"
                    .into(),
            );
        }
        Ok(())
    }

    pub fn request_digest(&self) -> Result<String, String> {
        self.validate()?;
        // Preserve binary64 distinctions; the legacy fixed-decimal digest is unsuitable here.
        task_digest_v2::agent_execution_digest(&self.method, &self.params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn durable_identity_binds_method_and_lossless_inputs_not_transport_id() {
        let mut task = DurableAgentTask {
            expected_store_id: None,
            execution_id: "a".repeat(32),
            method: RpcMethod::SolveBar1d,
            params: json!({"tip_force": 1.0000000000000002}),
        };
        let digest = task.request_digest().unwrap();
        task.execution_id = "b".repeat(32);
        assert_eq!(digest, task.request_digest().unwrap());
        task.expected_store_id = Some("c".repeat(32));
        assert_eq!(digest, task.request_digest().unwrap());
        task.expected_store_id = Some("invalid".into());
        assert!(task.request_digest().is_err());
        task.expected_store_id = None;
        task.params["tip_force"] = json!(1.0000000000000004);
        assert_ne!(digest, task.request_digest().unwrap());
        task.method = RpcMethod::Ping;
        assert!(task.validate().is_err());
        task.method = RpcMethod::RunDurableAgentTask;
        assert!(task.validate().is_err());
        task.method = RpcMethod::SubmitDurableAgentTask;
        assert!(task.validate().is_err());
        assert_eq!(
            serde_json::to_value(RpcMethod::SubmitDurableAgentTask).unwrap(),
            "submit_durable_agent_task"
        );
        for id in ["", "../execution", &"A".repeat(32)] {
            assert!(validate_durable_execution_id(id).is_err());
        }
    }
}
