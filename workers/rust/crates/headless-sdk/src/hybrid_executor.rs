use crate::{
    HeadlessEngine, HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome,
    MockHeadlessExecutor, ServiceHeadlessExecutor, find_action_contract,
    service_executor_supports_action,
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

    pub fn try_new(service_base_url: &str) -> Result<Self, HeadlessExecutorError> {
        Self::try_with_token(service_base_url, None)
    }

    pub fn with_token(service_base_url: &str, api_token: Option<&str>) -> Self {
        Self {
            service: ServiceHeadlessExecutor::with_token(service_base_url, api_token),
            browser: MockHeadlessExecutor,
        }
    }

    pub fn try_with_token(
        service_base_url: &str,
        api_token: Option<&str>,
    ) -> Result<Self, HeadlessExecutorError> {
        Ok(Self {
            service: ServiceHeadlessExecutor::try_with_token(service_base_url, api_token)?,
            browser: MockHeadlessExecutor,
        })
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
            Some(HeadlessEngine::Service) if service_executor_supports_action(action) => {
                self.service.execute_step(action, step_index, payload)
            }
            Some(HeadlessEngine::Service) => Err(HeadlessExecutorError {
                message: format!("unsupported native service action: {action}"),
            }),
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

    #[test]
    fn strict_constructor_rejects_service_base_url_paths() {
        let error = HybridHeadlessExecutor::try_new("http://127.0.0.1:3000/not-api")
            .expect_err("base URL path should fail");

        assert!(error.message.contains("paths, queries, and fragments"));
    }
}
