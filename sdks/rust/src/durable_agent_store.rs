use crate::{
    SdkResult, SolverRpcClient,
    durable_execution::{hex, invalid},
};
use serde_json::Value;

/// Read-only descriptor snapshot, not an admission reservation, task state or
/// authentication. A busy/unavailable descriptor does not mean a task is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableAgentStoreStatus {
    Ready { store_id: String },
    Disabled,
    Busy,
    Unavailable { reason: String },
    AdmissionBlocked { store_id: String },
}

impl DurableAgentStoreStatus {
    /// Parse the durable store section of a `describe_agent` result, failing
    /// closed on missing/contradictory fields instead of inferring readiness from ID.
    pub fn from_descriptor(descriptor: &Value) -> SdkResult<Self> {
        let policy = &descriptor["durable_execution"];
        if policy["schema_version"] != "kyuubiki.agent-durable-execution/v1"
            || policy["automatic_replay_authorized"] != false
        {
            return Err(invalid(
                "unsupported durable store descriptor or replay policy",
            ));
        }
        let enabled = boolean(policy, "enabled")?;
        if boolean(policy, "survives_agent_restart")? != enabled {
            return Err(invalid(
                "durable store restart policy contradicts enabled state",
            ));
        }
        if let Some(id) = policy.get("store_id").filter(|v| !v.is_null()) {
            hex(
                id.as_str()
                    .ok_or_else(|| invalid("invalid durable store identity"))?,
                32,
            )?;
        }
        if !policy["admission_blocked_after_write_failure"].is_null() {
            boolean(policy, "admission_blocked_after_write_failure")?;
        }
        let inventory_blocked = match policy.get("admission_blocked_after_inventory_change") {
            None => false, // Older Agents do not expose the inventory fence.
            Some(_) => boolean(policy, "admission_blocked_after_inventory_change")?,
        };
        if !enabled {
            if !(policy["available"].is_null() || policy["available"] == false)
                || !policy["store_id"].is_null()
                || !policy["unavailable_reason"].is_null()
                || policy["admission_blocked_after_write_failure"] == true
                || inventory_blocked
            {
                return Err(invalid(
                    "disabled durable store contains contradictory availability",
                ));
            }
            return Ok(Self::Disabled);
        }
        if !boolean(policy, "available")? {
            let reason = policy["unavailable_reason"]
                .as_str()
                .filter(|s| {
                    !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
                })
                .ok_or_else(|| invalid("unavailable durable store requires a bounded reason"))?;
            return Ok(if reason == "store_busy" {
                Self::Busy
            } else {
                Self::Unavailable {
                    reason: reason.into(),
                }
            });
        }
        if !policy["unavailable_reason"].is_null() {
            return Err(invalid(
                "available durable store has a contradictory unavailable reason",
            ));
        }
        let id = policy["store_id"]
            .as_str()
            .ok_or_else(|| invalid("available durable store is missing its identity"))?;
        let blocked = boolean(policy, "admission_blocked_after_write_failure")?;
        Ok(if blocked || inventory_blocked {
            Self::AdmissionBlocked {
                store_id: id.into(),
            }
        } else {
            Self::Ready {
                store_id: id.into(),
            }
        })
    }

    /// Preflight new work only. Existing pinned receipts can still be inspected
    /// when new admission is blocked. No polling, submission or replay occurs here.
    pub fn ready_store_id(&self) -> SdkResult<&str> {
        match self {
            Self::Ready { store_id } => {
                hex(store_id, 32)?;
                Ok(store_id)
            }
            Self::Disabled => Err(invalid(
                "durable store is disabled; configure a store before new submission",
            )),
            Self::Busy => Err(invalid(
                "durable store is temporarily busy; no new dispatch authorized by this descriptor; inspect the original Agent",
            )),
            Self::Unavailable { reason } => Err(invalid(&format!(
                "durable store unavailable ({reason}); preserve original evidence; no new dispatch authorized"
            ))),
            Self::AdmissionBlocked { .. } => Err(invalid(
                "durable store admission is blocked after a write failure or inventory change; stop Agent, inspect original records and restart; no new dispatch authorized",
            )),
        }
    }
}

impl SolverRpcClient {
    /// One descriptor read, without retries. Ready is a snapshot only; concurrent
    /// changes, capacity and final admission checks remain the Agent's authority.
    pub fn describe_durable_agent_store(&self) -> SdkResult<DurableAgentStoreStatus> {
        DurableAgentStoreStatus::from_descriptor(&self.describe_agent()?.result)
    }
}

fn boolean(policy: &Value, field: &str) -> SdkResult<bool> {
    policy[field].as_bool().ok_or_else(|| {
        invalid(&format!(
            "durable store descriptor requires boolean {field}"
        ))
    })
}
