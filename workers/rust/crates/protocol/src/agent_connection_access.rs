use serde::{Deserialize, Serialize};

pub const AGENT_CONNECTION_ACCESS_SCHEMA: &str = "kyuubiki.agent-connection-access/v1";

/// Connection-scoped readiness, never an Orchestra identity, user identity or lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConnectionAccess {
    pub schema_version: String,
    pub scope: String,
    pub peer_scope: String,
    pub control_mode: String,
    pub control_allowed: bool,
    pub lifecycle_mutations_allowed: bool,
    pub authenticated: bool,
    pub encrypted: bool,
}

impl AgentConnectionAccess {
    pub fn with_tls_token(mut self) -> Self {
        self.authenticated = true;
        self.encrypted = true;
        self.control_mode = if self.control_allowed {
            "tls_api_token"
        } else {
            "tls_discovery_only"
        }
        .into();
        self
    }

    pub(crate) fn new(loopback: bool, trusted_lan: bool) -> Self {
        Self {
            schema_version: AGENT_CONNECTION_ACCESS_SCHEMA.into(),
            scope: "native_socket_connection".into(),
            peer_scope: if loopback {
                "host_loopback"
            } else {
                "non_loopback"
            }
            .into(),
            control_mode: if loopback {
                "host_loopback"
            } else if trusted_lan {
                "trusted_lan_control"
            } else {
                "discovery_only"
            }
            .into(),
            control_allowed: loopback || trusted_lan,
            lifecycle_mutations_allowed: loopback,
            authenticated: false,
            encrypted: false,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let expected = match (self.peer_scope.as_str(), self.control_mode.as_str()) {
            ("host_loopback", "host_loopback") => Self::new(true, false),
            ("non_loopback", "trusted_lan_control") => Self::new(false, true),
            ("non_loopback", "discovery_only") => Self::new(false, false),
            ("host_loopback", "tls_api_token") => Self::new(true, false).with_tls_token(),
            ("non_loopback", "tls_api_token") => Self::new(false, true).with_tls_token(),
            ("non_loopback", "tls_discovery_only") => Self::new(false, false).with_tls_token(),
            _ => return Err("invalid_connection_access"),
        };
        if *self == expected {
            Ok(())
        } else {
            Err("invalid_connection_access")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn tls_token_authentication_preserves_source_policy_and_local_lifecycle_boundaries() {
        for (local, trusted) in [(true, false), (false, true), (false, false)] {
            let access = AgentConnectionAccess::new(local, trusted).with_tls_token();
            assert_eq!(access.validate(), Ok(()));
            assert!(access.authenticated && access.encrypted);
            assert_eq!(access.control_allowed, local || trusted);
            assert_eq!(access.lifecycle_mutations_allowed, local);
            let mut unencrypted = access.clone();
            unencrypted.encrypted = false;
            assert!(unencrypted.validate().is_err());
        }
    }

    #[test]
    fn access_observations_match_the_three_existing_native_policy_cases() {
        for (local, trusted, allowed, mutations) in [
            (true, false, true, true),
            (true, true, true, true),
            (false, false, false, false),
            (false, true, true, false),
        ] {
            let access = AgentConnectionAccess::new(local, trusted);
            assert_eq!(access.validate(), Ok(()));
            assert_eq!(access.control_allowed, allowed);
            assert_eq!(access.lifecycle_mutations_allowed, mutations);
            assert!(!access.authenticated && !access.encrypted);
        }
    }

    #[test]
    fn access_observations_reject_missing_fields_versions_and_authority_escalation() {
        for base in [
            AgentConnectionAccess::new(true, false),
            AgentConnectionAccess::new(false, false),
            AgentConnectionAccess::new(false, true),
        ] {
            let value = serde_json::to_value(&base).unwrap();
            for field in value.as_object().unwrap().keys() {
                let mut absent = value.clone();
                absent.as_object_mut().unwrap().remove(field);
                assert!(
                    serde_json::from_value::<AgentConnectionAccess>(absent).is_err(),
                    "{field}"
                );
            }
            for (field, wrong) in [
                ("schema_version", json!("future")),
                ("scope", json!("global")),
                ("peer_scope", json!("authenticated_peer")),
                ("control_mode", json!("authenticated")),
                ("control_allowed", json!(!base.control_allowed)),
                (
                    "lifecycle_mutations_allowed",
                    json!(!base.lifecycle_mutations_allowed),
                ),
                ("authenticated", json!(true)),
                ("encrypted", json!(true)),
            ] {
                let mut wrong_value = value.clone();
                wrong_value[field] = wrong;
                let parsed = serde_json::from_value::<AgentConnectionAccess>(wrong_value).unwrap();
                assert_eq!(
                    parsed.validate(),
                    Err("invalid_connection_access"),
                    "{field}"
                );
            }
            for wrong in [Value::Null, json!({}), json!(true), json!("allowed")] {
                assert!(serde_json::from_value::<AgentConnectionAccess>(wrong).is_err());
            }
            let mut extended = value;
            extended["credential"] = json!("not-authority");
            assert!(serde_json::from_value::<AgentConnectionAccess>(extended).is_err());
        }
    }
}
