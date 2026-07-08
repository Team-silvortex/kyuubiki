use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessEngine {
    Browser,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessRisk {
    Normal,
    Sensitive,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessRuntimeStyle {
    ServiceOnly,
    BrowserOnly,
    Hybrid,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessActionContract {
    pub id: &'static str,
    pub engine: HeadlessEngine,
    pub category: &'static str,
    pub risk: HeadlessRisk,
    pub required_payload_keys: &'static [&'static str],
    pub output_keys: &'static [&'static str],
}
