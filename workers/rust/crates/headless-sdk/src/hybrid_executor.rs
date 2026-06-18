use crate::{
    HeadlessEngine, HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome,
    MockHeadlessExecutor, ServiceHeadlessExecutor, find_action_contract,
};
use serde_json::Value;

#[derive(Debug)]
pub struct HybridHeadlessExecutor {
    service: ServiceHeadlessExecutor,
    browser: MockHeadlessExecutor,
}

impl HybridHeadlessExecutor {
    pub fn new(service_base_url: &str) -> Self {
        Self::with_token(service_base_url, None)
    }

    pub fn with_token(service_base_url: &str, api_token: Option<&str>) -> Self {
        Self {
            service: ServiceHeadlessExecutor::with_token(service_base_url, api_token),
            browser: MockHeadlessExecutor,
        }
    }
}

impl HeadlessExecutor for HybridHeadlessExecutor {
    fn name(&self) -> &'static str {
        "hybrid"
    }

    fn execute_step(
        &mut self,
        action: &str,
        step_index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        match find_action_contract(action).map(|contract| contract.engine) {
            Some(HeadlessEngine::Service) => self.service.execute_step(action, step_index, payload),
            Some(HeadlessEngine::Browser) => {
                let mut outcome = self.browser.execute_step(action, step_index, payload)?;
                outcome.status = "executed_mock_browser".to_string();
                Ok(outcome)
            }
            None => Err(HeadlessExecutorError {
                message: format!("unsupported hybrid action: {action}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn routes_browser_actions_to_mock_branch() {
        let mut executor = HybridHeadlessExecutor::new("http://127.0.0.1:3000");
        let outcome = executor
            .execute_step("open_page", 1, &json!({ "url": "https://example.com" }))
            .expect("browser action should route");
        assert_eq!(outcome.status, "executed_mock_browser");
        assert_eq!(outcome.result["url"].as_str(), Some("https://example.com"));
    }
}
